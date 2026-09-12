//! Sizing an input to the text in it, by measurement rather than assumption.
//!
//! Every text area in this editor — a block, a table cell — is sized to its
//! own content, because the document, not the input, decides how tall things
//! are. That only works if the height handed to an input leaves room for the
//! text after the input has taken its own padding out of it, and that padding
//! is not a number an application can know: it depends on the component, on
//! the theme and on the display, whose device pixels the text area is snapped
//! to. A text area even a fraction short of its text scrolls inside itself as
//! the caret moves between rows, and the text visibly jumps.
//!
//! So the gap is measured. [`InputFit`] remembers what an input did with the
//! height it was given last frame and asks for more when it came up short.

use gpui_kit::component::input::EditorState;
use gpui_kit::{App, Entity, Pixels, px};

use super::style;

/// What one input keeps for itself, learned from what it did last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputFit {
    inset: Pixels,
}

impl Default for InputFit {
    fn default() -> Self {
        Self {
            inset: style::INPUT_PAD_Y * 2.,
        }
    }
}

impl InputFit {
    /// The height to hand an input that has to show `text` pixels of text.
    pub fn height(&self, text: Pixels) -> Pixels {
        text + self.inset
    }

    /// How much taller that is than the assumed padding — space that reads as
    /// a gap under the text and can come off a neighbouring margin.
    pub fn surplus(&self) -> Pixels {
        (self.inset - style::INPUT_PAD_Y * 2.).max(px(0.))
    }

    /// Learn from the text area an input ended up with.
    ///
    /// The inset only ever widens: text areas are snapped to whole device
    /// pixels, and following that snapping in both directions never settles.
    /// A text area with far more room than its text asks for means the block
    /// changed shape underneath, and starts the measurement again.
    pub fn observe(&mut self, text: Pixels, text_area: Pixels) {
        if text_area <= px(0.) {
            return;
        }
        if text_area < text {
            // A device pixel of slack on top of the shortfall, so the next
            // frame lands over the line rather than on it.
            let widened = self.inset + (text - text_area) + style::INPUT_INSET_SLACK;
            self.inset = widened.min(style::MAX_INPUT_INSET);
        } else if text_area - text > style::INPUT_INSET_SLACK * 4. {
            *self = Self::default();
        }
    }

    /// Whether an input's text area currently holds all of its text, which is
    /// the invariant that keeps a document still.
    pub fn fits(text: Pixels, text_area: Option<Pixels>) -> bool {
        match text_area {
            Some(area) => area + px(0.01) >= text,
            None => true,
        }
    }
}

/// Put an input back to the top when its text area holds all of its text.
///
/// An input scrolls itself to keep the caret in view. While text is being
/// typed a text area can be a row short for one frame — long enough for the
/// input to scroll — and nothing puts that scroll back once the area has
/// caught up. Leaving it there hides the first row of the block.
pub fn reset_scroll_when_text_fits(
    state: &Entity<EditorState>,
    text: Pixels,
    cx: &mut App,
) {
    let (area, offset) = {
        let state = state.read(cx);
        (
            state.text_bounds().map(|bounds| bounds.size.height),
            state.scroll_offset(),
        )
    };
    if offset.y == px(0.) || !InputFit::fits(text, area) {
        return;
    }
    state.update(cx, |state, cx| {
        state.set_scroll_offset(gpui_kit::point(offset.x, px(0.)), cx)
    });
}

/// Height of the text in a single-input text area, in pixels, as the input
/// itself lays it out: at least one row, more when it wraps.
pub fn text_height(state: &Entity<EditorState>, text: &str, cx: &App) -> Pixels {
    let state = state.read(cx);
    let line_height = state.line_height().unwrap_or(px(20.));
    state
        .range_to_bounds(&(0..text.len()))
        .map(|bounds| bounds.size.height)
        .unwrap_or(line_height)
        .max(line_height)
}
