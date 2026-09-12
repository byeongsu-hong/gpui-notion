//! The slash command menu.
//!
//! Typing `/` at the start of a word opens it; the characters after the slash
//! filter it; Enter runs the item and removes the query text. Items come from
//! the registered node types, so a new block type appears here for free.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Anchor, AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    Point, StatefulInteractiveElement as _, Styled as _, Window, deferred, div, px,
};

use super::block::{BlockId, BlockRegistry, SlashItem};
use super::ui;
use super::view::NotionEditor;

/// Priority of the editor's own overlays, above ordinary content.
const OVERLAY_PRIORITY: usize = 100;

/// An open slash menu.
pub struct SlashMenu {
    /// Block the query lives in.
    pub block: BlockId,
    /// Byte offset of the `/`.
    pub start: usize,
    pub query: String,
    pub selected: usize,
}

/// One entry of the filtered list, resolved against the registry.
pub struct ResolvedItem {
    pub title: &'static str,
    pub group: &'static str,
    pub icon: &'static str,
    pub run: fn(&mut NotionEditor, &mut Window, &mut Context<NotionEditor>),
}

impl NotionEditor {
    pub fn slash_menu_is_open(&self) -> bool {
        self.slash.is_some()
    }

    /// Open the menu at the caret, inserting the `/` if it is not there yet.
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
        self.slash = Some(SlashMenu {
            block: self.blocks[ix].id,
            start,
            query: String::new(),
            selected: 0,
        });
        cx.notify();
    }

    pub fn close_slash_menu(&mut self, cx: &mut Context<Self>) {
        if self.slash.take().is_some() {
            cx.notify();
        }
    }

    /// Called after every edit: open on a fresh `/`, or re-read the query.
    pub(crate) fn sync_slash_menu(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.active_index() else {
            self.slash = None;
            return;
        };
        let caret = self.blocks[ix].state.read(cx).cursor();
        let text = self.blocks[ix].text.clone();
        let id = self.blocks[ix].id;

        if let Some(menu) = &mut self.slash {
            if menu.block != id || caret <= menu.start || !text[menu.start..].starts_with('/') {
                self.slash = None;
                cx.notify();
                return;
            }
            let query = text[menu.start + 1..caret].to_string();
            // A space with no matches ends the menu, as the suggestion plugin does.
            if query != menu.query {
                menu.query = query;
                menu.selected = 0;
            }
            if self.slash_items(cx).is_empty() {
                self.slash = None;
            }
            cx.notify();
            return;
        }

        // Open when a `/` was just typed at the start of a word.
        if caret == 0 || !text[..caret].ends_with('/') {
            return;
        }
        if !self.spec_at(ix, cx).caps().input_rules {
            return;
        }
        let start = caret - 1;
        let preceding = text[..start].chars().next_back();
        if preceding.is_some_and(|c| !c.is_whitespace()) {
            return;
        }
        self.slash = Some(SlashMenu {
            block: id,
            start,
            query: String::new(),
            selected: 0,
        });
        cx.notify();
    }

    /// Items matching the current query, in registry order.
    pub fn slash_items(&self, cx: &App) -> Vec<ResolvedItem> {
        let query = self
            .slash
            .as_ref()
            .map(|m| m.query.to_lowercase())
            .unwrap_or_default();

        BlockRegistry::global(cx)
            .slash_items()
            .into_iter()
            .filter(|item| matches_query(item, &query))
            .map(|item| ResolvedItem {
                title: item.title,
                group: item.group,
                icon: item.icon,
                run: item.run,
            })
            .collect()
    }

    pub fn move_slash_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.slash_items(cx).len();
        let Some(menu) = &mut self.slash else { return };
        if count == 0 {
            return;
        }
        let next = (menu.selected as isize + delta).rem_euclid(count as isize);
        menu.selected = next as usize;
        cx.notify();
    }

    pub fn confirm_slash_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.slash_items(cx);
        let Some(menu) = self.slash.take() else { return };
        let Some(item) = items.get(menu.selected) else {
            cx.notify();
            return;
        };
        let run = item.run;

        // Remove the `/query` before running the command.
        if let Some(ix) = self.index_of(menu.block) {
            let caret = self.blocks[ix].state.read(cx).cursor();
            let end = caret.max(menu.start);
            self.edit_block_text(ix, menu.start..end, "", Some(menu.start), window, cx);
        }
        run(self, window, cx);
        cx.notify();
    }

    pub(crate) fn run_slash_item(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = &mut self.slash {
            menu.selected = index;
        }
        self.confirm_slash_item(window, cx);
    }

    /// The menu, anchored under the caret.
    pub(crate) fn render_slash_menu(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let menu = self.slash.as_ref()?;
        let ix = self.index_of(menu.block)?;
        let state = self.blocks[ix].state.read(cx);
        let (caret, line_height) = state.cursor_layout()?;
        let position = caret.origin + Point::new(px(0.), line_height + px(6.));

        let items = self.slash_items(cx);
        if items.is_empty() {
            return None;
        }

        let mut rows: Vec<AnyElement> = Vec::new();
        let mut group = "";
        for (index, item) in items.iter().enumerate() {
            if item.group != group {
                group = item.group;
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
            rows.push(
                ui::menu_row(selected, cx)
                    .id(("slash-item", index))
                    .child(ui::icon(
                        item.icon,
                        px(16.),
                        if selected {
                            cx.theme().accent_foreground
                        } else {
                            cx.theme().muted_foreground
                        },
                    ))
                    .child(item.title.to_string())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_slash_item(index, window, cx)
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
                            .id("slash-menu")
                            .overflow_y_scroll()
                            .children(rows),
                    ),
            )
            .with_priority(OVERLAY_PRIORITY)
            .into_any_element(),
        )
    }
}

fn matches_query(item: &SlashItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query = query.trim();
    item.title.to_lowercase().contains(query)
        || item
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
}
