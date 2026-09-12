//! Actions and key bindings, named after the Tiptap commands they run.

use gpui_kit::{Action, App, KeyBinding, actions};
use serde::Deserialize;

use super::mark::{HighlightColor, TextColor};

/// Applying one of the palette colors, dispatched by the color menus.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Action)]
#[action(namespace = notion, no_json)]
pub enum ApplyColor {
    Text(TextColor),
    Highlight(HighlightColor),
}

actions!(
    notion,
    [
        ToggleBold,
        ToggleItalic,
        ToggleUnderline,
        ToggleStrike,
        ToggleCode,
        ToggleHighlight,
        ToggleSuperscript,
        ToggleSubscript,
        SetLink,
        ClearMarks,
        SetParagraph,
        SetHeading1,
        SetHeading2,
        SetHeading3,
        ToggleBulletList,
        ToggleOrderedList,
        ToggleTaskList,
        ToggleBlockquote,
        ToggleCodeBlock,
        SetHorizontalRule,
        OpenSlashMenu,
        DuplicateBlock,
        DeleteBlock,
        MoveBlockUp,
        MoveBlockDown,
        SelectBlock,
        CopyBlock,
        Cancel,
    ]
);

/// Key context of the editor surface; bindings below resolve inside it.
pub const CONTEXT: &str = "NotionEditor";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-b", ToggleBold, Some(CONTEXT)),
        KeyBinding::new("secondary-i", ToggleItalic, Some(CONTEXT)),
        KeyBinding::new("secondary-u", ToggleUnderline, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-s", ToggleStrike, Some(CONTEXT)),
        KeyBinding::new("secondary-e", ToggleCode, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-h", ToggleHighlight, Some(CONTEXT)),
        KeyBinding::new("secondary-.", ToggleSuperscript, Some(CONTEXT)),
        KeyBinding::new("secondary-,", ToggleSubscript, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-k", SetLink, Some(CONTEXT)),
        KeyBinding::new("secondary-/", OpenSlashMenu, Some(CONTEXT)),
        KeyBinding::new("secondary-r", ClearMarks, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-0", SetParagraph, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-1", SetHeading1, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-2", SetHeading2, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-3", SetHeading3, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-8", ToggleBulletList, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-7", ToggleOrderedList, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-9", ToggleTaskList, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-b", ToggleBlockquote, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-c", ToggleCodeBlock, Some(CONTEXT)),
        KeyBinding::new("secondary-d", DuplicateBlock, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-backspace", DeleteBlock, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-up", MoveBlockUp, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-down", MoveBlockDown, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        // The inputs bind redo per platform; the editor accepts both spellings.
        KeyBinding::new(
            "secondary-shift-z",
            gpui_kit::component::input::Redo,
            Some(CONTEXT),
        ),
        KeyBinding::new("secondary-y", gpui_kit::component::input::Redo, Some(CONTEXT)),
    ]);
}
