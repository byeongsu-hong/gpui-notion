use gpui_kit::assets::AllAssets;
use gpui_kit::component::Root;
use gpui_kit::*;
use gpui_notion::editor::block::{BlockAttrs, BlockContent, types};
use gpui_notion::editor::mark::{HighlightColor, Mark, MarkKind, MarkList};
use gpui_notion::editor::{self, NotionEditor};

/// The formatting sampler, with its marks measured off the text itself.
fn marked_paragraph() -> BlockContent {
    let text = "Bold, italic, underline, strike, code and a link.";
    let span = |needle: &str| {
        let start = text.find(needle).expect("sample text");
        start..start + needle.len()
    };
    BlockContent::paragraph(text).with_marks(MarkList::from_marks(vec![
        Mark::new(MarkKind::Bold, span("Bold")),
        Mark::new(MarkKind::Italic, span("italic")),
        Mark::new(MarkKind::Underline, span("underline")),
        Mark::new(MarkKind::Strike, span("strike")),
        Mark::new(MarkKind::Code, span("code")),
        Mark::new(MarkKind::Highlight(Some(HighlightColor::Yellow)), span("and")),
        Mark::new(MarkKind::Link("https://tiptap.dev".into()), span("a link")),
    ]))
}

fn demo_document() -> Vec<BlockContent> {
    let code = "fn greet(name: &str) {\n    println!(\"hello, {name}\");\n}";

    vec![
        BlockContent::new(types::HEADING, "Notion-like editor").with_attrs(BlockAttrs::level(1)),
        BlockContent::paragraph(
            "A block editor built with GPUI and gpui-kit, with Tiptap's commands, \
             markdown input rules and a slash menu. Every block is its own input, \
             and inline marks are painted over the text as decorations.",
        ),
        BlockContent::new(types::HEADING, "Inline formatting").with_attrs(BlockAttrs::level(2)),
        marked_paragraph(),
        BlockContent::new(types::HEADING, "Lists").with_attrs(BlockAttrs::level(2)),
        BlockContent::new(types::BULLET_LIST, "Type - or * to start a bullet list"),
        BlockContent::new(types::BULLET_LIST, "Tab nests, Shift-Tab lifts").with_indent(1),
        BlockContent::new(types::ORDERED_LIST, "Numbered lists start with 1."),
        BlockContent::new(types::TASK_LIST, "Finish the editor").with_attrs(BlockAttrs {
            checked: true,
            ..Default::default()
        }),
        BlockContent::new(types::TASK_LIST, "Ship it"),
        BlockContent::new(types::BLOCKQUOTE, "Type / anywhere to insert a block."),
        BlockContent::new(types::CODE_BLOCK, code).with_attrs(BlockAttrs::language("rust")),
        BlockContent::new(types::HORIZONTAL_RULE, ""),
        BlockContent::new(types::CALLOUT, "Callouts hold a note worth keeping."),
        BlockContent::paragraph(""),
    ]
}

/// Development aid: `NOTION_DEMO=toolbar` or `NOTION_DEMO=slash` starts the
/// window with that surface open, so it can be reviewed without driving the
/// pointer.
fn open_demo_state(editor: &mut NotionEditor, window: &mut Window, cx: &mut Context<NotionEditor>) {
    match std::env::var("NOTION_DEMO").as_deref() {
        Ok("toolbar") => editor.select_text_in_block(3, 0..12, window, cx),
        Ok("slash") => {
            editor.focus_last_block(window, cx);
            editor.open_slash_menu(window, cx);
        }
        _ => {}
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(AllAssets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        editor::init(cx);

        // Review aid: `NOTION_THEME=dark` starts in the dark palette.
        if std::env::var("NOTION_THEME").as_deref() == Ok("dark") {
            gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        }

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1100.), px(860.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| {
                    let mut editor = NotionEditor::with_content(demo_document(), window, cx);
                    open_demo_state(&mut editor, window, cx);
                    editor
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
