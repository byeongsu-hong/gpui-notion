//! Comment threads anchored to a range of text, the way Notion's are.
//!
//! A thread is a list of comments plus the quote it was started from. The
//! anchor lives in the document as a [`MarkKind::Comment`] over the commented
//! range, so it travels with edits like every other mark; the thread itself is
//! kept on the editor, because it is discussion, not text.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Disableable as _, Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, Context, Entity, Focusable as _,
    IntoElement, ParentElement as _, Point, SharedString, Styled as _,
    Subscription, Window, deferred, div, px,
};

use super::block::BlockId;
use super::mark::MarkKind;
use super::style;
use super::toolbar::OVERLAY_PRIORITY;
use super::ui;
use super::view::{DocumentChanged, NotionEditor};

/// Identity of a thread, stable across edits because the mark carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadId(pub u64);

/// One message in a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    author: SharedString,
    body: SharedString,
}

impl Comment {
    pub fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
        }
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

/// A discussion hanging off a range of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    id: ThreadId,
    block: BlockId,
    quote: SharedString,
    comments: Vec<Comment>,
    resolved: bool,
}

impl Thread {
    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn block(&self) -> BlockId {
        self.block
    }

    /// The text the thread was started from, shown at the top of the popover.
    pub fn quote(&self) -> &str {
        &self.quote
    }

    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
}

/// The comment being written, and the thread it will land in.
pub struct CommentDraft {
    thread: ThreadId,
    input: Entity<InputState>,
    _subscription: Subscription,
}

/// Who new comments are attributed to until a host says otherwise.
const DEFAULT_AUTHOR: &str = "You";

impl NotionEditor {
    /// Every thread in the document, oldest first.
    pub fn comment_threads(&self) -> &[Thread] {
        &self.comments
    }

    /// Threads that are still open, which is what a sidebar would list.
    pub fn open_comment_threads(&self) -> impl Iterator<Item = &Thread> {
        self.comments.iter().filter(|thread| !thread.resolved)
    }

    pub fn comment_thread(&self, id: ThreadId) -> Option<&Thread> {
        self.comments.iter().find(|thread| thread.id == id)
    }

    /// The thread whose popover is on screen.
    pub fn open_thread(&self) -> Option<ThreadId> {
        self.open_thread
    }

    /// Whether a comment is being written right now, which keeps the document
    /// keymap's hands off the keyboard.
    pub fn comment_draft_is_open(&self) -> bool {
        self.comment_draft.is_some()
    }

    /// Start a thread on the selection — the toolbar's Comment button and
    /// `Mod+Shift+M`.
    pub fn add_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((block, range)) = self.comment_target(cx) else {
            return;
        };
        if range.is_empty() {
            return;
        }
        let Some(ix) = self.index_of(block) else { return };

        let id = ThreadId(self.next_thread_id);
        self.next_thread_id += 1;
        let quote: String = self.blocks[ix].text[range.clone()].to_string();
        self.comments.push(Thread {
            id,
            block,
            quote: quote.into(),
            comments: Vec::new(),
            resolved: false,
        });

        self.blocks[ix].marks.add(MarkKind::Comment(id), range);
        self.apply_decorations(block, cx);
        self.open_comment_draft(id, window, cx);
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// The range a new thread would cover: the selected text, or the whole
    /// text of the first selected block.
    fn comment_target(&self, cx: &App) -> Option<(BlockId, std::ops::Range<usize>)> {
        if self.has_block_selection() {
            let id = self.selected_blocks().first().copied()?;
            let ix = self.index_of(id)?;
            return Some((id, 0..self.blocks[ix].text.len()));
        }
        self.selection(cx)
    }

    /// Show a thread and put the caret in its reply box.
    pub fn open_comment_thread(&mut self, id: ThreadId, window: &mut Window, cx: &mut Context<Self>) {
        if self.comment_thread(id).is_none() {
            return;
        }
        self.open_thread = Some(id);
        self.open_comment_draft(id, window, cx);
        cx.notify();
    }

    fn open_comment_draft(&mut self, thread: ThreadId, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(match self.comment_thread(thread) {
                Some(thread) if !thread.comments.is_empty() => "Reply…",
                _ => "Add a comment…",
            })
        });
        let subscription =
            cx.subscribe_in(&input, window, |this: &mut Self, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit_comment(window, cx);
                }
            });

        input.focus_handle(cx).focus(window, cx);
        self.open_thread = Some(thread);
        self.comment_draft = Some(CommentDraft {
            thread,
            input,
            _subscription: subscription,
        });
        cx.notify();
    }

    /// Post what is in the draft box.
    pub fn submit_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.comment_draft.as_ref() else {
            return;
        };
        let thread = draft.thread;
        let body = draft.input.read(cx).value().trim().to_string();
        if body.is_empty() {
            return;
        }

        if let Some(entry) = self.comments.iter_mut().find(|entry| entry.id == thread) {
            entry.comments.push(Comment::new(DEFAULT_AUTHOR, body));
        }
        if let Some(draft) = self.comment_draft.as_ref() {
            let input = draft.input.clone();
            input.update(cx, |input, cx| input.set_value("", window, cx));
            input.update(cx, |input, cx| {
                input.set_placeholder("Reply…", window, cx);
            });
        }
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// Close the popover; a thread nobody commented on is dropped, the way
    /// Notion discards an abandoned comment box.
    pub fn close_comment_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread) = self.open_thread.take() else {
            self.comment_draft = None;
            return;
        };
        self.comment_draft = None;

        let empty = self
            .comment_thread(thread)
            .is_some_and(|entry| entry.comments.is_empty());
        if empty {
            self.remove_comment_thread(thread, cx);
        }
        if let Some(id) = self.focused_id() {
            self.focus_block(id, super::view::Caret::End, window, cx);
        }
        cx.notify();
    }

    /// Resolve a thread: the highlight comes off and the discussion is kept.
    pub fn resolve_comment_thread(
        &mut self,
        thread: ThreadId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(entry) = self.comments.iter_mut().find(|entry| entry.id == thread) {
            entry.resolved = true;
        }
        self.remove_comment_mark(thread, cx);
        self.open_thread = None;
        self.comment_draft = None;
        if let Some(id) = self.focused_id() {
            self.focus_block(id, super::view::Caret::End, window, cx);
        }
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// Drop a thread and its highlight entirely.
    pub fn remove_comment_thread(&mut self, thread: ThreadId, cx: &mut Context<Self>) {
        self.remove_comment_mark(thread, cx);
        self.comments.retain(|entry| entry.id != thread);
        if self.open_thread == Some(thread) {
            self.open_thread = None;
            self.comment_draft = None;
        }
        cx.emit(DocumentChanged);
        cx.notify();
    }

    fn remove_comment_mark(&mut self, thread: ThreadId, cx: &mut Context<Self>) {
        let kind = MarkKind::Comment(thread);
        let blocks: Vec<BlockId> = self.blocks.iter().map(|block| block.id).collect();
        for id in blocks {
            let Some(ix) = self.index_of(id) else { continue };
            let ranges: Vec<std::ops::Range<usize>> = self.blocks[ix]
                .marks
                .iter()
                .filter(|mark| mark.kind == kind)
                .map(|mark| mark.range.clone())
                .collect();
            if ranges.is_empty() {
                continue;
            }
            for range in ranges {
                self.blocks[ix].marks.remove(&kind, &range);
            }
            self.apply_decorations(id, cx);
        }
    }

    /// The thread under the caret, which is how clicking commented text opens
    /// its discussion.
    pub fn thread_at_caret(&self, cx: &App) -> Option<ThreadId> {
        let id = self.focused_id()?;
        let ix = self.index_of(id)?;
        let caret = self.blocks[ix].state.read(cx).cursor();
        self.blocks[ix]
            .marks
            .active(&(caret..caret))
            .into_iter()
            .find_map(|kind| match kind {
                MarkKind::Comment(thread) => Some(thread),
                _ => None,
            })
    }

    /// A click inside commented text opens that thread; a click anywhere else
    /// closes the one on screen.
    pub(crate) fn open_thread_for_press(
        &mut self,
        event: &gpui_kit::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let thread = self
            .block_at_point(event.position)
            .and_then(|id| self.thread_at_offset(id, cx));

        match thread {
            Some(thread) if self.open_thread != Some(thread) => {
                self.open_comment_thread(thread, window, cx)
            }
            Some(_) => {}
            None if self.open_thread.is_some() => self.close_comment_popover(window, cx),
            None => {}
        }
    }

    /// The thread covering a block's caret, which is where a click just put it.
    fn thread_at_offset(&self, block: BlockId, cx: &App) -> Option<ThreadId> {
        let ix = self.index_of(block)?;
        let caret = self.blocks[ix].state.read(cx).cursor();
        self.blocks[ix]
            .marks
            .active(&(caret..caret))
            .into_iter()
            .find_map(|kind| match kind {
                MarkKind::Comment(thread) => Some(thread),
                _ => None,
            })
    }

    // ------------------------------------------------------------ rendering

    pub(crate) fn render_comment_popover(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let thread = self.open_thread?;
        let entry = self.comment_thread(thread)?;
        let position = self.comment_anchor(entry, cx)?;
        let draft = self.comment_draft.as_ref()?;

        let messages: Vec<AnyElement> = entry
            .comments
            .iter()
            .map(|comment| {
                v_flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().foreground)
                            .child(comment.author.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(cx.theme().foreground)
                            .child(comment.body.clone()),
                    )
                    .into_any_element()
            })
            .collect();

        let card = ui::popover_surface(cx)
            .w(px(300.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(12.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .border_l_2()
                    .border_color(style::comment_accent(cx))
                    .pl(px(8.))
                    .child(SharedString::from(quote_preview(entry.quote()))),
            )
            .children(messages)
            .child(Input::new(&draft.input).id("comment-input").small())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        Button::new("resolve-thread")
                            .ghost()
                            .small()
                            .label("Resolve")
                            .when(entry.comments.is_empty(), |this| this.disabled(true))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.resolve_comment_thread(thread, window, cx)
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(6.))
                            .child(
                                Button::new("cancel-comment")
                                    .ghost()
                                    .small()
                                    .label("Cancel")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_comment_popover(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("post-comment")
                                    .primary()
                                    .small()
                                    .label("Comment")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.submit_comment(window, cx)
                                    })),
                            ),
                    ),
            );

        let _ = window;
        Some(
            deferred(
                gpui_kit::base::Positioner::corner(Anchor::TopLeft, position)
                    .margin(px(8.))
                    .occlude()
                    .child(card),
            )
            .with_priority(OVERLAY_PRIORITY + 1)
            .into_any_element(),
        )
    }

    /// The popover hangs under the commented text.
    fn comment_anchor(&self, thread: &Thread, cx: &App) -> Option<Point<gpui_kit::Pixels>> {
        let ix = self.index_of(thread.block)?;
        let kind = MarkKind::Comment(thread.id);
        let range = self.blocks[ix]
            .marks
            .iter()
            .find(|mark| mark.kind == kind)
            .map(|mark| mark.range.clone());

        if let Some(range) = range {
            if let Some(bounds) = self.blocks[ix].state.read(cx).range_to_bounds(&range) {
                return Some(bounds.origin + Point::new(px(0.), bounds.size.height + px(6.)));
            }
        }
        let bounds = self.block_bounds(thread.block)?;
        Some(bounds.origin + Point::new(style::GUTTER_CONTROLS_WIDTH, bounds.size.height))
    }
}

/// Quotes are one line in the popover, however long the commented run is.
fn quote_preview(quote: &str) -> String {
    const MAX: usize = 80;
    let trimmed: String = quote.chars().take(MAX).collect();
    if quote.chars().count() > MAX {
        format!("{trimmed}…")
    } else {
        trimmed
    }
}
