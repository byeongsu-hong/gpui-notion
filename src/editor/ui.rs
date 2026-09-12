//! Shared presentation helpers: icons, popover surfaces, toolbar buttons.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, InteractiveElement as _, IntoElement, SharedString, Styled as _,
    div, px,
};

/// An icon from the bundled Lucide set, named the way the Tiptap template
/// names them (`heading-1`, `list-ordered`, …).
#[derive(Clone, Copy)]
pub struct Lucide(pub &'static str);

impl gpui_kit::assets::IconNamed for Lucide {
    fn path(self) -> SharedString {
        format!("icons/{}.svg", self.0).into()
    }
}

pub fn icon(name: &'static str, size: gpui_kit::Pixels, color: gpui_kit::Hsla) -> impl IntoElement {
    gpui_kit::svg()
        .path(Lucide(name).path_string())
        .size(size)
        .flex_none()
        .text_color(color)
}

impl Lucide {
    fn path_string(self) -> SharedString {
        format!("icons/{}.svg", self.0).into()
    }
}

/// The popover surface shared by the slash menu, toolbar and link editor.
pub fn popover_surface(cx: &App) -> Div {
    div()
        .bg(cx.theme().popover)
        .text_color(cx.theme().popover_foreground)
        .border_1()
        .border_color(cx.theme().border)
        // The theme owns the corner, so this surface matches the menus and
        // popovers gpui-kit draws beside it.
        .rounded(cx.theme().radius)
        .shadow_lg()
        .p(px(4.))
}

/// A row in a menu: fixed height, hover fill, rounded.
pub fn menu_row(selected: bool, cx: &App) -> Div {
    let row = div()
        .h(px(32.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(px(6.))
        .cursor_pointer()
        .text_size(px(14.));
    if selected {
        row.bg(cx.theme().accent).text_color(cx.theme().accent_foreground)
    } else {
        row.hover(|this| this.bg(cx.theme().accent.opacity(0.6)))
    }
}

/// A toolbar button: 2rem square, ghost by default, filled when active.
pub fn toolbar_button(id: impl Into<gpui_kit::ElementId>, active: bool, cx: &App) -> gpui_kit::Stateful<Div> {
    let button = div()
        .id(id)
        .size(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer();
    if active {
        button
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
    } else {
        button.hover(|this| this.bg(cx.theme().accent.opacity(0.6)))
    }
}
