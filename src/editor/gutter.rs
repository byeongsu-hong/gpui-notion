//! The left gutter: the `+` insert button and the drag handle, plus the
//! drag-to-reorder machinery they drive.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu};
use gpui_kit::component::{ActiveTheme, Sizable as _};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, DragMoveEvent, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div, px,
};


use super::block::BlockId;
use super::actions;
use super::block::BlockRegistry;
use super::ui::Lucide;
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
        let label = BlockRegistry::global(cx)
            .get(&block.ty)
            .label(&block.attrs);
        let focus = self.focus_handle_for_editor();
        // Centre the controls on the block's first line.
        let top = (layout.line_height_px() - px(24.)).max(px(0.)) / 2.;

        let mut controls = div()
            .absolute()
            .left(px(4.))
            .top(top)
            .h(px(24.))
            .flex()
            .items_center()
            .gap(px(2.));
        if !self.always_show_gutter {
            controls = controls
                .invisible()
                .group_hover(group_name(id), |this| this.visible());
        }

        controls
            .child(
                Button::new(("insert", id.0 as usize))
                    .ghost()
                    .xsmall()
                    .icon(Lucide("plus"))
                    .tooltip("Insert block")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.insert_block_below(id, window, cx)
                    })),
            )
            .child(
                draggable(
                    Button::new(("drag", id.0 as usize))
                        .ghost()
                        .xsmall()
                        .icon(Lucide("grip-vertical"))
                        .tooltip("Click for options, hold for drag")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_block_as_node(id, window, cx)
                        })),
                    id,
                    text,
                )
                .dropdown_menu(move |menu, window, cx| {
                    block_menu(id, label.clone(), focus.clone(), menu, window, cx)
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
        self.selected = vec![id];
        self.take_focus_from_blocks(window, cx);
        cx.notify();
    }

    /// Track where a dragged block would land.
    pub(crate) fn on_drag_over(
        &mut self,
        ix: usize,
        event: &DragMoveEvent<DraggedBlock>,
        cx: &mut Context<Self>,
    ) {
        // The listener fires on every block that has one, so the pointer
        // position decides which row is actually being dragged over.
        let bounds = event.bounds;
        let position = event.event.position;
        if !bounds.contains(&position) {
            return;
        }
        let below = position.y > bounds.center().y;
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

/// The block options menu, opened from the drag handle.
fn block_menu(
    id: BlockId,
    label: gpui_kit::SharedString,
    focus: gpui_kit::FocusHandle,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let _ = id;
    let turn_into_focus = focus.clone();
    let color_focus = focus.clone();
    menu.action_context(focus)
        .label(label)
        .menu("Comment", Box::new(actions::AddComment))
        .submenu("Color", window, cx, {
            let focus = color_focus.clone();
            move |menu, _, _| color_menu(focus.clone(), menu)
        })
        .submenu("Turn into", window, cx, {
            let focus = turn_into_focus.clone();
            move |menu, _, _| {
                menu.action_context(focus.clone())
                    .menu("Text", Box::new(actions::SetParagraph))
                    .menu("Heading 1", Box::new(actions::SetHeading1))
                    .menu("Heading 2", Box::new(actions::SetHeading2))
                    .menu("Heading 3", Box::new(actions::SetHeading3))
                    .menu("Bulleted list", Box::new(actions::ToggleBulletList))
                    .menu("Numbered list", Box::new(actions::ToggleOrderedList))
                    .menu("To-do list", Box::new(actions::ToggleTaskList))
                    .menu("Blockquote", Box::new(actions::ToggleBlockquote))
                    .menu("Code block", Box::new(actions::ToggleCodeBlock))
            }
        })
        .menu("Reset formatting", Box::new(actions::ClearMarks))
        .separator()
        .menu("Duplicate", Box::new(actions::DuplicateBlock))
        .menu("Copy to clipboard", Box::new(actions::CopyBlock))
        .separator()
        .menu("Move up", Box::new(actions::MoveBlockUp))
        .menu("Move down", Box::new(actions::MoveBlockDown))
        .separator()
        .menu("Delete", Box::new(actions::DeleteBlock))
}

/// Give the drag handle its drag, through the imperative `Interactivity` API:
/// the styled `Button` is interactive but not stateful-interactive, so the
/// builder-style `on_drag` is not available on it.
fn draggable(mut button: Button, id: BlockId, text: String) -> Button {
    button
        .interactivity()
        .on_drag(DraggedBlock { id }, move |_, _, _, cx| {
            cx.new(|_| DragPreview { text: text.clone() })
        });
    button
}

/// The text and highlight palettes, shared with the selection toolbar.
fn color_menu(focus: gpui_kit::FocusHandle, menu: PopupMenu) -> PopupMenu {
    use super::mark::{HighlightColor, TextColor};

    let mut menu = menu.action_context(focus).label("Text color");
    for color in TextColor::ALL {
        menu = menu.menu(color.label(), Box::new(actions::ApplyColor::Text(color)));
    }
    menu = menu.separator().label("Highlight color");
    for color in HighlightColor::ALL {
        menu = menu.menu(color.label(), Box::new(actions::ApplyColor::Highlight(color)));
    }
    menu
}
