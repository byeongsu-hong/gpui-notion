//! Key handling.
//!
//! The block inputs bind their own actions in the `Input` key context, so the
//! editor pre-empts the ones that mean something at block level — Enter,
//! Backspace at the start, arrows at an edge, Tab — by capturing the action on
//! the way down and stopping it there.

use gpui_kit::component::input::{
    Backspace, Delete, Enter, Escape, IndentInline, MoveDown, MoveLeft, MoveRight, MoveUp,
    OutdentInline,
};
use gpui_kit::{App, Context, Focusable as _, InteractiveElement, Window};

use super::actions;
use super::block::types;
use super::mark::{HighlightColor, MarkKind};
use super::view::{Caret, NotionEditor};

impl NotionEditor {
    /// Caret row within the block's wrapped text, and the row count.
    pub(crate) fn caret_row(&self, ix: usize, cx: &App) -> Option<(usize, usize)> {
        let state = self.blocks[ix].state.read(cx);
        let (caret, line_height) = state.cursor_layout()?;
        let text_bounds = state.text_bounds()?;
        if line_height <= gpui_kit::px(0.) {
            return None;
        }
        let offset = (caret.origin.y - text_bounds.origin.y) / line_height;
        let row = offset.round().max(0.) as usize;
        let rows = (text_bounds.size.height / line_height).round().max(1.) as usize;
        Some((row, rows))
    }

    fn caret_at_first_row(&self, ix: usize, cx: &App) -> bool {
        match self.caret_row(ix, cx) {
            Some((row, _)) => row == 0,
            // Before the first layout, fall back to the buffer line.
            None => self.blocks[ix].state.read(cx).cursor_position().line == 0,
        }
    }

    fn caret_at_last_row(&self, ix: usize, cx: &App) -> bool {
        match self.caret_row(ix, cx) {
            Some((row, rows)) => row + 1 >= rows,
            None => true,
        }
    }

    fn caret(&self, ix: usize, cx: &App) -> (usize, usize, bool) {
        let state = self.blocks[ix].state.read(cx);
        let range = state.selected_range();
        (range.start, range.end, range.is_empty())
    }

    /// Attach every editor-level key handler to the root element.
    pub(crate) fn with_key_handlers<E: InteractiveElement>(
        &self,
        el: E,
        cx: &mut Context<Self>,
    ) -> E {
        el
            // ------------------------------------------------- structural keys
            .capture_action(cx.listener(Self::on_enter))
            .capture_action(cx.listener(Self::on_backspace))
            .capture_action(cx.listener(Self::on_delete))
            .capture_action(cx.listener(Self::on_move_up))
            .capture_action(cx.listener(Self::on_move_down))
            .capture_action(cx.listener(Self::on_move_left))
            .capture_action(cx.listener(Self::on_move_right))
            .capture_action(cx.listener(Self::on_tab))
            .capture_action(cx.listener(Self::on_shift_tab))
            .capture_action(cx.listener(Self::on_escape))
            // -------------------------------------------------------- commands
            .on_action(cx.listener(|this, _: &actions::ToggleBold, window, cx| {
                this.toggle_bold(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::ToggleItalic, window, cx| {
                this.toggle_italic(window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &actions::ToggleUnderline, window, cx| {
                    this.toggle_underline(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &actions::ToggleStrike, window, cx| {
                this.toggle_strike(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::ToggleCode, window, cx| {
                this.toggle_code(window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &actions::ToggleHighlight, window, cx| {
                    this.toggle_highlight(Some(HighlightColor::Yellow), window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &actions::ToggleSuperscript, window, cx| {
                    this.toggle_superscript(window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &actions::ToggleSubscript, window, cx| {
                    this.toggle_subscript(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &actions::ClearMarks, window, cx| {
                this.unset_all_marks(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::SetParagraph, window, cx| {
                this.set_paragraph(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::SetHeading1, window, cx| {
                this.toggle_heading(1, window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::SetHeading2, window, cx| {
                this.toggle_heading(2, window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::SetHeading3, window, cx| {
                this.toggle_heading(3, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &actions::ToggleBulletList, window, cx| {
                    this.toggle_bullet_list(window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &actions::ToggleOrderedList, window, cx| {
                    this.toggle_ordered_list(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &actions::ToggleTaskList, window, cx| {
                this.toggle_task_list(window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &actions::ToggleBlockquote, window, cx| {
                    this.toggle_blockquote(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &actions::ToggleCodeBlock, window, cx| {
                this.toggle_code_block(window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &actions::SetHorizontalRule, window, cx| {
                    this.set_horizontal_rule(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &actions::DuplicateBlock, window, cx| {
                this.duplicate_block(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::DeleteBlock, window, cx| {
                this.delete_active_block(window, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::MoveBlockUp, _window, cx| {
                this.move_block(-1, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::MoveBlockDown, _window, cx| {
                this.move_block(1, cx)
            }))
            .on_action(cx.listener(|this, _: &actions::OpenSlashMenu, window, cx| {
                this.open_slash_menu(window, cx)
            }))
    }

    // ------------------------------------------------------------- handlers

    fn on_enter(&mut self, action: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.confirm_slash_item(window, cx);
            cx.stop_propagation();
            return;
        }
        let Some(ix) = self.active_index() else { return };

        // Shift-Enter is a soft break: let the input insert the newline.
        if action.shift {
            return;
        }

        if self.spec_at(ix, cx).caps().multiline {
            // Three Enters in a row leave a code block, as Tiptap's
            // `exitOnTripleEnter` does.
            let (start, _, collapsed) = self.caret(ix, cx);
            let text = &self.blocks[ix].text;
            if collapsed && start == text.len() && text.ends_with("\n\n") {
                let trimmed = text[..text.len() - 2].to_string();
                self.set_block_text(ix, trimmed, None, window, cx);
                let next = self.ensure_paragraph_after(ix, window, cx);
                self.focus_block(next, Caret::Start, window, cx);
                cx.stop_propagation();
            }
            return;
        }

        self.split_block(window, cx);
        cx.stop_propagation();
    }

    fn on_backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else { return };
        let (start, _, collapsed) = self.caret(ix, cx);
        if !collapsed || start != 0 {
            return;
        }
        self.join_backward(window, cx);
        cx.stop_propagation();
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else { return };
        let (start, _, collapsed) = self.caret(ix, cx);
        if !collapsed || start != self.blocks[ix].text.len() {
            return;
        }
        self.join_forward(window, cx);
        cx.stop_propagation();
    }

    fn on_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.move_slash_selection(-1, cx);
            cx.stop_propagation();
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if !self.caret_at_first_row(ix, cx) {
            return;
        }
        if self.focus_sibling(ix, -1, Caret::End, window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.move_slash_selection(1, cx);
            cx.stop_propagation();
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if !self.caret_at_last_row(ix, cx) {
            return;
        }
        if self.focus_sibling(ix, 1, Caret::Start, window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else { return };
        let (start, _, collapsed) = self.caret(ix, cx);
        if !collapsed || start != 0 {
            return;
        }
        if self.focus_sibling(ix, -1, Caret::End, window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else { return };
        let (start, _, collapsed) = self.caret(ix, cx);
        if !collapsed || start != self.blocks[ix].text.len() {
            return;
        }
        if self.focus_sibling(ix, 1, Caret::Start, window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_tab(&mut self, _: &IndentInline, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.move_slash_selection(1, cx);
            cx.stop_propagation();
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if self.spec_at(ix, cx).caps().multiline {
            return;
        }
        if self.sink_list_item(window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_shift_tab(&mut self, _: &OutdentInline, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.move_slash_selection(-1, cx);
            cx.stop_propagation();
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if self.spec_at(ix, cx).caps().multiline {
            return;
        }
        if self.lift_list_item(window, cx) {
            cx.stop_propagation();
        }
    }

    fn on_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.slash_menu_is_open() {
            self.close_slash_menu(cx);
            cx.stop_propagation();
            return;
        }
        if !self.selected.is_empty() {
            self.selected.clear();
            cx.notify();
            return;
        }
        // Escape with a caret selects the block as a node, as Notion does.
        let Some(id) = self.focused else { return };
        self.selected = vec![id];
        self.focused = None;
        self.focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    /// Mark shortcuts the toolbar mirrors; used by tests.
    pub fn mark_for_action(action: &dyn gpui_kit::Action) -> Option<MarkKind> {
        if action.as_any().is::<actions::ToggleBold>() {
            return Some(MarkKind::Bold);
        }
        if action.as_any().is::<actions::ToggleItalic>() {
            return Some(MarkKind::Italic);
        }
        if action.as_any().is::<actions::ToggleUnderline>() {
            return Some(MarkKind::Underline);
        }
        if action.as_any().is::<actions::ToggleStrike>() {
            return Some(MarkKind::Strike);
        }
        if action.as_any().is::<actions::ToggleCode>() {
            return Some(MarkKind::Code);
        }
        None
    }

    /// The type name of the active block, for the "Turn into" label.
    pub fn active_type(&self) -> &str {
        self.active_index()
            .map(|ix| self.blocks[ix].ty.as_ref())
            .unwrap_or(types::PARAGRAPH)
    }
}
