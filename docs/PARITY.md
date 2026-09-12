# Parity grading against the Tiptap Notion-like editor template

Graded against `docs/research/tiptap-notion-spec.md` §7 (42 items), §1–§6 for detail.

**Basis.** Working tree at `git` HEAD `3e0ba35` plus uncommitted changes, snapshotted
2026-09-12 10:10 local; `src/editor/view.rs` sha256 `c5a66278…`, `commands.rs`
`9e910e10…`, `keymap.rs` `8805ca34…`, `block.rs` `0d85b89c…`, `tests/editing.rs`
`5be61204…`. The tree was being edited while this was written, so line numbers are
valid for those hashes. `cargo test` on that snapshot: **36/36** UI tests in
`tests/editing.rs` and **15/15** unit tests in `src/editor/mark.rs` pass.

Statuses are strict: `Done` means the code does what §7 asks, not that something
adjacent exists.

---

## 1. What is actually implemented

The document is a **flat `Vec<Block>` owned by one `NotionEditor` entity**
(`view.rs:33-53`); each `Block` (`block.rs:377-394`) is a node-type name +
`BlockAttrs` + a `String` mirror of its text + a `MarkList`, and owns its own
`Entity<EditorState>` — i.e. every block is a separate gpui-kit multi-line input,
not one document buffer. Inline formatting is **not** in the text: marks are byte
ranges in `MarkList` (`mark.rs:140-193`) that are flattened into non-overlapping
runs and pushed into the input's Monaco-style decoration layer as
`TextDecoration`/`HighlightStyle` (`view.rs:486-512`, `view.rs:813-856`), with the
text mirror re-diffed after every `InputEvent::Change` (`view.rs:388-441`,
`mark.rs:437-467`) so marks travel with edits. Node types are **data**: a
`BlockSpec` trait (`block.rs:217-297`) supplies label, caps, layout, leading
marker, wrapper chrome, split-into type, markdown input rules and slash-menu
entries, and a `BlockRegistry` global (`block.rs:300-373`, registrations in
`blocks.rs:24-36`) is the single source for the slash menu, the input-rule engine
and "turn into" — adding a node type is one `impl` plus one `register` call.
Document-level undo is a snapshot stack over `Vec<BlockContent>` with 500 ms typing
coalescing (`history.rs:17-134`), because the per-input undo stacks cannot see
structural edits.

---

## 2. The 42 checklist items

| # | Item | Status | Evidence | Gap |
| --- | --- | --- | --- | --- |
| 1 | Node set (para, h1–6, lists, task, quote, code, hr, image(+caption), imageUpload, table, hardBreak, mention/emoji atoms) | Partial | `blocks.rs:24-36` registers paragraph/heading/bullet/ordered/task/quote/code/hr/image/callout/toggle; `block.rs:29-41`; hard break = `shift-enter` newline inside the block, test `shift_enter_breaks_the_line_inside_a_block` | No `table`, no `imageUpload` node, no image caption, no `tocNode`. Mention/emoji are not inline atoms (mention = a mark over plain text `slash.rs:259-269`; emoji = a literal character). Headings 4–6 reachable only via the `####` input rule (`blocks.rs:107-115`), no menu/shortcut. Extra non-spec nodes: `callout`, `details` toggle. |
| 2 | Mark set incl. exclusive `code`, multicolor highlight, textStyle color, super/subscript | Partial | `mark.rs:15-29` (all 10 kinds + `Mention`); exclusivity of super/sub in `commands.rs:136-144`; multicolor `mark.rs:62-94`; color `mark.rs:96-138` | `code` is not exclusive: toggling it neither strips nor blocks other marks (`commands.rs:132-134` is a plain `toggle_mark`), it is only non-inclusive for continued typing (`mark.rs:56-58`). Superscript/subscript apply **no rendering at all** — `view.rs:852` is an empty match arm (see §4: impossible on this stack). |
| 3 | Stable node ids on every `UniqueID` type | Partial | `block.rs:24` `BlockId(u64)`, minted in `view.rs:228-232`, used by focus/drag/selection | Ids are runtime-only: `BlockContent` (`block.rs:419-425`) has no id, so ids are not serialized and undo/redo mints new ones (`history.rs:107-134`). No anchor-link feature depends on them. |
| 4 | Node input rules `#`, `-/+/*`, `1.`, `[] / [x] `, `> `, ```` ```lang ````/`~~~`, `---` | Done | `blocks.rs:107-115` (h1–6), `:204-209`, `:268-282` (start attr), `:404-421`, `:485-490`, `:583-612`, `:661-666`; engine `input_rules.rs:154-197`; test `markdown_rules_create_nodes` | (Tests cover `#`, `-`, `1.`, `>` only; task/code/hr rules are implemented but untested.) |
| 5 | Mark input rules `**`,`__`,`*`,`_`,`~~`,`` ` ``,`==` on typing **and** on paste | Partial | `input_rules.rs:38-67`, applied at `:199-238`; tests `inline_rules_apply_marks`, `typing_after_an_inline_rule_is_plain` | Paste runs **node** rules only: `split_pasted_lines` (`view.rs:443-484`) calls `apply_markdown_prefix` (`input_rules.rs:91-127`), which never touches `INLINE_RULES`. Pasting `**x**` leaves the asterisks. |
| 6 | Smart typography `--`, `...`, quotes, `(c)` | Partial | `input_rules.rs:70-85` (`(c) (r) (tm) ... <- -> -- != <= >= +/-`), applied `:240-259` | No curly-quote substitution — the one rule of the four named in §7 that is missing. |
| 7 | All 30+ §4 bindings, `Mod` = platform modifier, badges in tooltips/menus | Partial | `actions.rs:58-98` (27 bindings, `secondary-` = platform Mod); handlers `keymap.rs:56-194` | Missing: `Mod-Alt-4/5/6`, `Mod-Shift-L/E/R/J` align, `Mod-Shift-2` mention, `Mod-Shift-E` emoji, `Mod-Shift-I` image upload, `Mod-Shift-D` download, `Mod-Ctrl-L` copy anchor, `Mod-Enter`. **No shortcut badges anywhere** — menus are built with plain `.menu(label, action)` (`gutter.rs:225-244`, `toolbar.rs:132-140`) and tooltips are bare strings. |
| 8 | Enter: split; heading→paragraph at end; empty list item lifts; triple-Enter exits code; Shift/Mod-Enter hard break | Partial | `keymap.rs:198-233`; split `commands.rs:387-431`; lift `commands.rs:398-407`; triple-Enter `keymap.rs:216-229`; tests `enter_splits_a_block_at_the_caret`, `enter_continues_a_list_and_an_empty_item_leaves_it`, `enter_inside_a_code_block_adds_a_line`, `shift_enter_breaks_the_line_inside_a_block` | `Heading` has no `split_into` override (`blocks.rs:71-145`), so Enter **mid-heading** yields heading + paragraph instead of two headings. `Mod-Enter` is not bound. |
| 9 | Backspace at block start: join/lift/unwrap/heading-downgrade; whole-atom delete; node-selected delete as its own undo step | Partial | `keymap.rs:235-252`; `commands.rs:434-472` (lift → downgrade → join, deletes a preceding atom node); block-selection delete `selection.rs:99-123` with `Step::Structural` (`history.rs:66-85`); tests `backspace_at_the_start_joins_with_the_previous_block`, `shift_down_selects_whole_blocks_and_backspace_removes_them` | No whole-atom deletion: a mention is ordinary text under a mark, so Backspace removes one character of the name. |
| 10 | Tab/Shift-Tab: list sink/lift, table cells, menu nav with wrap | Partial | `keymap.rs:349-386`; `commands.rs:502-553`; menu wrap `slash.rs:216-226` (`rem_euclid`); test `tab_nests_a_list_item_and_shift_tab_lifts_it` | No table cell traversal (no table node). Sink is capped at "one deeper than the item above" (`commands.rs:510-524`), which matches ProseMirror. |
| 11 | Undo/redo with typing coalescing; `Mod-Y` also redoes | Done | `history.rs:17-134` (500 ms `COALESCE`, `Step::Typing` vs `Structural`); `actions.rs:96-97` binds `secondary-shift-z` and `secondary-y`; `keymap.rs:78-85` captures `Undo`/`Redo` from the inputs; tests `undo_reverts_typing_and_redo_restores_it`, `undo_reverts_a_split`, `undo_reverts_a_node_change`, `redo_through_the_api` | — |
| 12 | Slash opens on `/` at start or after whitespace; also `Mod-/` and gutter `+` | Done | `slash.rs:136-164` (word-boundary check at `:146-149`); `Mod-/` → `actions.rs:69` → `slash.rs:74-93`; `+` button `gutter.rs:88-93` → `gutter.rs:119-124`; tests `the_slash_menu_filters_and_runs_an_item`, `the_gutter_plus_adds_a_block_below`, `a_colon_in_prose_does_not_hold_a_menu_open` | — |
| 13 | Ghost `Filter...` decoration; filter on title + keywords; hide unavailable items | Partial | filter `slash.rs:167-203` + `suggestion.rs:118-126` (case-insensitive title **and** keywords) | The `Filter...` string exists (`suggestion.rs:41-46`) but `Trigger::hint()` **has no caller** — no ghost decoration is ever rendered. There is no per-item `check(editor)`, so nothing can be conditionally hidden. |
| 14 | Group order AI → Style → Insert → Upload, labels, separators, exact §3.1 titles | Partial | group headers `slash.rs:305-318`; titles/keywords/groups per spec in `blocks.rs:55-64, 117-144, 211-220, 284-293, 423-432, 492-501, 614-623, 668-677, 731-740` | Rows are emitted in **registry order** (`block.rs:367-372`), and a header is printed whenever the group name changes, so the real order is Style… → Insert(Separator) → Upload(Image) → Insert(Callout) → Style(Toggle list): groups interleave and repeat. No AI group. Missing items: Continue Writing, Ask AI, Mention, Emoji, Table, Table of contents. Extra items: Callout, Toggle list. No separators between groups. |
| 15 | ↑/↓/Tab/Shift-Tab/Home/End/Enter/Escape; first item preselected; reset on query change; scroll into view | Partial | ↑/↓ `keymap.rs:273-317`, Tab/Shift-Tab `:349-386`, Enter `:204-208`, Escape `:387-400`; preselect `slash.rs:88-91`; reset on query change `slash.rs:122-126`; test `escape_closes_the_slash_menu_and_keeps_the_text` | `Home`/`End` are not intercepted — they move the caret and re-scope the query instead of jumping to first/last. No scroll-into-view for the selected row (the list is `overflow_y_scroll` with `max_h(360px)`, `slash.rs:368-373`, but nothing scrolls it). |
| 16 | Committing deletes the `/query` text before running the command | Done | `slash.rs:228-272` — `edit_block_text(menu.start..end, "")` then `run(self, …)`; test `the_slash_menu_filters_and_runs_an_item` asserts the block text is `""` after commit | — |
| 17 | Toolbar only for non-empty valid selections; hidden while dragging / during AI / on small viewports | Partial | `toolbar.rs:32-45` (non-empty range **and** the block's spec allows marks); hidden while a drag is tracked (`drop_target.is_some()`) and while a suggestion menu is open; test `the_toolbar_appears_over_a_selection` | "Dragging" is approximated by `drop_target`, which is only set once the pointer has moved over a block (`gutter.rs:139-159`); there is no `isDragging`/`UiState`. No viewport rule (no mobile layout at all), no AI/comment states. |
| 18 | Order: AI improve \| Turn into \| B I U S Code \| image actions \| link, color \| more(super/sub, 4 aligns) | Partial | `toolbar.rs:64-88` — Turn into ¦ B I U S Code ¦ link, color ¦ more; `more` = super/sub + Reset formatting (`toolbar.rs:155-169`); the `more` group is suppressed while `code` is active, as in the template | No AI improve, no image-specific group. The four alignment buttons are absent — see §4, alignment is not available on this stack. `Reset formatting` sits in `more` (the template puts it in the drag menu). |
| 19 | Turn-into shows exactly the 9 §3.2 labels and reflects the active type | Done | `toolbar.rs:132-140` — Text, Heading 1/2/3, Bulleted list, Numbered list, To-do list, Blockquote, Code block, verbatim and in order; trigger label = active node's display name (`toolbar.rs:123`, `:145-153`) | (The active type is shown on the trigger, not as a checked row.) |
| 20 | Buttons show active state with brand-colored icons | Partial | `toolbar.rs:109-118` `.selected(is_mark_active)`, link `:171-180` | The active look is gpui-kit's `Button` selected styling; no brand token is used anywhere in the editor (`rg 6229ff|7a52ff` over `src/` returns nothing). |
| 21 | `+` and grip on hover in the left margin, 16 px off the block, top-aligned >40 px else centred | Partial | `gutter.rs:60-107`: `.invisible().group_hover(group_name(id), …)`; the block row reaches into the margin (`view.rs:759-760`, `ml(-52px)/pl(52px)`) so the hover area bridges handle→text | Placed at a fixed `left(4px)` inside that 52 px reach, not a 16 px main-axis offset. Vertical position is always "centred on the first line" (`gutter.rs:68`), which approximates but does not implement the >40 px rule. No `transition: top .2s`. |
| 22 | Grip tooltip is two lines "Click for options" / "Hold for drag" | Partial | `gutter.rs:103` `.tooltip("Click for options, hold for drag")` | One line, comma-joined; gpui-kit's tooltip takes a string. |
| 23 | Drag reorders; editor cursor `grabbing`; handle hides during text selection | Partial | drag payload + preview `gutter.rs:250-256`, `:37-65`; tracking `:139-159`; drop `:161-175`; indicator `:177-189`; reorder `commands.rs:595-604`; tests `dragging_the_handle_reorders_blocks`, `a_block_can_be_dragged_below_another` | No `grabbing` cursor on the editor, no `grab` cursor on the handle. The gutter does not hide while a text selection is active. |
| 24 | Drag menu: opens left, ≥15 rem, node name label, §3.3 groups incl. Color submenu, shortcut badges | Partial | `gutter.rs:209-245`: label = node display name, Turn into submenu (9 items), Reset formatting, ¦ Duplicate, Copy to clipboard, ¦ Move up/down, ¦ Delete | Default popover placement, no `min-width: 15rem`. Missing: **Color submenu** (Recent/Text/Highlight), Copy anchor link, Ask AI, Download image, and every shortcut badge. "Duplicate" not "Duplicate node". |
| 25 | Link popover: `Paste a link...`, Enter applies, apply/open/remove, apply disabled when empty and no active link | Partial | `toolbar.rs:214-254` (seeded from the existing href, autofocus + select-all), Enter via `InputEvent::PressEnter` `:233-241`, render `:296-357`, apply `:318-325`, remove `:327-345`, `normalize_href` `:377-383`; test `the_link_editor_sets_a_link_on_the_selection` | No open-in-new-tab button. Apply is disabled whenever the field is empty, without the "…and no link is active" exception. |
| 26 | Image upload node: dashed drop zone, exact strings, per-file progress, drag states, accent border when selected | Missing | `blocks.rs:705-729` renders a dashed 120 px box with the literal `Click to upload or drag and drop` and nothing else; `commands.rs:335-362` inserts an `image` block | No file picking, no drop handling, no `imageUpload` node, no `limit`/`maxSize` subtext, no progress rows, no drag-active/drag-over states, no selected-state border. The "Click to upload" half is not italicised. |
| 27 | Table: resize handles, extend buttons, insert labels, cell overlay, clear contents | Missing | no `table` in `block.rs:29-41` or `blocks.rs:24-36` | The node type does not exist. |
| 28 | Emoji (`:`) and mention (`@`) share the slash-menu keyboard contract | Done | one `SuggestionMenu` + one `Trigger` for all three (`slash.rs:21-29`, `suggestion.rs:13-62`); the emoji menu waits for a shortcode (`suggestion.rs:50-52`, `slash.rs:185-194`); tests `the_emoji_menu_replaces_the_shortcode`, `the_mention_menu_inserts_a_marked_name`, `a_colon_in_prose_does_not_hold_a_menu_open` | (They share it exactly — including the gaps listed under #15.) |
| 29 | Placeholder only in the focused empty block, italic `Write, type '/' for commands…` | Partial | exact string `blocks.rs:51-53`; focus rule `view.rs:143-165` + `view.rs:124-137`; the input shows it only when empty | Not italic (`view.rs:674-703` sets no per-placeholder font style). `Heading` sets `placeholder_always` (`blocks.rs:103-105`), so every heading shows `Heading N` whether focused or not — a deliberate Notion-ism, not the template's rule. Other specs define their own strings (`blocks.rs:179, 242, 358, 465, 764, 834`), shown under the focus rule. |
| 30 | A trailing paragraph always exists after a non-paragraph last node; clicking below focuses it | Partial | `commands.rs:365-381` `ensure_paragraph_after`, called after HR (`:323`), image (`:360`), code-block exit (`keymap.rs:224`), ArrowDown off the last block (`keymap.rs:310-316`) and non-textual input rules (`input_rules.rs:189`); click-below `view.rs:905-919` + `:930-948`; tests `clicking_below_the_document_appends_a_paragraph`, `arrow_down_leaves_a_code_block_at_the_end_of_the_document` | It is a set of call sites, not an invariant: deleting the paragraph after a rule/image leaves a non-textual node last until you arrow down or click below. Bottom space is a fixed `280 px` (`style.rs:18`), not `30vh`. |
| 31 | Node selection highlight = translucent brand fill, 8 px radius; images get a 2 px outline | Partial | `view.rs:761-763` `.rounded(px(4.)).bg(theme.selection.opacity(0.4))` | 4 px radius, not 8; the fill is the theme's selection color, not brand-500 at 20 %; images get the same fill rather than a 2 px brand outline. |
| 32 | Selection stays visible when focus moves to a toolbar/popover | Missing | — | Nothing implements it. The *range* survives because it lives in the block's `EditorState` (`toolbar.rs:32-45` and `view.rs:216-220` read it after focus moves), so toolbar and menu commands still act on the right text; but no `selection`-class decoration is painted while the input is unfocused. Opening the link editor explicitly moves focus to its input (`toolbar.rs:242-245`). |
| 33 | Content column 708 px, side gutters ≥96 px, padding 48/48/30vh | Partial | `style.rs:10` `PAGE_WIDTH = 708`, `:12` `PAGE_PADDING = 48`; applied `view.rs:900-903`; wrap width 612 px (`view.rs:63`) | `GUTTER_WIDTH = 96` (`style.rs:14`) is declared and **never used** — the column is simply centred, so side margins are whatever the window leaves and can fall below 96 px. Bottom space is a fixed 280 px, not 30vh. No ≤768 px/≤480 px behaviour. |
| 34 | Paragraph 16/1.6/400 + 20 px top; H1 1.5em/700/3em-top, H2 1.25em/700/2.5em, H3 1.125em/600/2em; first heading no top margin | Partial | `style.rs:21-25` (16 px, 1.6, 20 px); `blocks.rs:80-97` (24 px/700, 20 px/700, 18 px/600); first block margin 0 at `view.rs:737-743` | Heading top margins are wrong: the spec's `3em/2.5em/2em` of the heading's own size = 72/50/36 px, the code uses factors `1.35/1.5/1.6` = 32/30/29 px. H4–H6 collapse to 16 px/600. |
| 35 | Lists: 1.5em vertical margin, 1.5em left padding, marker cycles, 24 px indent | Partial | marker cycles `blocks.rs:187-202` (disc/circle/square) and `:250-266` (decimal/alpha/roman, `alpha_ordinal`/`roman_ordinal` `:296-332`); `style.rs:28` `INDENT_WIDTH = 24`; `leading_width = 24` (`blocks.rs:149-158`) | List top margin is 20 px, not 1.5em (24 px); there is no bottom margin; consecutive items get a 2 px gap (`view.rs:737-743`) rather than CSS-collapsed zero. |
| 36 | Blockquote: 0.25em bar in gray-900, 1em left padding, 1.5rem vertical margin | Partial | `blocks.rs:469-483` — 4 px bar in `theme.foreground`, `pl(16px)`, `py(6px)`; `margin_top: 24px` (`:455-463`) | `margin_bottom: 0` instead of 1.5rem, and the bar is rounded (`rounded(px(2.))`) where the template's is square. |
| 37 | Code block 1em padding, 1 px border, 6 px radius, mono 16 px; inline code 0.875em, tinted bg + border | Partial | block: `blocks.rs:538-581` (`p(16)`, `border_1`, `rounded(6)`, `mono: true`); inline: `view.rs:832-835` (background + red foreground, `style.rs:55-69`) | Code-block text is 14 px, not 16 px. Inline code has no border and cannot be 0.875em — see §4. |
| 38 | Task checkbox 1em square, 0.25rem radius, checked = gray-a-900 fill + white check, checked text 50 % + strike | Partial | `blocks.rs:366-402` (16 px box, `rounded(4)`, click toggles) and `:349-356` (`text_opacity 0.5` + `strikethrough`), painted through the decoration layer (`view.rs:490-492`) | Fill/border use `theme.primary`, not gray-a-900; the check is the text glyph `✓` at 11 px rather than a masked SVG; no 80 ms transition. |
| 39 | Light/dark palettes match the §5.4 token table incl. selection and brand | Partial | mark palettes `style.rs:76-97` (highlight) and `:99-123` (text color); chrome colors come from the gpui-kit theme (`ui.rs:35-79`) | Values do not match §5.4: e.g. yellow highlight light `#fef3c0` vs spec `#fef9c3`; orange text is **swapped** (code: light `#d9730d`, dark `#c77d48`; spec: light `#c77d48`, dark `#d9730d`). No brand ramp, no `#7a52ff33` selection, no Tiptap gray/alpha ramps — page, border, popover, caret and link colors are whatever the active kit theme says. |
| 40 | Popovers: 1.125 rem radius, 0.375 rem padding, elevated-md 4-layer shadow, 2 rem rows, ghost hover, 0.75 rem/600 labels, 1 px separators | Partial | `ui.rs:35-44` (radius 12 px, padding 4 px, `shadow_lg`), `ui.rs:47-62` (32 px rows, hover fill, 6 px radius) | Radius 12 px vs 18 px, padding 4 px vs 6 px, gpui-kit's single-layer `shadow_lg` vs the 4-layer elevated-md, group labels 11 px at default weight (`slash.rs:308-317`) vs 12 px/600, and the slash menu draws **no** separators between groups. |
| 41 | Floating toolbar: 0.188 rem padding, 0.875 rem radius, 2 rem buttons, 0.125 rem gap, 1 px × 1.5 rem separators | Partial | `toolbar.rs:64-69` (`p(4px)`, `gap(2px)`, surface from `ui.rs:35`), separators `toolbar.rs:360-366` (1 px × 20 px) | Padding 4 px vs 3 px, radius 12 px vs 14 px, separators 20 px vs 24 px tall; buttons are gpui-kit `Button::small()`, not a 2 rem square (the local 32 px `toolbar_button` helper, `ui.rs:65-79`, is unused). |
| 42 | Tooltips: dark pill, 0.375 rem radius, 12 px/500, shortcut as a `kbd` badge | Partial | gpui-kit tooltips used throughout (`toolbar.rs:114, 161, 177, 189, 322, 332`; `gutter.rs:92, 103`) | Appearance is entirely the kit's default; no geometry is set and **no shortcut badge is rendered anywhere**, in tooltips or menus. |

**Counts — Done 6 · Partial 33 · Missing 3 · Not possible on this stack 0.**

No whole checklist item is blocked by the platform; three *sub*-requirements are
(superscript/subscript rendering in #2, the four alignment buttons in #18, inline
code's 0.875em in #37). They are itemised in §4.

---

## 3. Known gaps, ranked by value

1. **Slash menu order, groups and missing items (#14).** Sort
   `BlockRegistry::slash_items()` (`block.rs:367-372`) by a declared group order
   (AI, Style, Insert, Upload) before rendering, emit one header per group and a
   `separator()` row between groups in `slash.rs:305-318`; add Mention/Emoji/Table
   entries that just insert the trigger character.
2. **Shortcut badges everywhere (#7, #24, #42).** Add `shortcut: Option<&str>` to
   `SlashItem` and use gpui-kit's `menu_with_check`/label+badge row for
   `gutter.rs:209-245` and `toolbar.rs:132-140`; render the same string as a `kbd`
   pill in tooltips.
3. **Table node (#27, and the Tab half of #10).** A `Table` `BlockSpec` whose
   `render_body` lays out a grid of child `EditorState`s held in `BlockAttrs.extra`
   or a side table keyed by `BlockId`; Tab/Shift-Tab handled in `keymap.rs:349-386`
   before the list branch.
4. **Image upload node (#26).** A second spec `imageUpload` with `caps::atom()`, a
   drop target using gpui's `on_drop::<ExternalPaths>`, a `Vec<Upload>` progress
   model on the editor, and replacement by the `image` node on completion.
5. **Color submenu in the drag menu (#24).** The palettes and actions already exist
   (`toolbar.rs:183-209`, `actions::ApplyColor`); reuse the same builder as a
   `.submenu("Color", …)` in `gutter.rs:209-245` and keep a `recent: Vec<ApplyColor>`
   on the editor for the "Recent colors" group.
6. **Mark input rules on paste (#5).** In `apply_markdown_prefix`
   (`input_rules.rs:91-127`), after the node rule fires, scan the finished line with
   the `INLINE_RULES` regexes globally (not anchored at `$`) and apply marks.
7. **Heading split semantics (#8).** Give `Heading` a `split_into` that returns
   `(HEADING, attrs)` when the caret is not at the end of the text, paragraph when it
   is — needs the caret passed into `split_into`, or the decision made in
   `commands.rs:418`.
8. **Ghost `Filter...` decoration and item availability (#13).** Render
   `Trigger::hint()` as a `TextDecoration` at `menu.start+1..menu.start+1` — or, since
   zero-width decorations do not paint, as an absolutely-positioned muted label at
   `cursor_layout()`; add `check: fn(&NotionEditor,&App)->bool` to `SlashItem`.
9. **Selection visible while a popover has focus (#32).** On focus loss, push a
   `TextDecoration` over the last selected range with the selection fill (the
   decoration collection already exists per block, `view.rs:489-500`) and drop it on
   refocus.
10. **Home/End and scroll-into-view in menus (#15).** Capture `MoveToStartOfLine`/
    `MoveToEndOfLine` in `keymap.rs` while `suggestion_is_open()`, and give the row
    list a `ScrollHandle` so `menu.selected` can be scrolled into view.
11. **Heading and list metrics (#34, #35).** Change the heading margin factors in
    `blocks.rs:83-88` to `3.0/2.5/2.0` and `list_layout()` `margin_top` to 24 px with a
    matching bottom margin; this is the single largest visual divergence.
12. **Drag affordances (#21, #23).** `cursor_grab()` on the handle, a `dragging: bool`
    on the editor set by `on_drag`/`on_drop` that sets `cursor_grabbing()` on the root
    and hides the gutter, and a `crossAxis` rule using the already-measured
    `block.rows` (`block.rs:392`) for the >40 px case.

---

## 4. Deliberate deviations (platform-forced)

* **Marks are decorations, not styled spans; per-range font size/family is
  impossible.** `HighlightStyle` carries only
  `color, font_weight, font_style, background_color, underline, strikethrough, fade_out`
  (`docs/research/gpui-kit-api.md:414-417`, `:1085-1086`). Consequences:
  superscript/subscript have a mark kind and mutually-exclusive toggles but paint
  nothing (`view.rs:852`); inline `code` cannot be 0.875em or use a mono family
  inside prose, so it is shown as a tinted background plus Notion's red foreground
  (`view.rs:832-835`, `style.rs:55-69`); a completed to-do is struck through by
  synthesising a full-width `Strike` run (`view.rs:490-492`) rather than styling the
  text element.
* **One `EditorState` per block instead of one document buffer.** The same limit
  means "H1 inside the same input is impossible" (`gpui-kit-api.md:1085-1086`), so
  each node type gets its own input sized by its own `BlockLayout`
  (`view.rs:674-703`, created in `view.rs:234-265`). This is why the editor re-implements cross-block caret
  movement (`keymap.rs:273-347`), cross-block selection (`selection.rs`) and a
  document-level undo stack (`history.rs`) instead of getting them from the input.
* **No text alignment.** `InputState::set_text_align` is documented
  "single-line only, no-op otherwise" (`gpui-kit-api.md:92`), and every block is a
  soft-wrapping multi-line editor (`view.rs:258`), so the template's four align
  buttons and `Mod-Shift-L/E/R/J` are omitted rather than shipped dead
  (`toolbar.rs:155-169`).
* **Mentions and emoji are not inline atoms.** There is no inline-embed/atomic-widget
  API inside an input — `inline_object`/`InlineElement` belong to `TextView`, not to
  the input family (`gpui-kit-api.md:1087-1089`). A mention is therefore the person's
  name as plain text under a `Mention(id)` mark (`slash.rs:259-269`,
  `view.rs:848-851`) and an emoji is the literal character (`slash.rs:248-258`);
  neither deletes as one unit (#9).
* **Block text is a mirror plus a diff, not an edit stream.** The input reports only
  "something changed" (`gpui-kit-api.md` decoration section; `InputEvent::Change`),
  so `on_block_changed` recovers the edit from a common-prefix/suffix diff
  (`view.rs:388-441`, `mark.rs:432-467`) to remap marks — and infers a paste from
  "an insertion longer than one character containing a newline" (`view.rs:400-404`)
  because there is no paste event to hook.
* **Heights are estimated before layout.** The input lays out inside the height it is
  given, so each block's height is pre-computed by wrapping its text with
  `line_wrapper` and reconciled with the last measured bounds
  (`view.rs:602-655`), with the row count cached on the block (`block.rs:390-392`).
* **Document undo is a snapshot stack.** Per-input undo cannot see splits, joins or
  type changes across blocks, so `history.rs:17-134` snapshots
  `Vec<BlockContent>` and the editor captures the inputs' `Undo`/`Redo` actions on
  the way down (`keymap.rs:78-85`). Focus is restored on the next frame via
  `window.defer` (`history.rs:127-131`) because the replayed inputs are not yet in
  the focus tree.
* **Colors come from the gpui-kit theme, not the Tiptap token set.** Only the mark
  palettes are hard-coded (`style.rs:76-123`); page, border, popover, selection and
  link colors are read from `cx.theme()` so light/dark follow the host application
  (`style.rs:42-73`, `ui.rs:35-79`). This is the root cause of #39/#40/#41/#31.
