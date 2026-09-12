//! The suggestion menus: `/` for blocks, `:` for emoji, `@` for mentions.
//!
//! One menu state serves all three. A trigger character typed at a word
//! boundary opens it, the characters after it filter, Enter commits and
//! Escape closes, leaving the typed text alone.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Anchor, AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    Point, SharedString, StatefulInteractiveElement as _, Styled as _, Window, deferred, div, px,
};

use super::block::{BlockId, BlockRegistry};
use super::mark::MarkKind;
use super::suggestion::{EMOJI, Emoji, Mention, Trigger, default_mentions, matches};
use super::toolbar::OVERLAY_PRIORITY;
use super::ui;
use super::view::NotionEditor;

/// An open suggestion menu.
pub struct SuggestionMenu {
    pub trigger: Trigger,
    /// Block the query lives in.
    pub block: BlockId,
    /// Byte offset of the trigger character.
    pub start: usize,
    pub query: String,
    pub selected: usize,
}

/// One row of a menu, whatever opened it.
pub enum SuggestionItem {
    Block {
        title: &'static str,
        group: &'static str,
        icon: &'static str,
        run: fn(&mut NotionEditor, &mut Window, &mut Context<NotionEditor>),
    },
    Emoji(&'static Emoji),
    Mention(Mention),
}

impl SuggestionItem {
    pub fn title(&self) -> SharedString {
        match self {
            Self::Block { title, .. } => SharedString::from(*title),
            Self::Emoji(emoji) => SharedString::from(emoji.name),
            Self::Mention(mention) => mention.name.clone(),
        }
    }

    pub fn group(&self) -> &'static str {
        match self {
            Self::Block { group, .. } => group,
            Self::Emoji(_) => "Emoji",
            Self::Mention(_) => "People",
        }
    }
}

impl NotionEditor {
    pub fn suggestion_is_open(&self) -> bool {
        self.suggestion.is_some()
    }

    /// Kept for the `/` affordances: the gutter `+` and `Mod+/`.
    pub fn slash_menu_is_open(&self) -> bool {
        self.suggestion
            .as_ref()
            .is_some_and(|menu| menu.trigger == Trigger::Slash)
    }

    /// Open the block menu at the caret, inserting the `/` if needed.
    pub fn open_slash_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else { return };
        if !self.spec_at(ix, cx).caps().input_rules {
            return;
        }
        let caret = self.blocks[ix].state.read(cx).cursor();
        let already_slash = self.blocks[ix].text[..caret].ends_with('/');
        if !already_slash {
            self.edit_block_text(ix, caret..caret, "/", Some(caret + 1), window, cx);
        }
        let start = if already_slash { caret - 1 } else { caret };
        self.suggestion = Some(SuggestionMenu {
            trigger: Trigger::Slash,
            block: self.blocks[ix].id,
            start,
            query: String::new(),
            selected: 0,
        });
        cx.notify();
    }

    pub fn close_suggestion_menu(&mut self, cx: &mut Context<Self>) {
        if self.suggestion.take().is_some() {
            cx.notify();
        }
    }

    /// Called after every edit: open on a fresh trigger, or re-read the query.
    pub(crate) fn sync_suggestion_menu(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else {
            self.suggestion = None;
            return;
        };
        let caret = self.blocks[ix].state.read(cx).cursor();
        let text = self.blocks[ix].text.clone();
        let id = self.blocks[ix].id;

        if let Some(menu) = &mut self.suggestion {
            let trigger = menu.trigger.character();
            let still_open = menu.block == id
                && caret > menu.start
                && text[menu.start..].starts_with(trigger)
                && menu.start < text.len();
            if !still_open {
                self.suggestion = None;
                cx.notify();
                return;
            }
            let query = text[menu.start + trigger.len_utf8()..caret].to_string();
            if query != menu.query {
                menu.query = query;
                menu.selected = 0;
            }
            // A query that matches nothing ends the menu, as the suggestion
            // plugin does, so ordinary prose is not held hostage by a `:`.
            if self.suggestion_items(cx).is_empty() {
                self.suggestion = None;
            }
            cx.notify();
            return;
        }

        if caret == 0 || !self.spec_at(ix, cx).caps().input_rules {
            return;
        }
        let Some(character) = text[..caret].chars().next_back() else {
            return;
        };
        let Some(trigger) = Trigger::from_character(character) else {
            return;
        };
        let start = caret - character.len_utf8();
        let preceding = text[..start].chars().next_back();
        if preceding.is_some_and(|previous| !previous.is_whitespace()) {
            return;
        }

        self.suggestion = Some(SuggestionMenu {
            trigger,
            block: id,
            start,
            query: String::new(),
            selected: 0,
        });
        if !trigger.opens_empty() {
            // The emoji menu waits for a shortcode before showing anything.
            cx.notify();
            return;
        }
        cx.notify();
    }

    /// Items matching the current query, in registry order.
    pub fn suggestion_items(&self, cx: &App) -> Vec<SuggestionItem> {
        let Some(menu) = self.suggestion.as_ref() else {
            return Vec::new();
        };
        let query = menu.query.clone();

        match menu.trigger {
            Trigger::Slash => BlockRegistry::global(cx)
                .slash_items()
                .into_iter()
                .filter(|item| matches(&query, item.title, item.keywords))
                .map(|item| SuggestionItem::Block {
                    title: item.title,
                    group: item.group,
                    icon: item.icon,
                    run: item.run,
                })
                .collect(),
            Trigger::Emoji => {
                if query.is_empty() {
                    return Vec::new();
                }
                EMOJI
                    .iter()
                    .filter(|emoji| matches(&query, emoji.name, emoji.keywords))
                    .map(SuggestionItem::Emoji)
                    .collect()
            }
            Trigger::Mention => self
                .mentions
                .iter()
                .filter(|mention| matches(&query, &mention.name, &[]))
                .cloned()
                .map(SuggestionItem::Mention)
                .collect(),
        }
    }

    /// Kept for the tests and hosts that speak in terms of the slash menu.
    pub fn slash_items(&self, cx: &App) -> Vec<SuggestionItem> {
        self.suggestion_items(cx)
    }

    /// The people offered by `@`; an application sets its own list.
    pub fn set_mentions(&mut self, mentions: Vec<Mention>, cx: &mut Context<Self>) {
        self.mentions = mentions;
        cx.notify();
    }

    pub fn move_suggestion_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.suggestion_items(cx).len();
        let Some(menu) = &mut self.suggestion else {
            return;
        };
        if count == 0 {
            return;
        }
        menu.selected = (menu.selected as isize + delta).rem_euclid(count as isize) as usize;
        cx.notify();
    }

    pub fn confirm_suggestion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.suggestion_items(cx);
        let Some(menu) = self.suggestion.take() else {
            return;
        };
        let Some(item) = items.into_iter().nth(menu.selected) else {
            cx.notify();
            return;
        };
        let Some(ix) = self.index_of(menu.block) else {
            return;
        };
        let caret = self.blocks[ix].state.read(cx).cursor();
        let end = caret.max(menu.start);

        match item {
            SuggestionItem::Block { run, .. } => {
                self.edit_block_text(ix, menu.start..end, "", Some(menu.start), window, cx);
                run(self, window, cx);
            }
            SuggestionItem::Emoji(emoji) => {
                let caret_after = menu.start + emoji.character.len();
                self.edit_block_text(
                    ix,
                    menu.start..end,
                    emoji.character,
                    Some(caret_after),
                    window,
                    cx,
                );
            }
            SuggestionItem::Mention(mention) => {
                let text = format!("@{} ", mention.name);
                let caret_after = menu.start + text.len();
                let marked = menu.start..menu.start + text.trim_end().len();
                self.edit_block_text(ix, menu.start..end, &text, Some(caret_after), window, cx);
                if let Some(block) = self.blocks.get_mut(ix) {
                    block.marks.add(mention.mark(), marked);
                    block.stored_marks = Some(Vec::new());
                }
                self.apply_decorations(menu.block, cx);
            }
        }
        cx.notify();
    }

    pub(crate) fn run_suggestion_item(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = &mut self.suggestion {
            menu.selected = index;
        }
        self.confirm_suggestion(window, cx);
    }

    /// The menu, anchored under the caret.
    pub(crate) fn render_suggestion_menu(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let menu = self.suggestion.as_ref()?;
        let ix = self.index_of(menu.block)?;
        let state = self.blocks[ix].state.read(cx);
        let (caret, line_height) = state.cursor_layout()?;
        let position = caret.origin + Point::new(px(0.), line_height + px(6.));

        let items = self.suggestion_items(cx);
        if items.is_empty() {
            return None;
        }

        let mut rows: Vec<AnyElement> = Vec::new();
        let mut group = "";
        for (index, item) in items.iter().enumerate() {
            if item.group() != group {
                group = item.group();
                rows.push(
                    div()
                        .px(px(8.))
                        .pt(px(8.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .text_color(cx.theme().muted_foreground)
                        .child(group.to_string())
                        .into_any_element(),
                );
            }

            let selected = index == menu.selected;
            let leading = match item {
                SuggestionItem::Block { icon, .. } => ui::icon(
                    icon,
                    px(16.),
                    if selected {
                        cx.theme().accent_foreground
                    } else {
                        cx.theme().muted_foreground
                    },
                )
                .into_any_element(),
                SuggestionItem::Emoji(emoji) => div()
                    .w(px(16.))
                    .child(emoji.character.to_string())
                    .into_any_element(),
                SuggestionItem::Mention(mention) => div()
                    .w(px(16.))
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        mention
                            .name
                            .chars()
                            .next()
                            .map(|initial| initial.to_string())
                            .unwrap_or_default(),
                    )
                    .into_any_element(),
            };

            rows.push(
                ui::menu_row(selected, cx)
                    .id(("suggestion", index))
                    .child(leading)
                    .child(item.title())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_suggestion_item(index, window, cx)
                    }))
                    .into_any_element(),
            );
        }

        Some(
            deferred(
                gpui_kit::base::Positioner::corner(Anchor::TopLeft, position)
                    .margin(px(8.))
                    .occlude()
                    .child(
                        ui::popover_surface(cx)
                            .w(px(320.))
                            .max_h(px(360.))
                            .id("suggestion-menu")
                            .overflow_y_scroll()
                            .children(rows),
                    ),
            )
            .with_priority(OVERLAY_PRIORITY)
            .into_any_element(),
        )
    }
}

/// Mentions are styled like a soft chip; the mark carries the person's id.
pub fn mention_mark_id(kind: &MarkKind) -> Option<SharedString> {
    match kind {
        MarkKind::Mention(id) => Some(id.clone()),
        _ => None,
    }
}

/// Default people list, exposed so an application can start from it.
pub fn default_people() -> Vec<Mention> {
    default_mentions()
}
