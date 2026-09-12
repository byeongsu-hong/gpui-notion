//! Test drive of the basics: typing, splitting, joining, navigation, lists,
//! markdown rules, marks and the slash menu — all through the real UI.

use gpui_kit::component::Root;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext, px, size};
use gpui_notion::editor::{self, CellPosition, MarkKind, NotionEditor, types};

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
            let offset = editor.caret_offset(id, cx)?;
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
            .map(|item| item.title().to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        titles,
        vec![
            "Heading 1".to_string(),
            "Heading 2".to_string(),
            "Heading 3".to_string()
        ]
    );

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

#[gpui_kit::test]
fn the_toolbar_appears_over_a_selection(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("select me", cx);
    assert!(!cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)));

    harness.press("shift-home", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)));

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.click("mark-bold", cx);
    })
    .unwrap();

    cx.update(|cx| {
        assert!(harness.editor.read(cx).content()[0]
            .marks
            .has(&gpui_notion::editor::MarkKind::Bold, &(0..9)));
    });
}

#[gpui_kit::test]
fn the_link_editor_sets_a_link_on_the_selection(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("tiptap", cx);
    harness.press("shift-home", cx);
    harness.press("secondary-shift-k", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).link_editor_is_open()));

    harness.type_text("tiptap.dev", cx);
    harness.press("enter", cx);

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        assert!(!editor.link_editor_is_open());
        let block = &editor.content()[0];
        let mark = block
            .marks
            .mark_at(
                &gpui_notion::editor::MarkKind::Link(Default::default()),
                0,
            )
            .expect("link mark");
        match &mark.kind {
            gpui_notion::editor::MarkKind::Link(href) => {
                assert_eq!(href.as_ref(), "https://tiptap.dev")
            }
            other => panic!("unexpected mark {other:?}"),
        }
    });
}

#[gpui_kit::test]
fn pasted_lines_become_blocks(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.ui(cx, |window, cx| {
        window.input("# Title\n- one\n- two", cx);
    });

    assert_eq!(harness.texts(cx), vec!["Title", "one", "two"]);
    assert_eq!(
        harness.types(cx),
        vec![types::HEADING, types::BULLET_LIST, types::BULLET_LIST]
    );
}

#[gpui_kit::test]
fn shift_down_selects_whole_blocks_and_backspace_removes_them(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("enter", cx);
    harness.type_text("three", cx);
    harness.press("up", cx);

    harness.press("shift-down", cx);
    assert_eq!(
        cx.update(|cx| harness.editor.read(cx).selected_blocks().len()),
        2
    );

    harness.press("backspace", cx);
    assert_eq!(harness.texts(cx), vec!["one"]);
}

#[gpui_kit::test]
fn select_all_escalates_from_block_to_document(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);

    harness.press("secondary-a", cx);
    assert!(!cx.update(|cx| harness.editor.read(cx).has_block_selection()));

    harness.press("secondary-a", cx);
    assert_eq!(
        cx.update(|cx| harness.editor.read(cx).selected_blocks().len()),
        2
    );
}

#[gpui_kit::test]
fn dragging_the_handle_reorders_blocks(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("enter", cx);
    harness.type_text("three", cx);
    assert_eq!(harness.texts(cx), vec!["one", "two", "three"]);

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.hover(("block", 1usize), cx);
        window.render_frame(cx);
        window.drag_to(("drag", 1usize), ("block", 3usize), cx);
    })
    .unwrap();
    cx.run_until_parked();

    // The pointer ends on the upper half of the third block, so the dragged
    // block lands above it.
    assert_eq!(harness.texts(cx), vec!["two", "one", "three"]);
}

#[gpui_kit::test]
fn shift_enter_breaks_the_line_inside_a_block(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("shift-enter", cx);
    harness.type_text("two", cx);

    assert_eq!(harness.texts(cx), vec!["one\ntwo"]);
}

#[gpui_kit::test]
fn arrows_move_within_a_wrapped_block_before_leaving_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("first", cx);
    harness.press("enter", cx);
    harness.type_text("one", cx);
    harness.press("shift-enter", cx);
    harness.type_text("two", cx);

    // The caret is on the second row of the second block: up stays inside it.
    harness.press("up", cx);
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(1));
    harness.press("up", cx);
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(0));
}

#[gpui_kit::test]
fn clicking_below_the_document_appends_a_paragraph(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("only", cx);

    harness.ui(cx, |window, cx| window.click("trailing-space", cx));
    assert_eq!(harness.texts(cx), vec!["only", ""]);
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(1));
}

#[gpui_kit::test]
fn deleting_the_last_block_leaves_an_empty_paragraph(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("# gone", cx);
    harness.press("secondary-shift-backspace", cx);

    assert_eq!(harness.texts(cx), vec![""]);
    assert_eq!(harness.types(cx), vec![types::PARAGRAPH]);
}

#[gpui_kit::test]
fn the_emoji_menu_replaces_the_shortcode(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("ship it :rocket", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).suggestion_is_open()));

    harness.press("enter", cx);
    assert_eq!(harness.texts(cx), vec!["ship it 🚀"]);
    assert!(!cx.update(|cx| harness.editor.read(cx).suggestion_is_open()));
}

#[gpui_kit::test]
fn the_mention_menu_inserts_a_marked_name(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("ping @ada", cx);
    assert!(cx.update(|cx| harness.editor.read(cx).suggestion_is_open()));

    harness.press("enter", cx);
    assert_eq!(harness.texts(cx), vec!["ping @Ada Lovelace "]);
    cx.update(|cx| {
        let block = &harness.editor.read(cx).content()[0];
        assert!(block.marks.has(
            &gpui_notion::editor::MarkKind::Mention(Default::default()),
            &(5..18)
        ));
    });
}

#[gpui_kit::test]
fn a_colon_in_prose_does_not_hold_a_menu_open(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("note: something", cx);
    assert!(!cx.update(|cx| harness.editor.read(cx).suggestion_is_open()));
    assert_eq!(harness.texts(cx), vec!["note: something"]);
}

#[gpui_kit::test]
fn arrow_down_leaves_a_code_block_at_the_end_of_the_document(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("```", cx);
    harness.type_text(" ", cx);
    assert_eq!(harness.types(cx), vec![types::CODE_BLOCK]);

    harness.type_text("let x = 1;", cx);
    harness.press("down", cx);

    assert_eq!(
        harness.types(cx),
        vec![types::CODE_BLOCK, types::PARAGRAPH]
    );
    assert_eq!(harness.caret(cx).map(|(ix, _)| ix), Some(1));
}

#[gpui_kit::test]
fn enter_inside_a_code_block_adds_a_line(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("``` ", cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);

    assert_eq!(harness.texts(cx), vec!["one\ntwo"]);
    assert_eq!(harness.types(cx), vec![types::CODE_BLOCK]);
}

#[gpui_kit::test]
fn enter_in_the_middle_of_a_heading_keeps_both_halves_headings(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("# one two", cx);
    for _ in 0..4 {
        harness.press("left", cx);
    }
    harness.press("enter", cx);

    assert_eq!(harness.texts(cx), vec!["one", " two"]);
    assert_eq!(harness.types(cx), vec![types::HEADING, types::HEADING]);

    // At the end of a heading, Enter starts a paragraph instead.
    harness.press("end", cx);
    harness.press("enter", cx);
    assert_eq!(
        harness.types(cx),
        vec![types::HEADING, types::HEADING, types::PARAGRAPH]
    );
}

#[gpui_kit::test]
fn the_slash_menu_groups_items_in_template_order(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/", cx);

    let groups: Vec<&str> = cx.update(|cx| {
        let mut seen: Vec<&str> = Vec::new();
        for item in harness.editor.read(cx).suggestion_items(cx) {
            if seen.last() != Some(&item.group()) {
                seen.push(item.group());
            }
        }
        seen
    });
    assert_eq!(groups, vec!["Style", "Insert", "Upload"]);
}

#[gpui_kit::test]
fn the_emoji_entry_of_the_slash_menu_opens_the_emoji_menu(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/emoji", cx);
    harness.press("enter", cx);

    assert!(cx.update(|cx| harness.editor.read(cx).suggestion_is_open()));
    harness.type_text("rocket", cx);
    harness.press("enter", cx);
    assert_eq!(harness.texts(cx), vec!["🚀"]);
}

#[gpui_kit::test]
fn an_image_block_takes_a_dropped_file(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/image", cx);
    harness.press("enter", cx);
    assert_eq!(harness.types(cx)[0], types::IMAGE);

    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0)).unwrap();
    cx.update(|cx| {
        harness.editor.clone().update(cx, |editor, cx| {
            editor.set_image_source(id, std::path::PathBuf::from("/tmp/picture.png"), cx)
        })
    });

    cx.update(|cx| {
        let block = &harness.editor.read(cx).content()[0];
        assert_eq!(block.attrs.src.as_deref(), Some("/tmp/picture.png"));
        assert_eq!(block.attrs.alt.as_deref(), Some("picture.png"));
    });
}

#[gpui_kit::test]
fn dragging_across_blocks_selects_them(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("enter", cx);
    harness.type_text("three", cx);

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.drag_to(("block", 1usize), ("block", 3usize), cx);
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(cx.update(|cx| harness.editor.read(cx).selected_blocks().len()), 3);

    // Backspace on that selection takes all three blocks away.
    harness.ui(cx, |window, cx| window.press("backspace", cx));
    assert_eq!(harness.texts(cx), vec![""]);
}

#[gpui_kit::test]
fn shift_clicking_another_block_selects_the_range(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("enter", cx);
    harness.type_text("three", cx);

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.click(("block", 1usize), cx);
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        let position = window.find(("block", 3usize)).bounds().center();
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                button: gpui_kit::MouseButton::Left,
                position,
                modifiers: gpui_kit::Modifiers {
                    shift: true,
                    ..Default::default()
                },
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(cx.update(|cx| harness.editor.read(cx).selected_blocks().len()), 3);
}

#[gpui_kit::test]
fn typing_over_selected_blocks_replaces_them(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("shift-up", cx);
    assert_eq!(cx.update(|cx| harness.editor.read(cx).selected_blocks().len()), 2);

    harness.type_text("x", cx);
    assert_eq!(harness.texts(cx), vec!["x"]);
}

#[gpui_kit::test]
fn bold_over_selected_blocks_marks_all_of_them(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("shift-up", cx);

    harness.press("secondary-b", cx);
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        for block in editor.content() {
            assert!(
                block.marks.has(&MarkKind::Bold, &(0..block.text.len())),
                "block {:?} is not bold",
                block.text
            );
        }
    });

    // A second press takes it off every block again.
    harness.press("secondary-b", cx);
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        for block in editor.content() {
            assert!(!block.marks.has(&MarkKind::Bold, &(0..block.text.len())));
        }
    });
}

#[gpui_kit::test]
fn turning_selected_blocks_into_a_list_changes_all_of_them(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("shift-up", cx);

    harness.press("secondary-shift-8", cx);
    assert_eq!(
        harness.types(cx),
        vec![types::BULLET_LIST, types::BULLET_LIST]
    );
}

#[gpui_kit::test]
fn commenting_a_selection_opens_a_thread(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("needs review", cx);
    harness.ui(cx, |window, cx| {
        window.press("secondary-a", cx);
        window.press("secondary-shift-m", cx);
    });

    let (threads, quote) = cx.update(|cx| {
        let editor = harness.editor.read(cx);
        (
            editor.comment_threads().len(),
            editor
                .comment_threads()
                .first()
                .map(|thread| thread.quote().to_string()),
        )
    });
    assert_eq!(threads, 1);
    assert_eq!(quote.as_deref(), Some("needs review"));

    // The commented range carries the mark that anchors the thread.
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let id = editor.comment_threads()[0].id();
        let block = &editor.content()[0];
        assert!(block.marks.has(&MarkKind::Comment(id), &(0..block.text.len())));
    });

    // Typing into the draft and pressing Enter posts the comment.
    harness.ui(cx, |window, cx| {
        window.input("looks good", cx);
        window.press("enter", cx);
    });
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let thread = &editor.comment_threads()[0];
        assert_eq!(thread.comments().len(), 1);
        assert_eq!(thread.comments()[0].body(), "looks good");
        assert_eq!(thread.comments()[0].author(), "You");
    });
}

#[gpui_kit::test]
fn an_abandoned_comment_leaves_no_thread_behind(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("draft text", cx);
    harness.ui(cx, |window, cx| {
        window.press("secondary-a", cx);
        window.press("secondary-shift-m", cx);
        window.press("escape", cx);
    });

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        assert!(editor.comment_threads().is_empty());
        assert!(editor.content()[0].marks.is_empty());
    });
}

#[gpui_kit::test]
fn resolving_a_thread_takes_the_highlight_off(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("ship it", cx);
    harness.ui(cx, |window, cx| {
        window.press("secondary-a", cx);
        window.press("secondary-shift-m", cx);
        window.input("done", cx);
        window.press("enter", cx);
    });

    let id = cx.update(|cx| harness.editor.read(cx).comment_threads()[0].id());
    harness.ui(cx, |window, cx| window.click("resolve-thread", cx));

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let thread = editor.comment_thread(id).expect("thread is kept");
        assert!(thread.is_resolved());
        assert!(editor.content()[0].marks.is_empty());
        assert!(editor.open_thread().is_none());
    });
}

#[gpui_kit::test]
fn the_slash_menu_inserts_a_table_and_tab_walks_its_cells(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);

    let (ty, rows, columns) = cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let id = editor.block_id_at(0).expect("a table block");
        let grid = editor.grid(id).expect("a grid beside it");
        (editor.content()[0].ty.to_string(), grid.rows(), grid.columns())
    });
    assert_eq!(ty, types::TABLE);
    assert_eq!((rows, columns), (3, 3));

    // The caret starts in the header cell; Tab walks across the row.
    harness.type_text("Name", cx);
    harness.press("tab", cx);
    harness.type_text("Role", cx);

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let id = editor.block_id_at(0).unwrap();
        let grid = editor.grid(id).unwrap();
        assert_eq!(grid.cell(CellPosition::new(0, 0)).unwrap().text(), "Name");
        assert_eq!(grid.cell(CellPosition::new(0, 1)).unwrap().text(), "Role");
    });
}

#[gpui_kit::test]
fn tab_in_the_last_cell_adds_a_row(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);

    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0).unwrap());
    // Walk to the last cell: 3 x 3 - 1 tabs.
    for _ in 0..8 {
        harness.press("tab", cx);
    }
    harness.press("tab", cx);

    let rows = cx.update(|cx| harness.editor.read(cx).grid(id).unwrap().rows());
    assert_eq!(rows, 4);
}

#[gpui_kit::test]
fn a_table_survives_undo_and_redo_with_its_text(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    harness.type_text("Kept", cx);

    harness.press("secondary-z", cx);
    harness.press("secondary-shift-z", cx);

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let id = editor.block_id_at(0).expect("the table is back");
        let grid = editor.grid(id).expect("with a grid");
        assert_eq!(grid.cell(CellPosition::new(0, 0)).unwrap().text(), "Kept");
    });
}

#[gpui_kit::test]
fn a_table_grows_and_shrinks_by_row_and_column(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0).unwrap());

    // Type in the header so the shape is observable after the edits.
    harness.type_text("A", cx);
    harness.press("tab", cx);
    harness.type_text("B", cx);

    // The controls show while the pointer is over the table.
    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.hover(("block", id.0 as usize), cx);
        window.render_frame(cx);
        window.click(("add-column", id.0 as usize), cx);
        window.render_frame(cx);
        window.hover(("block", id.0 as usize), cx);
        window.render_frame(cx);
        window.click(("add-row", id.0 as usize), cx);
    })
    .unwrap();
    cx.run_until_parked();

    let (rows, columns) = cx.update(|cx| {
        let grid = harness.editor.read(cx).grid(id).unwrap();
        (grid.rows(), grid.columns())
    });
    assert_eq!((rows, columns), (4, 4));

    // The row control beside the first row takes that row away.
    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.hover(("block", id.0 as usize), cx);
        window.render_frame(cx);
        window.click(("drop-row", 0usize), cx);
    })
    .unwrap();
    cx.run_until_parked();

    let (rows, header) = cx.update(|cx| {
        let grid = harness.editor.read(cx).grid(id).unwrap();
        (
            grid.rows(),
            grid.cell(CellPosition::new(0, 0)).unwrap().text().to_string(),
        )
    });
    assert_eq!(rows, 3);
    assert_eq!(header, "", "the row carrying A was the one removed");
}

#[gpui_kit::test]
fn a_table_copies_as_a_markdown_table(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    harness.type_text("Name", cx);
    harness.press("tab", cx);
    harness.type_text("Role", cx);

    harness.press("escape", cx);
    let markdown = cx.update(|cx| harness.editor.read(cx).selected_markdown(cx));
    assert!(
        markdown.starts_with("| Name | Role |"),
        "unexpected markdown: {markdown}"
    );
    assert!(markdown.contains("| --- | --- |"));
}

#[gpui_kit::test]
fn the_toolbar_waits_for_the_button_to_come_up(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("select me", cx);

    // Mid-drag the toolbar stays away, however much text is covered.
    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        let bounds = window.find(("block", 1usize)).bounds();
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                button: gpui_kit::MouseButton::Left,
                position: bounds.center(),
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.render_frame(cx);
    })
    .unwrap();
    assert!(cx.update(|cx| harness.editor.read(cx).press_in_progress()));
    assert!(!cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)));

    // Once the button comes up a selection shows it again.
    cx.update_window(harness.window, |_, window, cx| {
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                button: gpui_kit::MouseButton::Left,
                position: window.find(("block", 1usize)).bounds().center(),
                modifiers: Default::default(),
                click_count: 1,
            }),
            cx,
        );
        window.render_frame(cx);
        window.press("secondary-a", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)));
}

#[gpui_kit::test]
fn dragging_a_selected_block_moves_the_whole_selection(cx: &mut TestAppContext) {
    let harness = setup(cx);
    for (ix, text) in ["one", "two", "three", "four"].iter().enumerate() {
        if ix > 0 {
            harness.press("enter", cx);
        }
        harness.type_text(text, cx);
    }
    assert_eq!(harness.texts(cx), vec!["one", "two", "three", "four"]);

    // Select "one" and "two", then drag the handle of "one" past "four".
    harness.ui(cx, |window, cx| {
        window.click(("block", 1usize), cx);
    });
    harness.press("shift-down", cx);
    assert_eq!(cx.update(|cx| harness.editor.read(cx).selected_blocks().len()), 2);

    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.hover(("block", 1usize), cx);
        window.render_frame(cx);
        window.drag_to(("drag", 1usize), ("block", 4usize), cx);
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(harness.texts(cx), vec!["three", "one", "two", "four"]);
}

#[gpui_kit::test]
fn duplicating_a_selection_copies_every_block_below_it(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);
    harness.press("shift-up", cx);

    harness.press("secondary-d", cx);
    assert_eq!(harness.texts(cx), vec!["one", "two", "one", "two"]);
    assert_eq!(cx.update(|cx| harness.editor.read(cx).selected_blocks().len()), 2);
}

#[gpui_kit::test]
fn the_comment_box_takes_the_toolbar_off_screen(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one line", cx);
    harness.press("secondary-a", cx);
    assert!(
        cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)),
        "a selection shows the toolbar"
    );

    harness.press("secondary-shift-m", cx);
    assert!(
        !cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)),
        "the comment box replaces it"
    );

    harness.press("escape", cx);
    assert!(
        cx.update(|cx| harness.editor.read(cx).selection_toolbar_visible(cx)),
        "cancelling the comment gives the selection, and the toolbar, back"
    );
}

#[gpui_kit::test]
fn clicking_different_lines_of_a_block_does_not_move_the_page(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text(
        "This paragraph is deliberately long enough that it wraps onto several rows inside its own input, which is where a click on one row must not change the height of anything.",
        cx,
    );
    harness.press("enter", cx);
    harness.type_text("below", cx);

    let heights = |cx: &mut TestAppContext| -> (f32, f32) {
        cx.update_window(harness.window, |_, window, cx| {
            window.render_frame(cx);
            (
                f32::from(window.find(("block", 1usize)).bounds().size.height),
                f32::from(window.find(("block", 2usize)).bounds().origin.y),
            )
        })
        .unwrap()
    };

    let first = heights(cx);
    for offset in [4., 20., 40., 60.] {
        cx.update_window(harness.window, |_, window, cx| {
            window.render_frame(cx);
            window.click_at(("block", 1usize), gpui_kit::point(px(200.), px(offset)), cx);
        })
        .unwrap();
        cx.run_until_parked();
        let now = heights(cx);
        assert_eq!(
            now, first,
            "clicking at y={offset} changed the block height or what follows it"
        );
    }
}

#[gpui_kit::test]
fn clicking_the_last_row_does_not_scroll_the_block(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text(
        "This paragraph is deliberately long enough that it wraps onto several rows inside its own input, which is where a click on one row must not shift the text that is already on screen.",
        cx,
    );

    let offset_after_click = |y: f32, cx: &mut TestAppContext| -> (f32, f32) {
        cx.update_window(harness.window, |_, window, cx| {
            window.render_frame(cx);
            window.click_at(("block", 1usize), gpui_kit::point(px(200.), px(y)), cx);
            window.render_frame(cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            let editor = harness.editor.read(cx);
            let id = editor.block_id_at(0).unwrap();
            let offset = editor.block_scroll_offset(id, cx).unwrap();
            (f32::from(offset.x), f32::from(offset.y))
        })
    };

    let top = offset_after_click(4., cx);
    let bottom = offset_after_click(60., cx);
    assert_eq!(top, (0., 0.), "the block starts unscrolled");
    assert_eq!(bottom, top, "clicking the last row scrolled the block");

    // The reason it cannot scroll: the text area is never shorter than the
    // text, whatever the input keeps for itself.
    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        for block in editor.content() {
            let id = editor.block_id_at(0).unwrap();
            let _ = block;
            assert!(
                editor.text_area_fits_text(id, cx),
                "the text area is shorter than the text in it"
            );
        }
    });
}

#[gpui_kit::test]
fn every_block_gets_a_text_area_that_fits_its_text(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("# A heading that is long enough to wrap once it reaches the end of the column", cx);
    harness.press("enter", cx);
    harness.type_text(
        "A paragraph long enough to wrap across three rows, which is the shape that makes a short text area show up as text jumping whenever the caret changes row.",
        cx,
    );
    harness.press("enter", cx);
    harness.type_text("- a list item", cx);

    cx.update_window(harness.window, |_, window, cx| {
        // A second frame lets the measured inset settle.
        window.render_frame(cx);
        window.render_frame(cx);
    })
    .unwrap();

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        for ix in 0..editor.content().len() {
            let id = editor.block_id_at(ix).unwrap();
            assert!(
                editor.text_area_fits_text(id, cx),
                "block {ix} has a text area shorter than its text"
            );
            assert_eq!(
                editor.block_scroll_offset(id, cx).map(|p| f32::from(p.y)),
                Some(0.),
                "block {ix} scrolled inside itself"
            );
        }
    });
}

#[gpui_kit::test]
fn a_cell_grows_to_hold_text_that_wraps(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0).unwrap());

    let one_row = cx.update(|cx| {
        f32::from(harness.editor.read(cx).grid(id).unwrap().row_text_height(0))
    });

    harness.type_text(
        "a cell with quite a lot of text in it, far more than one row can hold",
        cx,
    );
    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
    })
    .unwrap();

    let (wrapped, fits, scroll) = cx.update(|cx| {
        let editor = harness.editor.read(cx);
        (
            f32::from(editor.grid(id).unwrap().row_text_height(0)),
            editor.cells_fit_their_text(id, cx),
            editor.cell_scroll_offset(id, CellPosition::new(0, 0), cx),
        )
    });
    assert!(
        wrapped > one_row,
        "the row stayed {one_row} tall for text that has to wrap (now {wrapped})"
    );
    let metrics = cx.update(|cx| {
        harness
            .editor
            .read(cx)
            .cell_metrics(id, CellPosition::new(0, 0), cx)
    });
    assert!(fits, "a cell is shorter than the text in it");
    assert_eq!(
        scroll,
        Some(0.),
        "a cell scrolled inside itself; metrics (needed, area, line, scroll) = {metrics:?}"
    );
}

#[gpui_kit::test]
fn every_cell_of_a_row_is_as_tall_as_the_row(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0).unwrap());

    // One cell of the second row takes two lines; the rest of the row follows.
    cx.update_window(harness.window, |_, window, cx| {
        harness.editor.clone().update(cx, |editor, cx| {
            editor.set_cell_text(
                id,
                CellPosition::new(1, 1),
                "text long enough that this cell has to take a second line for it",
                window,
                cx,
            );
        });
        window.render_frame(cx);
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let grid = editor.grid(id).unwrap();
        assert!(
            grid.row_text_height(1) > grid.row_text_height(0),
            "the row with wrapped text did not grow"
        );
        for column in 0..grid.columns() {
            let at = CellPosition::new(1, column);
            let (needed, area, _, scroll) = editor.cell_metrics(id, at, cx).unwrap();
            assert!(
                area + 0.01 >= needed,
                "cell {column} has {area} of room for {needed} of text"
            );
            assert_eq!(scroll, 0., "cell {column} scrolled inside itself");
        }
        assert!(editor.cells_fit_their_text(id, cx));
    });
}

#[gpui_kit::test]
fn a_table_in_a_loaded_document_comes_back_with_its_cells(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(editor::init);

    let content = vec![
        gpui_notion::editor::block::BlockContent::paragraph("above"),
        editor::table_content(&[&["Name", "Role"], &["Ada", "Author"]]),
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
        let id = editor.block_id_at(1).expect("the table block");
        let grid = editor.grid(id).expect("its cells");
        assert_eq!((grid.rows(), grid.columns()), (2, 2));
        assert_eq!(grid.cell(CellPosition::new(0, 0)).unwrap().text(), "Name");
        assert_eq!(grid.cell(CellPosition::new(1, 1)).unwrap().text(), "Author");
        assert_eq!(
            editor.markdown_of(&[id], cx),
            "| Name | Role |\n| --- | --- |\n| Ada | Author |"
        );
    });
}

#[gpui_kit::test]
fn a_table_cell_has_nothing_to_scroll(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("/table", cx);
    harness.press("enter", cx);
    harness.type_text("Name", cx);
    let id = cx.update(|cx| harness.editor.read(cx).block_id_at(0).unwrap());

    // Asking a cell to scroll a long way is clamped to whatever room it has
    // below its last line, which for a cell sized to its text is none.
    let states: Vec<_> = cx.update(|cx| {
        let editor = harness.editor.read(cx);
        let grid = editor.grid(id).unwrap();
        grid.positions()
            .map(|at| (at, grid.cell(at).unwrap().state().clone()))
            .collect()
    });
    cx.update_window(harness.window, |_, window, cx| {
        for (_, state) in &states {
            state.update(cx, |state, cx| {
                state.set_scroll_offset(gpui_kit::point(px(0.), px(-500.)), cx)
            });
        }
        window.render_frame(cx);
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update(|cx| {
        let editor = harness.editor.read(cx);
        for (at, _) in &states {
            assert_eq!(
                editor.cell_scroll_offset(id, *at, cx),
                Some(0.),
                "cell {at:?} had room to scroll"
            );
        }
    });
}

#[gpui_kit::test]
fn a_narrow_window_reflows_instead_of_clipping(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(editor::init);

    let content = vec![gpui_notion::editor::block::BlockContent::paragraph(
        "A paragraph that is comfortably one row wide in a roomy window and has to \
         take several rows in a narrow one, which is the case that used to leave \
         blocks sized for a width they never got.",
    )];
    let mut view = None;
    // Narrower than the 708px content column.
    let handle = cx.open_window(size(px(420.), px(600.)), |window, cx| {
        let editor = cx.new(|cx| NotionEditor::with_content(content.clone(), window, cx));
        view = Some(editor.clone());
        Root::new(editor, window, cx)
    });
    let view = view.unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
    })
    .unwrap();

    cx.update(|cx| {
        let editor = view.read(cx);
        let id = editor.block_id_at(0).unwrap();
        assert!(
            editor.text_area_fits_text(id, cx),
            "the block is shorter than its text in a narrow window"
        );
        assert_eq!(
            editor.block_scroll_offset(id, cx).map(|p| f32::from(p.y)),
            Some(0.),
            "the block scrolled inside itself in a narrow window"
        );
    });
}

#[gpui_kit::test]
fn a_drag_that_starts_at_the_left_of_the_text_still_selects_blocks(cx: &mut TestAppContext) {
    let harness = setup(cx);
    harness.type_text("one", cx);
    harness.press("enter", cx);
    harness.type_text("two", cx);

    // Press on the first glyph of the block, not in the middle of the row.
    cx.update_window(harness.window, |_, window, cx| {
        window.render_frame(cx);
        let id = harness.editor.read(cx).block_id_at(0).unwrap();
        let from = harness
            .editor
            .read(cx)
            .block_bounds(id)
            .expect("the block has laid out");
        let to = window.find(("block", 2usize)).bounds();
        window.drag(
            gpui_kit::point(from.left() + px(2.), from.center().y),
            to.center(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(
        cx.update(|cx| harness.editor.read(cx).selected_blocks().len()),
        2
    );
}
