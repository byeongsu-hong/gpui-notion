//! Selection that spans blocks, and what can be done to it.
//!
//! Inside a block the input owns the selection. Once a selection reaches past
//! a block's edge the editor takes over and selects whole blocks, which is
//! what Notion does and what `Mod+A` escalates to.

use gpui_kit::base::actions::{SelectDown, SelectUp};
use gpui_kit::component::input::{Copy, Cut, SelectAll};
use gpui_kit::{ClipboardItem, Context, Window};

use super::block::{BlockContent, BlockId, BlockRegistry, types};
use super::history::Step;
use super::view::{Caret, NotionEditor};

impl NotionEditor {
    /// Blocks currently selected as nodes, in document order.
    pub fn selected_blocks(&self) -> Vec<BlockId> {
        let mut ids: Vec<BlockId> = self
            .blocks
            .iter()
            .filter(|block| self.selected.contains(&block.id))
            .map(|block| block.id)
            .collect();
        ids.dedup();
        ids
    }

    pub fn has_block_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Select every block between `from` and `to`, inclusive.
    pub fn select_block_range(
        &mut self,
        from: usize,
        to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (low, high) = if from <= to { (from, to) } else { (to, from) };
        self.selected = self.blocks[low..=high.min(self.blocks.len() - 1)]
            .iter()
            .map(|block| block.id)
            .collect();
        self.take_focus_from_blocks(window, cx);
        cx.notify();
    }

    pub fn select_all_blocks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = self.blocks.iter().map(|block| block.id).collect();
        self.take_focus_from_blocks(window, cx);
        cx.notify();
    }

    /// Move keyboard focus to the editor itself, so a block selection is not
    /// undone by the focused input reclaiming it on the next frame.
    pub(crate) fn take_focus_from_blocks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = None;
        let handle = self.focus_handle_for_editor();
        handle.focus(window, cx);
    }

    pub fn clear_block_selection(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        self.selected.clear();
        cx.notify();
    }

    /// Extend the block selection by one block in `delta`'s direction.
    fn extend_block_selection(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let anchor = self
            .selected
            .first()
            .and_then(|id| self.index_of(*id))
            .or_else(|| self.active_index());
        let Some(anchor) = anchor else { return false };

        let last = self
            .selected
            .last()
            .and_then(|id| self.index_of(*id))
            .unwrap_or(anchor);
        let next = last as isize + delta;
        if next < 0 || next as usize >= self.blocks.len() {
            return false;
        }
        self.select_block_range(anchor, next as usize, window, cx);
        true
    }

    /// Delete every selected block, leaving a paragraph if none remain.
    pub fn delete_selected_blocks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        self.record(Step::Structural, cx);
        let first = self
            .selected_blocks()
            .first()
            .and_then(|id| self.index_of(*id))
            .unwrap_or(0);

        for id in self.selected_blocks() {
            self.remove_block(id, cx);
        }
        self.selected.clear();

        if self.blocks.is_empty() {
            let id = self.insert_block(0, BlockContent::paragraph(""), window, cx);
            self.focus_block(id, Caret::Start, window, cx);
            return;
        }
        let target = first.min(self.blocks.len() - 1);
        let id = self.blocks[target].id;
        self.focus_block(id, Caret::Start, window, cx);
    }

    /// The selected blocks as markdown, which is what the clipboard gets.
    pub fn selected_markdown(&self, cx: &gpui_kit::App) -> String {
        let registry = BlockRegistry::global(cx);
        self.blocks
            .iter()
            .filter(|block| self.selected.contains(&block.id))
            .map(|block| {
                let indent = "    ".repeat(block.indent);
                let prefix = match block.ty.as_ref() {
                    types::HEADING => "#".repeat(block.attrs.level.max(1) as usize) + " ",
                    types::BULLET_LIST => "- ".to_string(),
                    types::ORDERED_LIST => "1. ".to_string(),
                    types::TASK_LIST => {
                        if block.attrs.checked {
                            "- [x] ".to_string()
                        } else {
                            "- [ ] ".to_string()
                        }
                    }
                    types::BLOCKQUOTE => "> ".to_string(),
                    types::HORIZONTAL_RULE => return "---".to_string(),
                    types::CODE_BLOCK => {
                        let language = block.attrs.language.clone().unwrap_or_default();
                        return format!("```{language}\n{}\n```", block.text);
                    }
                    _ => String::new(),
                };
                let _ = registry.get(&block.ty);
                format!("{indent}{prefix}{}", block.text)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn copy_selected_blocks(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        let markdown = self.selected_markdown(cx);
        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
    }

    // ------------------------------------------------------------- handlers

    pub(crate) fn on_select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_block_selection() {
            if self.extend_block_selection(-1, window, cx) {
                cx.stop_propagation();
            }
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if !self.caret_on_first_row(ix, cx) || ix == 0 {
            return;
        }
        self.select_block_range(ix, ix - 1, window, cx);
        cx.stop_propagation();
    }

    pub(crate) fn on_select_down(
        &mut self,
        _: &SelectDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_block_selection() {
            if self.extend_block_selection(1, window, cx) {
                cx.stop_propagation();
            }
            return;
        }
        let Some(ix) = self.active_index() else { return };
        if !self.caret_on_last_row(ix, cx) || ix + 1 >= self.blocks.len() {
            return;
        }
        self.select_block_range(ix, ix + 1, window, cx);
        cx.stop_propagation();
    }

    /// `Mod+A` selects the block's text first, the document second.
    pub(crate) fn on_select_all(
        &mut self,
        _: &SelectAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.active_index() else { return };
        let selection = self.blocks[ix].state.read(cx).selected_range();
        let whole_block = !self.blocks[ix].text.is_empty()
            && selection.start == 0
            && selection.end == self.blocks[ix].text.len();

        if whole_block || self.has_block_selection() || self.blocks[ix].text.is_empty() {
            self.select_all_blocks(window, cx);
            cx.stop_propagation();
        }
    }

    pub(crate) fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_block_selection() {
            return;
        }
        self.copy_selected_blocks(cx);
        cx.stop_propagation();
    }

    pub(crate) fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_block_selection() {
            return;
        }
        self.copy_selected_blocks(cx);
        self.delete_selected_blocks(window, cx);
        cx.stop_propagation();
    }
}
