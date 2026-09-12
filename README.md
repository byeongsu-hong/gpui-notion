# gpui-notion

A Notion-like block editor written in Rust on [GPUI](https://www.gpui.rs) through
[`gpui-kit`](https://github.com/longbridge/gpui-kit), with the feature surface and command
names of Tiptap's [Notion-like editor template](https://tiptap.dev/docs/ui-components/templates/notion-like-editor).

```bash
cargo run            # the demo document
cargo test           # unit tests + UI integration tests
```

## What it does

- **Blocks**: paragraph, heading 1–3, bullet / numbered / to-do lists (nested), blockquote,
  code block with syntax highlighting, separator, image, callout, toggle.
- **Inline marks**: bold, italic, underline, strike, code, link, highlight (10 colors),
  text color (10 colors), mention.
- **Markdown input rules**: `# `, `## `, `### `, `- `, `* `, `+ `, `1. `, `[] `, `[x] `,
  `> `, ` ``` `, `---`, plus `**bold**`, `_italic_`, `~~strike~~`, `` `code` ``, `==mark==`
  and smart typography (`--`, `...`, `(c)`).
- **Menus**: `/` block commands, `:` emoji, `@` mentions — one keyboard contract, filtered
  live, Enter commits, Escape closes and leaves the typed text alone.
- **Selection toolbar**: turn-into, B/I/U/S/code, link editor, color palettes, more menu.
- **Gutter**: hover `+` to insert a block, grip to select, drag to reorder, click for the
  block menu (turn into, reset formatting, duplicate, copy, move, delete).
- **Document undo/redo** with typing coalescing, block-level selection, clipboard as
  markdown, paste that becomes blocks.

Parity is graded item by item against the template in [`docs/PARITY.md`](docs/PARITY.md).
The spec it is graded against is [`docs/research/tiptap-notion-spec.md`](docs/research/tiptap-notion-spec.md),
extracted from Tiptap's docs and shipped bundle.

## Architecture

One entity owns the document; each block owns the input that renders it.

```
NotionEditor (Entity)            src/editor/view.rs
├── Vec<Block>                   src/editor/block.rs
│   ├── ty: BlockType + attrs    node type, heading level, checked, language…
│   ├── text: String             mirror of the input, used to diff edits
│   ├── marks: MarkList          inline formatting as byte ranges
│   └── state: Entity<EditorState>  one gpui-kit code-editor input per block
├── suggestion / link_editor / selection / history / drop_target
└── BlockRegistry (Global)       node types, their rendering and their rules
```

Inline formatting is painted by handing the input's decoration layer one
`HighlightStyle` run per mark run (`view.rs::apply_decorations`); the marks themselves live
in the editor and are remapped across every edit (`mark.rs`), so the text and its formatting
never drift apart.

The command surface mirrors Tiptap's: `toggle_bold`, `toggle_heading(2)`,
`toggle_bullet_list`, `split_block`, `join_backward`, `sink_list_item`, `lift_list_item`,
`set_link`, `unset_all_marks`, `set_horizontal_rule`, `insert_content` — see
`src/editor/commands.rs`.

## Adding a block type

Node types are data. Implement [`BlockSpec`](src/editor/block.rs) and register it; the slash
menu, the markdown rules, the turn-into menu and the layout all pick it up.

```rust
use gpui_notion::editor::block::*;

struct Warning;

impl BlockSpec for Warning {
    fn type_name(&self) -> &'static str { "warning" }
    fn label(&self, _: &BlockAttrs) -> SharedString { "Warning".into() }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout { inner_padding: px(16.), ..Default::default() }
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^!\s$",
            build: |_| Some(("warning".into(), BlockAttrs::default())),
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Warning",
            subtext: "Something to watch out for",
            keywords: &["warning", "caution"],
            group: "Insert",
            icon: "triangle-alert",
            run: |editor, window, cx| editor.toggle_node("warning", BlockAttrs::default(), window, cx),
        }]
    }

    fn wrap(&self, _: &BlockContext, content: AnyElement, _: &mut Window, cx: &mut App) -> AnyElement {
        div().bg(cx.theme().warning).child(content).into_any_element()
    }
}

// in your app's init, after `gpui_notion::editor::init(cx)`
BlockRegistry::register(cx, Warning);
```

A spec can also supply `render_leading` (the bullet, number or checkbox column),
`render_body` (for nodes that are not text, like the separator), `caps` (whether marks,
input rules, list nesting or multi-line Enter apply) and `split_into` (what Enter creates).

## Keyboard

| Keys | Command |
| --- | --- |
| `Mod+B` / `I` / `U` / `Shift+S` / `E` | bold, italic, underline, strike, code |
| `Mod+.` / `Mod+,` | superscript / subscript |
| `Mod+Shift+K` | link editor · `Mod+R` reset formatting |
| `Mod+Alt+0…3` | paragraph, heading 1–3 |
| `Mod+Shift+7` / `8` / `9` | numbered, bullet, to-do list |
| `Mod+Shift+B` / `Mod+Alt+C` | blockquote / code block |
| `Enter` / `Shift+Enter` | split block / line break inside the block |
| `Backspace` at block start | join with the block above, or lift it to a paragraph |
| `Tab` / `Shift+Tab` | nest / lift a list item |
| `↑` `↓` `←` `→` at an edge | move to the neighbouring block |
| `Shift+↑` `Shift+↓` at an edge | select whole blocks · `Mod+A` twice selects the document |
| `Mod+D` / `Mod+Shift+Backspace` | duplicate / delete block |
| `Mod+Shift+↑` `Mod+Shift+↓` | move the block |
| `Mod+Z` / `Mod+Shift+Z` / `Mod+Y` | undo / redo |
| `Mod+/` | open the block menu · `Escape` closes menus, then selects the block |

## Testing

`tests/editing.rs` drives the real UI headlessly with `#[gpui_kit::test]`: it clicks,
types, presses keys and drags, then asserts the document. Unit tests for the mark algebra
live beside it in `src/editor/mark.rs`.

```bash
cargo test --test editing        # UI integration tests
NOTION_DEMO=toolbar cargo run    # open the window with the selection toolbar shown
```

## Notes and limits

Some template behaviour cannot be expressed on this stack today, and is recorded as such in
`docs/PARITY.md`: GPUI's `HighlightStyle` carries no font size or family, so per-range font
switching (real superscript baselines, monospace inline code) is unavailable, and
multi-line inputs do not support per-block text alignment. Tables and image uploading are
not implemented.
