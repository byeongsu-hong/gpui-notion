//! The left gutter: the `+` insert button and the drag handle, plus the
//! drag-to-reorder machinery they drive.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, DragMoveEvent, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use super::block::BlockId;
use super::style;
use super::ui;
use super::view::{Caret, NotionEditor, group_name};

/// The payload carried while dragging a block.
#[derive(Clone, Debug)]
pub struct DraggedBlock {
    pub id: BlockId,
}

/// What the pointer carries during a drag.
pub struct DragPreview {
    pub text: String,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.))
            .py(px(4.))
            .rounded(px(6.))
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .text_size(px(14.))
            .text_color(cx.theme().foreground)
            .shadow_md()
            .child(if self.text.is_empty() {
                "Empty block".to_string()
            } else {
                self.text.chars().take(48).collect::<String>()
            })
    }
}

/// Where a dragged block would land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropTarget {
    pub index: usize,
    pub below: bool,
}

impl NotionEditor {
    /// The `+` and drag handle, shown while the pointer is over the block.
    pub(crate) fn render_gutter(
        &self,
        ix: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let block = &self.blocks[ix];
        let id = block.id;
        let text = block.text.clone();
        let layout = self.layout_at(ix, cx);
        // Centre the controls on the block's first line.
        let top = (layout.line_height_px() - px(24.)).max(px(0.)) / 2.;

        div()
            .absolute()
            .left(-style::GUTTER_CONTROLS_WIDTH)
            .top(top)
            .h(px(24.))
            .flex()
            .items_center()
            .gap(px(2.))
            .invisible()
            .group_hover(group_name(id), |this| this.visible())
            .child(
                ui::toolbar_button(("insert", id.0 as usize), false, cx)
                    .size(px(24.))
                    .child(ui::icon("plus", px(16.), cx.theme().muted_foreground))
                    .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Insert block").build(window, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.insert_block_below(id, window, cx)
                    })),
            )
            .child(
                ui::toolbar_button(("grip", id.0 as usize), false, cx)
                    .size(px(24.))
                    .child(ui::icon(
                        "grip-vertical",
                        px(16.),
                        cx.theme().muted_foreground,
                    ))
                    .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Click for options, hold for drag").build(window, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_block_as_node(id, window, cx)
                    }))
                    .on_drag(DraggedBlock { id }, move |_, _, _, cx| {
                        cx.new(|_| DragPreview { text: text.clone() })
                    }),
            )
            .into_any_element()
    }

    /// Insert an empty paragraph under `id` and put the caret in it, the way
    /// the template's `+` button does.
    pub fn insert_block_below(&mut self, id: BlockId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(id) else { return };
        let new_id = self.insert_block(ix + 1, super::block::BlockContent::paragraph(""), window, cx);
        self.focus_block(new_id, Caret::Start, window, cx);
        self.open_slash_menu(window, cx);
    }

    /// Select a block as a node, as clicking the drag handle does.
    pub fn select_block_as_node(
        &mut self,
        id: BlockId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focused = None;
        self.selected = vec![id];
        self.focus_handle_for_editor().focus(window, cx);
        cx.notify();
    }

    /// Track where a dragged block would land.
    pub(crate) fn on_drag_over(
        &mut self,
        ix: usize,
        event: &DragMoveEvent<DraggedBlock>,
        cx: &mut Context<Self>,
    ) {
        let bounds = event.bounds;
        let below = event.event.position.y > bounds.center().y;
        let target = DropTarget { index: ix, below };
        if self.drop_target != Some(target) {
            self.drop_target = Some(target);
            cx.notify();
        }
    }

    /// Complete a drag, moving the block to the tracked position.
    pub(crate) fn on_drop_block(&mut self, dragged: &DraggedBlock, cx: &mut Context<Self>) {
        let Some(target) = self.drop_target.take() else {
            return;
        };
        let Some(from) = self.index_of(dragged.id) else {
            return;
        };
        let to = if target.below {
            target.index + 1
        } else {
            target.index
        };
        self.reorder_block(from, to, cx);
        cx.notify();
    }

    pub(crate) fn drop_indicator(&self, ix: usize, cx: &App) -> Option<impl IntoElement> {
        let target = self.drop_target?;
        if target.index != ix {
            return None;
        }
        Some(
            div()
                .absolute()
                .left(px(0.))
                .right(px(0.))
                .when_else(
                    target.below,
                    |this| this.bottom(px(-1.)),
                    |this| this.top(px(-1.)),
                )
                .h(px(2.))
                .rounded(px(1.))
                .bg(cx.theme().primary),
        )
    }
}

/// `FluentBuilder::when` with both branches, kept local to this module.
trait WhenElse: Sized {
    fn when_else(self, condition: bool, then: impl FnOnce(Self) -> Self, other: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { other(self) }
    }
}

impl<T: Sized> WhenElse for T {}
