//! Typography and geometry of the page, matching the Notion-like editor.
//!
//! Every value is in logical pixels at the editor's base size. Colors are read
//! from the active `gpui-kit` theme so light and dark both work.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Hsla, Pixels, px};

/// Max width of the content column, `minmax(auto, 708px)` in the template.
pub const PAGE_WIDTH: Pixels = px(708.);
/// Padding inside the column: `3rem 3rem 30vh`.
pub const PAGE_PADDING: Pixels = px(48.);
/// Space either side of the column, where the drag handle and `+` live.
pub const GUTTER_WIDTH: Pixels = px(96.);
/// Width taken by the `+` and drag handle pair.
pub const GUTTER_CONTROLS_WIDTH: Pixels = px(52.);
/// Empty space under the last block, clickable to append a paragraph.
pub const PAGE_BOTTOM: Pixels = px(280.);

/// Base body size; every other size is derived from it.
pub const TEXT_SIZE: Pixels = px(16.);
/// Paragraph line height.
pub const LINE_HEIGHT: f32 = 1.6;
/// Gap above a paragraph that follows another block.
pub const BLOCK_GAP: Pixels = px(20.);

/// Indent per nesting level of a list.
pub const INDENT_WIDTH: Pixels = px(24.);
/// Width reserved for a bullet, number or checkbox.
pub const MARKER_WIDTH: Pixels = px(24.);

/// Horizontal padding `Input` gives a multi-line editor at the default size
/// (`Size::Medium::input_px`). Blocks cancel it so text lines up with markers.
pub const INPUT_PAD_X: Pixels = px(10.);
/// Vertical counterpart (`Size::Medium::input_py`).
pub const INPUT_PAD_Y: Pixels = px(8.);

/// Radius of popovers (slash menu, bubble toolbar, link editor).
pub const POPOVER_RADIUS: Pixels = px(8.);

/// Text color of a block, and of inline text colors.
pub fn text_color(cx: &App) -> Hsla {
    cx.theme().foreground
}

pub fn muted(cx: &App) -> Hsla {
    cx.theme().muted_foreground
}

pub fn border(cx: &App) -> Hsla {
    cx.theme().border
}

/// Fill behind an inline `code` mark.
pub fn code_background(cx: &App) -> Hsla {
    let mut color = cx.theme().muted;
    color.a = if cx.theme().mode.is_dark() { 0.55 } else { 0.9 };
    color
}

/// Foreground of an inline `code` mark — Notion's red on a light surface.
pub fn code_foreground(cx: &App) -> Hsla {
    if cx.theme().mode.is_dark() {
        gpui_kit::rgb(0xff8a80).into()
    } else {
        gpui_kit::rgb(0xeb5757).into()
    }
}

/// Link color of the `link` mark.
pub fn link_color(cx: &App) -> Hsla {
    cx.theme().link
}

/// Fill of a `highlight` mark.
pub fn highlight_fill(color: Option<super::mark::HighlightColor>, cx: &App) -> Hsla {
    use super::mark::HighlightColor::*;
    let dark = cx.theme().mode.is_dark();
    let rgb = match color.unwrap_or(Yellow) {
        Yellow if dark => 0x5c4d1a,
        Yellow => 0xfef3c0,
        Green if dark => 0x1f4a33,
        Green => 0xd3f0e0,
        Blue if dark => 0x1e3a5c,
        Blue => 0xd3e5ef,
        Purple if dark => 0x3f2b56,
        Purple => 0xe8deee,
        Pink if dark => 0x522038,
        Pink => 0xf5e0e9,
        Red if dark => 0x5c2323,
        Red => 0xfbe4e4,
        Gray if dark => 0x3a3a3a,
        Gray => 0xe3e2e0,
    };
    gpui_kit::rgb(rgb).into()
}

/// Foreground of a `textStyle` color mark; `None` keeps the block color.
pub fn text_color_value(color: super::mark::TextColor, cx: &App) -> Option<Hsla> {
    use super::mark::TextColor::*;
    let dark = cx.theme().mode.is_dark();
    let rgb = match color {
        Default => return None,
        Gray if dark => 0x9b9a97,
        Gray => 0x787774,
        Brown if dark => 0xba856f,
        Brown => 0x9f6b53,
        Orange if dark => 0xc77d48,
        Orange => 0xd9730d,
        Yellow if dark => 0xca9849,
        Yellow => 0xcb912f,
        Green if dark => 0x529e72,
        Green => 0x448361,
        Blue if dark => 0x5e87c9,
        Blue => 0x337ea9,
        Purple if dark => 0x9d68d3,
        Purple => 0x9065b0,
        Pink if dark => 0xd15796,
        Pink => 0xc14c8a,
        Red if dark => 0xdf5452,
        Red => 0xd44c47,
    };
    Some(gpui_kit::rgb(rgb).into())
}
