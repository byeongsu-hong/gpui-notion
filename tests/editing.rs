//! Test drive of the basics: typing, splitting, joining, navigation, lists,
//! markdown rules, marks and the slash menu — all through the real UI.

use gpui_kit::component::Root;
use gpui_kit::test::{TestSupportExt as _, TestWindowExt as _};
use gpui_kit::{AnyWindowHandle, AppContext as _, Context, Entity, TestAppContext, px, size};
use gpui_notion::editor::{self, NotionEditor, types};

struct Harness {
    editor: Entity<NotionEditor>,
    window: AnyWindowHandle,
}

fn setup(cx: &mut TestAppContext) -> Harness {
    cx.update(gpui_kit::init);
    cx.update(editor::init);

    let mut editor = None;
    let handle = cx.open_window(size(px(900.), px(700.)), |window, cx| {
        let view = cx.new(|cx| NotionEditor::new(window, cx));
        editor = Some(view.clone());
        Root::new(view, window, cx)
    });

    let harness = Harness {
        editor: editor.unwrap(),
        window: handle.into(),
    };
    harness.focus_first(cx);
    harness
}

impl Harness {
    /// Run `f` inside the window, with a frame rendered first.
    fn ui<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App) -> R,
    ) -> R {
        let result = cx
            .update_window(self.window, |_, window, cx| {
                window.render_frame(cx);
                let result = f(window, cx);
                // A frame after the interaction delivers focus changes.
                window.render_frame(cx);
                result
            })
            .unwrap();
        // Deferred work — focus moves, overlays — lands before the next step.
        cx.run_until_parked();
        result
    }

    fn focus_first(&self, cx: &mut TestAppContext) {
        self.ui(cx, |window, cx| {
            window.click(("block", 1usize), cx);
        });
    }

    /// Type the way a person does: one character at a time, so input rules
    /// and the slash menu see each keystroke.
    fn type_text(&self, text: &str, cx: &mut TestAppContext) {
        for ch in text.chars() {
            let ch = ch.to_string();
            self.ui(cx, |window, cx| window.input(&ch, cx));
        }
    }

    fn press(&self, key: &str, cx: &mut TestAppContext) {
        self.ui(cx, |window, cx| window.press(key, cx));
    }

    fn texts(&self, cx: &mut TestAppContext) -> Vec<String> {
        cx.update(|cx| {
            self.editor
                .read(cx)
                .content()
                .into_iter()
                .map(|block| block.text)
                .collect()
        })
    }

    fn types(&self, cx: &mut TestAppContext) -> Vec<String> {
        cx.update(|cx| {
            self.editor
                .read(cx)
                .content()
                .into_iter()
                .map(|block| block.ty.to_string())
                .collect()
        })
    }

    fn indents(&self, cx: &mut TestAppContext) -> Vec<usize> {
        cx.update(|cx| {
            self.editor
                .read(cx)
                .content()
                .into_iter()
                .map(|block| block.indent)
                .collect()
        })
    }

    /// (block index, caret offset) of the focused block.
    fn caret(&self, cx: &mut TestAppContext) -> Option<(usize, usize)> {
        cx.update(|cx| {
            let editor = self.editor.read(cx);
            let id = editor.focused_id()?;
            let ix = editor.index_of(id)?;
            let offset = editor.block(id)?.state.read(cx).cursor();
            Some((ix, offset))
        })
    }
}

#[gpui_kit::test]
fn types_text_into_the_first_block(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("Hello editor", cx);
    assert_eq!(harness.texts(cx), vec!["Hello editor"]);
}

#[gpui_kit::test]
fn enter_splits_a_block_at_the_caret(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("Hello world", cx);
    harness.press("left", cx);
    harness.press("left", cx);
    harness.press("left", cx);
    harness.press("left", cx);
    harness.press("left", cx);
    harness.press("enter", cx);

    assert_eq!(harness.texts(cx), vec!["Hello ", "world"]);
    assert_eq!(harness.caret(cx), Some((1, 0)));
}

#[gpui_kit::test]
fn backspace_at_the_start_joins_with_the_previous_block(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("first", cx);
    harness.press("enter", cx);
    harness.type_text("second", cx);
    assert_eq!(harness.texts(cx), vec!["first", "second"]);

    for _ in 0..6 {
        harness.press("left", cx);
    }
    harness.press("backspace", cx);

    assert_eq!(harness.texts(cx), vec!["firstsecond"]);
    assert_eq!(harness.caret(cx), Some((0, 5)));
}

#[gpui_kit::test]
fn arrows_walk_between_blocks(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);

    harness.press("up", cx);
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(0));
    harness.press("down", cx);
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(1));

    // Left at offset 0 crosses to the end of the block above.
    harness.press("home", cx);
    harness.press("left", cx);
    assert_eq!(harness.caret(cx), Some((0, 3)));
    harness.press("right", cx);
    assert_eq!(harness.caret(cx), Some((1, 0)));
}

#[gpui_kit::test]
fn markdown_rules_create_nodes(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("# Title", cx);
    assert_eq!(harness.types(cx), vec![types::HEADING]);
    assert_eq!(harness.texts(cx), vec!["Title"]);

    harness.press("enter", cx);
    harness.type_text("- item", cx);
    assert_eq!(
        harness.types(cx),
        vec![types::HEADING, types::BULLET_LIST]
    );
    assert_eq!(harness.texts(cx)[1], "item");

    harness.press("enter", cx);
    harness.type_text("1. one", cx);
    assert_eq!(harness.types(cx)[2], types::ORDERED_LIST);

    harness.press("enter", cx);
    harness.press("enter", cx);
    harness.type_text("> quoted", cx);
    assert_eq!(*harness.types(cx).last().unwrap(), types::BLOCKQUOTE);
}

#[gpui_kit::test]
fn enter_continues_a_list_and_an_empty_item_leaves_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("- one", cx);
    harness.press("enter", cx);
    assert_eq!(harness.types(cx)[1], types::BULLET_LIST);

    harness.type_text("two", cx);
    harness.press("enter", cx);
    harness.press("enter", cx);
    assert_eq!(*harness.types(cx).last().unwrap(), types::PARAGRAPH);
}

#[gpui_kit::test]
fn tab_nests_a_list_item_and_shift_tab_lifts_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("- one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);

    harness.press("tab", cx);
    assert_eq!(harness.indents(cx), vec![0, 1]);
    harness.press("shift-tab", cx);
    assert_eq!(harness.indents(cx), vec![0, 0]);
    // The first item has nothing to nest under.
    harness.press("up", cx);
    harness.press("tab", cx);
    assert_eq!(harness.indents(cx), vec![0, 0]);
}

#[gpui_kit::test]
fn inline_rules_apply_marks(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("say **bold** now", cx);

    assert_eq!(harness.texts(cx), vec!["say bold now"]);
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let block = &editor.content()[0];
        assert!(block.marks.has(&gpui_notion::editor::MarkKind::Bold, &(4..8)));
        assert!(!block.marks.has(&gpui_notion::editor::MarkKind::Bold, &(9..12)));
    });
}

#[gpui_kit::test]
fn the_bold_shortcut_marks_the_selection(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("hello", cx);
    harness.press("shift-home", cx);
    harness.press("secondary-b", cx);

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        assert!(editor.content()[0]
            .marks
            .has(&gpui_notion::editor::MarkKind::Bold, &(0..5)));
    });
}

#[gpui_kit::test]
fn the_slash_menu_filters_and_runs_an_item(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).slash_menu_is_open()));

    harness.type_text("head", cx);
    let titles = cx.update(|cx| {
        harness
            .editor
            .read(cx)
            .slash_items(cx)
            .into_iter()
            .map(|item| item.title)
            .collect::<Vec<_>>()
    });
    assert_eq!(titles, vec!["Heading 1", "Heading 2", "Heading 3"]);

    harness.press("down", cx);
    harness.press("enter", cx);

    assert_eq!(harness.types(cx), vec![types::HEADING]);
    assert_eq!(harness.texts(cx), vec![""]);
    assert!(!cx.update(|cx| harness.editor.read(cx).slash_menu_is_open()));
    cx.update(|cx| assert_eq!(harness.editor.read(cx).content()[0].attrs.level, 2));
}

#[gpui_kit::test]
fn escape_closes_the_slash_menu_and_keeps_the_text(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("a /he", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).slash_menu_is_open()));
    harness.press("escape", cx);
    assert!(!cx.update(|cx| harness.editor.read(cx).slash_menu_is_open()));
    assert_eq!(harness.texts(cx), vec!["a /he"]);
}

#[gpui_kit::test]
fn typing_after_bold_text_continues_the_mark(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("hi", cx);
    harness.press("shift-home", cx);
    harness.press("secondary-b", cx);
    harness.press("end", cx);
    harness.type_text("!", cx);

    cx.update(|cx| {
        let block = &harness.editor.read(cx).content()[0];
        assert!(block.marks.has(&gpui_notion::editor::MarkKind::Bold, &(0..3)));
    });
}

#[gpui_kit::test]
fn typing_after_an_inline_rule_is_plain(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("*it* rest", cx);
    cx.update(|cx| {
        let block = &harness.editor.read(cx).content()[0];
        assert_eq!(block.text, "it rest");
        assert!(block.marks.has(&gpui_notion::editor::MarkKind::Italic, &(0..2)));
        assert!(!block.marks.has(&gpui_notion::editor::MarkKind::Italic, &(3..7)));
    });
}

#[gpui_kit::test]
fn the_gutter_plus_adds_a_block_below(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("first", cx);
    let id = cx.update(|cx| harness.editor.read(cx).content().len());
    assert_eq!(id, 1);

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.hover(("block", 1usize), cx);
        window.render_frame(cx);
        window.click(("insert", 1usize), cx);
    })
    .unwrap();

    assert_eq!(harness.texts(cx), vec!["first", "/"]);
}

#[gpui_kit::test]
fn a_block_can_be_dragged_below_another(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    assert_eq!(harness.texts(cx), vec!["one", "two"]);

    cx.update(|cx| {
        harness.editor.clone().update(cx, |editor, cx| {
            editor.reorder_block(0, 2, cx);
        })
    });
    assert_eq!(harness.texts(cx), vec!["two", "one"]);
}

#[gpui_kit::test]
fn a_document_loads_exactly_the_blocks_it_was_given(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(editor::init);

    let content = vec![
        gpui_notion::editor::block::BlockContent::paragraph("one"),
        gpui_notion::editor::block::BlockContent::paragraph("two"),
        gpui_notion::editor::block::BlockContent::paragraph(""),
    ];
    let mut view = None;
    let handle = cx.open_window(size(px(900.), px(700.)), |window, cx| {
        let editor = cx.new(|cx| NotionEditor::with_content(content.clone(), window, cx));
        view = Some(editor.clone());
        Root::new(editor, window, cx)
    });
    let view = view.unwrap();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();

    cx.update(|cx| {
        let editor = view.read(cx);
        assert_eq!(
            editor
                .content()
                .into_iter()
                .map(|block| block.text)
                .collect::<Vec<_>>(),
            vec!["one", "two", ""]
        );
        assert!(!editor.slash_menu_is_open());
    });
}

#[gpui_kit::test]
fn undo_reverts_typing_and_redo_restores_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("hello", cx);
    assert_eq!(harness.texts(cx), vec!["hello"]);

    harness.press("secondary-z", cx);
    assert_eq!(harness.texts(cx), vec![""]);

    assert!(cx.update(|cx| harness.editor.read(cx).focused_id().is_some()), "focus after undo");
    harness.press("secondary-y", cx);
    assert_eq!(harness.texts(cx), vec!["hello"]);
}

#[gpui_kit::test]
fn undo_reverts_a_split(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one two", cx);
    harness.press("enter", cx);
    assert_eq!(harness.texts(cx), vec!["one two", ""]);

    harness.press("secondary-z", cx);
    assert_eq!(harness.texts(cx), vec!["one two"]);
}

#[gpui_kit::test]
fn undo_reverts_a_node_change(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("title", cx);
    harness.press("secondary-alt-1", cx);
    assert_eq!(harness.types(cx), vec![types::HEADING]);

    harness.press("secondary-z", cx);
    assert_eq!(harness.types(cx), vec![types::PARAGRAPH]);
    assert_eq!(harness.texts(cx), vec!["title"]);
}

#[gpui_kit::test]
fn redo_through_the_api(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("hello", cx);
    cx.update_window(harness.window, |_, window, cx| {
        harness.editor.clone().update(cx, |editor, cx| editor.undo(window, cx));
    }).unwrap();
    assert_eq!(harness.texts(cx), vec![""]);
    cx.update_window(harness.window, |_, window, cx| {
        harness.editor.clone().update(cx, |editor, cx| editor.redo(window, cx));
    }).unwrap();
    assert_eq!(harness.texts(cx), vec!["hello"]);
}

#[gpui_kit::test]
fn clicking_a_block_focuses_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    let (focused, handle_focused) = cx
        .update_window(harness.window, |_, window, cx| {
            let editor = harness.editor.read(cx);
            let id = editor.focused_id();
            let block = &editor.content();
            let _ = block;
            let handle = harness
                .editor
                .read(cx)
                .block_focus_handle(0, cx)
                .map(|h| h.is_focused(window));
            (id, handle)
        })
        .unwrap();
    assert_eq!(handle_focused, Some(true), "the input has keyboard focus");
    assert!(focused.is_some(), "the editor tracks the clicked block");
}
