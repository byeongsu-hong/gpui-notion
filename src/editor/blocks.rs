//! The built-in node types.
//!
//! Each is a [`BlockSpec`]; the registry holds them in the order registered,
//! which is the order the slash menu lists them. An application adds a node
//! type the same way: implement [`BlockSpec`], call
//! [`BlockRegistry::register`].

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::{ActiveTheme, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, img, px, relative,
};

use super::block::{
    BlockAttrs, BlockCaps, BlockContext, BlockInputRule, BlockLayout, BlockRegistry, BlockSpec,
    BlockType, SlashItem, leading_slot, types,
};
use super::style;

/// Register the node types the Notion-like editor ships with.
pub fn init(cx: &mut App) {
    cx.set_global(BlockRegistry::new(std::sync::Arc::new(Paragraph)));
    BlockRegistry::register(cx, Heading);
    BlockRegistry::register(cx, BulletList);
    BlockRegistry::register(cx, OrderedList);
    BlockRegistry::register(cx, TaskList);
    BlockRegistry::register(cx, Blockquote);
    BlockRegistry::register(cx, CodeBlock);
    BlockRegistry::register(cx, HorizontalRule);
    BlockRegistry::register(cx, Image);
    BlockRegistry::register(cx, Callout);
    BlockRegistry::register(cx, Toggle);
}

// ------------------------------------------------------------------ paragraph

pub struct Paragraph;

impl BlockSpec for Paragraph {
    fn type_name(&self) -> &'static str {
        types::PARAGRAPH
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Text".into()
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "Write, type '/' for commands…".into()
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Text",
            subtext: "Regular text paragraph",
            keywords: &["p", "paragraph", "text"],
            group: "Style",
            icon: "type",
            run: |editor, window, cx| editor.set_paragraph(window, cx),
        }]
    }
}

// -------------------------------------------------------------------- heading

pub struct Heading;

impl BlockSpec for Heading {
    fn type_name(&self) -> &'static str {
        types::HEADING
    }

    fn label(&self, attrs: &BlockAttrs) -> SharedString {
        format!("Heading {}", attrs.level.max(1)).into()
    }

    fn layout(&self, attrs: &BlockAttrs) -> BlockLayout {
        // 1.5em / 1.25em / 1.125em over a 1rem base, with the template's
        // em-relative top margins resolved against each heading's own size.
        // The template's heading margins are em-relative to each heading's own
        // size: 3em, 2.5em, 2em.
        let (size, weight, margin_em) = match attrs.level.max(1) {
            1 => (px(24.), FontWeight::BOLD, 3.0),
            2 => (px(20.), FontWeight::BOLD, 2.5),
            3 => (px(18.), FontWeight::SEMIBOLD, 2.0),
            _ => (px(16.), FontWeight::SEMIBOLD, 2.0),
        };
        BlockLayout {
            text_size: size,
            font_weight: weight,
            line_height: 1.3,
            margin_top: size * margin_em,
            margin_bottom: px(2.),
            ..Default::default()
        }
    }

    fn placeholder(&self, attrs: &BlockAttrs) -> SharedString {
        format!("Heading {}", attrs.level.max(1)).into()
    }

    fn placeholder_always(&self) -> bool {
        true
    }

    fn split_into(&self, attrs: &BlockAttrs, at_end: bool) -> (BlockType, BlockAttrs) {
        if at_end {
            (types::PARAGRAPH.into(), BlockAttrs::default())
        } else {
            (types::HEADING.into(), attrs.clone())
        }
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^(#{1,6})\s$",
            build: |caps| {
                let level = caps.get(1)?.as_str().len() as u8;
                Some((types::HEADING.into(), BlockAttrs::level(level)))
            },
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![
            SlashItem {
                title: "Heading 1",
                subtext: "Top-level heading",
                keywords: &["h", "heading1", "h1"],
                group: "Style",
                icon: "heading-1",
                run: |editor, window, cx| editor.toggle_heading(1, window, cx),
            },
            SlashItem {
                title: "Heading 2",
                subtext: "Key section heading",
                keywords: &["h2", "heading2", "subheading"],
                group: "Style",
                icon: "heading-2",
                run: |editor, window, cx| editor.toggle_heading(2, window, cx),
            },
            SlashItem {
                title: "Heading 3",
                subtext: "Subsection and group heading",
                keywords: &["h3", "heading3", "subheading"],
                group: "Style",
                icon: "heading-3",
                run: |editor, window, cx| editor.toggle_heading(3, window, cx),
            },
        ]
    }
}

// ----------------------------------------------------------------- list items

fn list_layout() -> BlockLayout {
    BlockLayout {
        // A list gets air above it; items inside it sit tight together,
        // which `collapse_with_siblings` takes care of.
        margin_top: px(24.),
        leading_width: px(24.),
        collapse_with_siblings: true,
        ..Default::default()
    }
}

pub struct BulletList;

impl BlockSpec for BulletList {
    fn type_name(&self) -> &'static str {
        types::BULLET_LIST
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Bullet List".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps::list()
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        list_layout()
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "List".into()
    }

    fn split_into(&self, _: &BlockAttrs, _: bool) -> (BlockType, BlockAttrs) {
        (types::BULLET_LIST.into(), BlockAttrs::default())
    }

    fn render_leading(&self, ctx: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        // disc → circle → square, cycling every three levels.
        let glyph = match ctx.indent % 3 {
            0 => "•",
            1 => "◦",
            _ => "▪",
        };
        Some(leading_slot(
            &list_layout(),
            div()
                .w_full()
                .text_size(px(16.))
                .text_color(cx.theme().foreground)
                .child(glyph),
        ))
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^\s*([-+*])\s$",
            build: |_| Some((types::BULLET_LIST.into(), BlockAttrs::default())),
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Bullet List",
            subtext: "List with unordered items",
            keywords: &["ul", "li", "list", "bulletlist", "bullet list"],
            group: "Style",
            icon: "list",
            run: |editor, window, cx| editor.toggle_bullet_list(window, cx),
        }]
    }
}

pub struct OrderedList;

impl BlockSpec for OrderedList {
    fn type_name(&self) -> &'static str {
        types::ORDERED_LIST
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Numbered List".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps::list()
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        list_layout()
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "List".into()
    }

    fn split_into(&self, _: &BlockAttrs, _: bool) -> (BlockType, BlockAttrs) {
        (types::ORDERED_LIST.into(), BlockAttrs::default())
    }

    fn render_leading(&self, ctx: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        // decimal → lower-alpha → lower-roman, cycling every three levels.
        let n = ctx.ordinal.max(1);
        let label = match ctx.indent % 3 {
            0 => format!("{n}."),
            1 => format!("{}.", alpha_ordinal(n)),
            _ => format!("{}.", roman_ordinal(n)),
        };
        Some(leading_slot(
            &list_layout(),
            div()
                .w_full()
                .text_size(px(15.))
                .text_color(cx.theme().foreground)
                .child(label),
        ))
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^(\d+)\.\s$",
            build: |caps| {
                let start = caps.get(1)?.as_str().parse().ok();
                Some((
                    types::ORDERED_LIST.into(),
                    BlockAttrs {
                        start,
                        ..Default::default()
                    },
                ))
            },
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Numbered List",
            subtext: "List with ordered items",
            keywords: &["ol", "li", "list", "numberedlist", "numbered list"],
            group: "Style",
            icon: "list-ordered",
            run: |editor, window, cx| editor.toggle_ordered_list(window, cx),
        }]
    }
}

fn alpha_ordinal(n: usize) -> String {
    let mut n = n;
    let mut out = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        out.insert(0, (b'a' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    out
}

fn roman_ordinal(n: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut n = n;
    let mut out = String::new();
    for (value, glyph) in TABLE {
        while n >= value {
            out.push_str(glyph);
            n -= value;
        }
    }
    out
}

pub struct TaskList;

impl BlockSpec for TaskList {
    fn type_name(&self) -> &'static str {
        types::TASK_LIST
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "To-do list".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps::list()
    }

    fn layout(&self, attrs: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            // A checked item is dimmed and struck through.
            text_opacity: if attrs.checked { 0.5 } else { 1.0 },
            strikethrough: attrs.checked,
            ..list_layout()
        }
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "To-do".into()
    }

    fn split_into(&self, _: &BlockAttrs, _: bool) -> (BlockType, BlockAttrs) {
        (types::TASK_LIST.into(), BlockAttrs::default())
    }

    fn render_leading(&self, ctx: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let checked = ctx.attrs.checked;
        let id = ctx.id;
        let editor = ctx.editor.clone();
        let theme = cx.theme();
        let (bg, border_color) = if checked {
            (theme.primary, theme.primary)
        } else {
            (gpui_kit::transparent_black(), theme.border)
        };

        Some(leading_slot(
            &list_layout(),
            div()
                .id(("check", id.0 as usize))
                .size(px(16.))
                .rounded(px(4.))
                .border_1()
                .border_color(border_color)
                .bg(bg)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(checked, |this| {
                    this.child(
                        div()
                            .text_size(px(11.))
                            .text_color(cx.theme().primary_foreground)
                            .child("✓"),
                    )
                })
                .on_click(move |_, _window, cx| {
                    let _ = editor.update(cx, |editor, cx| editor.toggle_check(id, cx));
                }),
        ))
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^\s*\[([ xX])?\]\s$",
            build: |caps| {
                let checked = caps
                    .get(1)
                    .map(|m| m.as_str().eq_ignore_ascii_case("x"))
                    .unwrap_or(false);
                Some((
                    types::TASK_LIST.into(),
                    BlockAttrs {
                        checked,
                        ..Default::default()
                    },
                ))
            },
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "To-do list",
            subtext: "List with tasks",
            keywords: &["tasklist", "task list", "todo", "checklist"],
            group: "Style",
            icon: "list-todo",
            run: |editor, window, cx| editor.toggle_task_list(window, cx),
        }]
    }
}

// ----------------------------------------------------------------- blockquote

pub struct Blockquote;

impl BlockSpec for Blockquote {
    fn type_name(&self) -> &'static str {
        types::BLOCKQUOTE
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Blockquote".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps {
            lifts_when_empty: true,
            ..Default::default()
        }
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            margin_top: px(24.),
            margin_bottom: px(0.),
            inner_padding: px(16.),
            collapse_with_siblings: true,
            ..Default::default()
        }
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "Empty quote".into()
    }

    fn wrap(&self, _: &BlockContext, content: AnyElement, _: &mut Window, cx: &mut App) -> AnyElement {
        h_flex()
            .w_full()
            .items_stretch()
            .py(px(6.))
            .child(
                div()
                    .w(px(4.))
                    .flex_none()
                    .rounded(px(2.))
                    .bg(cx.theme().foreground),
            )
            .child(div().flex_1().pl(px(16.)).child(content))
            .into_any_element()
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^\s*>\s$",
            build: |_| Some((types::BLOCKQUOTE.into(), BlockAttrs::default())),
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Blockquote",
            subtext: "Blockquote block",
            keywords: &["quote", "blockquote"],
            group: "Style",
            icon: "quote",
            run: |editor, window, cx| editor.toggle_blockquote(window, cx),
        }]
    }
}

// ------------------------------------------------------------------ code block

pub struct CodeBlock;

impl BlockSpec for CodeBlock {
    fn type_name(&self) -> &'static str {
        types::CODE_BLOCK
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Code Block".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps {
            marks: false,
            input_rules: false,
            multiline: true,
            ..Default::default()
        }
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            text_size: px(14.),
            line_height: 1.5,
            margin_top: px(24.),
            margin_bottom: px(0.),
            mono: true,
            inner_padding: px(16.),
            ..Default::default()
        }
    }

    fn wrap(&self, ctx: &BlockContext, content: AnyElement, _: &mut Window, cx: &mut App) -> AnyElement {
        let language = ctx
            .attrs
            .language
            .clone()
            .unwrap_or(SharedString::new_static("plain text"));

        v_flex()
            .w_full()
            .relative()
            .p(px(16.))
            .rounded(px(6.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.5))
            .child(content)
            .child(
                // The language sits in the corner and only shows on hover,
                // the way a code block names itself without shouting.
                div()
                    .absolute()
                    .top(px(4.))
                    .right(px(4.))
                    .invisible()
                    .group_hover(super::view::group_name(ctx.id), |this| this.visible())
                    .child(
                        Button::new(("code-language", ctx.id.0 as usize))
                            .ghost()
                            .xsmall()
                            .label(language)
                            .dropdown_menu(move |menu, _window, _cx| {
                                let mut menu = menu.label("Language");
                                for language in CODE_LANGUAGES {
                                    menu = menu.menu(
                                        *language,
                                        Box::new(super::actions::SetCodeLanguage(language)),
                                    );
                                }
                                menu
                            }),
                    ),
            )
            .into_any_element()
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![
            BlockInputRule {
                pattern: r"^```([a-zA-Z0-9+#-]+)?[\s]$",
                build: |caps| {
                    let language = caps.get(1).map(|m| SharedString::from(m.as_str().to_string()));
                    Some((
                        types::CODE_BLOCK.into(),
                        BlockAttrs {
                            language,
                            ..Default::default()
                        },
                    ))
                },
            },
            BlockInputRule {
                pattern: r"^~~~([a-zA-Z0-9+#-]+)?[\s]$",
                build: |caps| {
                    let language = caps.get(1).map(|m| SharedString::from(m.as_str().to_string()));
                    Some((
                        types::CODE_BLOCK.into(),
                        BlockAttrs {
                            language,
                            ..Default::default()
                        },
                    ))
                },
            },
        ]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Code Block",
            subtext: "Code block with syntax highlighting",
            keywords: &["code", "pre"],
            group: "Style",
            icon: "code",
            run: |editor, window, cx| editor.toggle_code_block(window, cx),
        }]
    }
}

// -------------------------------------------------------------- horizontal rule

pub struct HorizontalRule;

impl BlockSpec for HorizontalRule {
    fn type_name(&self) -> &'static str {
        types::HORIZONTAL_RULE
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Separator".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps::atom()
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            margin_top: px(24.),
            margin_bottom: px(24.),
            ..Default::default()
        }
    }

    fn render_body(&self, _: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(
            div()
                .w_full()
                .h(px(1.))
                .bg(cx.theme().border)
                .into_any_element(),
        )
    }

    fn input_rules(&self) -> Vec<BlockInputRule> {
        vec![BlockInputRule {
            pattern: r"^(---|___|\*\*\*)$",
            build: |_| Some((types::HORIZONTAL_RULE.into(), BlockAttrs::default())),
        }]
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Separator",
            subtext: "Horizontal line to separate content",
            keywords: &["hr", "horizontalRule", "line", "separator"],
            group: "Insert",
            icon: "minus",
            run: |editor, window, cx| editor.set_horizontal_rule(window, cx),
        }]
    }
}

// ----------------------------------------------------------------------- image

pub struct Image;

impl BlockSpec for Image {
    fn type_name(&self) -> &'static str {
        types::IMAGE
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Image".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps::atom()
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            margin_top: px(24.),
            margin_bottom: px(24.),
            ..Default::default()
        }
    }

    fn render_body(&self, ctx: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let id = ctx.id;
        let editor = ctx.editor.clone();
        let selected = ctx.selected;

        let Some(src) = ctx.attrs.src.clone() else {
            let dropped = editor.clone();
            return Some(
                div()
                    .id(("image-drop", id.0 as usize))
                    .w_full()
                    .h(px(120.))
                    .rounded(px(6.))
                    .border_1()
                    .border_dashed()
                    .border_color(if selected {
                        cx.theme().primary
                    } else {
                        cx.theme().border
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(|this| this.bg(cx.theme().muted.opacity(0.4)))
                    .text_color(cx.theme().muted_foreground)
                    .child(super::ui::icon("image-up", px(18.), cx.theme().muted_foreground))
                    .child("Click to upload or drag and drop")
                    .on_click(move |_, window, cx| {
                        let _ = editor.update(cx, |editor, cx| editor.pick_image(id, window, cx));
                    })
                    .on_drop(move |paths: &gpui_kit::ExternalPaths, _window, cx| {
                        let Some(path) = paths.paths().first().cloned() else {
                            return;
                        };
                        let _ =
                            dropped.update(cx, |editor, cx| editor.set_image_source(id, path, cx));
                    })
                    .into_any_element(),
            );
        };
        Some(
            div()
                .w_full()
                .when(selected, |this| {
                    this.border_2().border_color(cx.theme().primary)
                })
                .child(img(src.to_string()).w_full().rounded(px(6.)))
                .into_any_element(),
        )
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Image",
            subtext: "Resizable image with caption",
            keywords: &["image", "imageUpload", "upload", "img", "picture", "media"],
            group: "Upload",
            icon: "image",
            run: |editor, window, cx| editor.set_image("", "", window, cx),
        }]
    }
}

// --------------------------------------------------------------------- callout

pub struct Callout;

impl BlockSpec for Callout {
    fn type_name(&self) -> &'static str {
        types::CALLOUT
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Callout".into()
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            margin_top: px(20.),
            inner_padding: px(16.),
            ..Default::default()
        }
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "Callout".into()
    }

    fn wrap(&self, ctx: &BlockContext, content: AnyElement, _: &mut Window, cx: &mut App) -> AnyElement {
        // A document may carry its own emoji; without one the callout uses an
        // icon, which renders on every platform whether or not a color emoji
        // font is installed.
        let marker = match ctx.attrs.emoji.clone() {
            Some(emoji) => div().child(emoji).into_any_element(),
            None => super::ui::icon(
                "lightbulb",
                px(18.),
                cx.theme().muted_foreground,
            )
            .into_any_element(),
        };

        h_flex()
            .w_full()
            .items_start()
            .gap(px(10.))
            .p(px(16.))
            .rounded(px(6.))
            .bg(cx.theme().muted.opacity(0.6))
            .child(div().flex_none().pt(px(2.)).child(marker))
            .child(div().flex_1().child(content))
            .into_any_element()
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Callout",
            subtext: "Make writing stand out",
            keywords: &["callout", "note", "info"],
            group: "Insert",
            icon: "info",
            run: |editor, window, cx| editor.toggle_callout(window, cx),
        }]
    }
}

// ---------------------------------------------------------------------- toggle

pub struct Toggle;

impl BlockSpec for Toggle {
    fn type_name(&self) -> &'static str {
        types::TOGGLE
    }

    fn label(&self, _: &BlockAttrs) -> SharedString {
        "Toggle list".into()
    }

    fn caps(&self) -> BlockCaps {
        BlockCaps {
            lifts_when_empty: true,
            ..Default::default()
        }
    }

    fn layout(&self, _: &BlockAttrs) -> BlockLayout {
        BlockLayout {
            leading_width: px(24.),
            margin_top: px(4.),
            ..Default::default()
        }
    }

    fn placeholder(&self, _: &BlockAttrs) -> SharedString {
        "Toggle".into()
    }

    fn render_leading(&self, ctx: &BlockContext, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let id = ctx.id;
        let collapsed = ctx.attrs.collapsed;
        let editor = ctx.editor.clone();
        Some(leading_slot(
            &BlockLayout {
                leading_width: px(24.),
                ..Default::default()
            },
            div()
                .id(("toggle", id.0 as usize))
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .cursor_pointer()
                .text_size(px(10.))
                .text_color(cx.theme().muted_foreground)
                .hover(|this| this.bg(cx.theme().accent))
                .child(if collapsed { "▶" } else { "▼" })
                .on_click(move |_, _window, cx| {
                    let _ = editor.update(cx, |editor, cx| editor.toggle_collapsed(id, cx));
                }),
        ))
    }

    fn slash_items(&self) -> Vec<SlashItem> {
        vec![SlashItem {
            title: "Toggle list",
            subtext: "Hide and show content",
            keywords: &["toggle", "details", "collapse"],
            group: "Style",
            icon: "chevron-right",
            run: |editor, window, cx| editor.toggle_node(types::TOGGLE, BlockAttrs::default(), window, cx),
        }]
    }
}

/// Languages offered by a code block's language menu. The set a build can
/// actually highlight comes from the `tree-sitter-*` features it enables.
pub const CODE_LANGUAGES: &[&str] = &[
    "bash", "c", "cpp", "css", "diff", "go", "html", "java", "javascript", "json", "kotlin", "lua",
    "markdown", "php", "plain text", "python", "ruby", "rust", "sql", "swift", "toml", "tsx",
    "typescript", "yaml",
];

/// Line height helper shared by specs that size their own children.
pub fn line_height(layout: &BlockLayout) -> gpui_kit::Pixels {
    layout.text_size * layout.line_height
}

/// Relative line height for a text element.
pub fn relative_line_height(layout: &BlockLayout) -> gpui_kit::DefiniteLength {
    relative(layout.line_height)
}

/// Re-exported so specs in other crates can size themselves like the built-ins.
pub use style::TEXT_SIZE as BASE_TEXT_SIZE;
