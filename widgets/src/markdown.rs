use crate::{
    link_label::LinkLabel, makepad_derive_widget::*, makepad_draw::*, text_flow::TextFlow,
    scroll_bars::ScrollBars, widget::*, WidgetMatchEvent,
};

use pulldown_cmark::{
    Alignment, CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use std::collections::{HashMap, HashSet};

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    mod.widgets.MarkdownLinkBase = #(MarkdownLink::register_widget(vm))

    mod.widgets.MarkdownBase = #(Markdown::register_widget(vm))

    mod.widgets.MarkdownLink = set_type_default() do mod.widgets.MarkdownLinkBase{
        width: Fit height: Fit
        align: Align{x: 0. y: 0.}

        label_walk: Walk{width: Fit height: Fit}

        draw_icon +: {
            hover: instance(0.0)
            pressed: instance(0.0)

            get_color: fn() {
                return mix(
                    mix(
                        theme.color_label_inner,
                        theme.color_label_inner_hover,
                        self.hover
                    ),
                    theme.color_label_inner_down,
                    self.pressed
                )
            }
        }

        animator: Animator{
            hover: {
                default: @off
                off: AnimatorState{
                    from: {all: Forward {duration: 0.1}}
                    apply: {
                        draw_bg: {pressed: 0.0 hover: 0.0}
                        draw_icon: {pressed: 0.0 hover: 0.0}
                        draw_text: {pressed: 0.0 hover: 0.0}
                    }
                }

                on: AnimatorState{
                    from: {
                        all: Forward {duration: 0.1}
                        pressed: Forward {duration: 0.01}
                    }
                    apply: {
                        draw_bg: {pressed: 0.0 hover: snap(1.0)}
                        draw_icon: {pressed: 0.0 hover: snap(1.0)}
                        draw_text: {pressed: 0.0 hover: snap(1.0)}
                    }
                }

                pressed: AnimatorState{
                    from: {all: Forward {duration: 0.2}}
                    apply: {
                        draw_bg: {pressed: snap(1.0) hover: 1.0}
                        draw_icon: {pressed: snap(1.0) hover: 1.0}
                        draw_text: {pressed: snap(1.0) hover: 1.0}
                    }
                }
            }
        }

        draw_bg +: {
            pressed: instance(0.0)
            hover: instance(0.0)

            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let offset_y = 1.0
                sdf.move_to(0. self.rect_size.y-offset_y)
                sdf.line_to(self.rect_size.x self.rect_size.y-offset_y)
                return sdf.stroke(mix(
                    theme.color_label_inner,
                    theme.color_label_inner_down,
                    self.pressed
                ), mix(0.0, 0.8, self.hover))
            }
        }

        draw_text +: {
            pressed: instance(0.0)
            hover: instance(0.0)

            color_hover: uniform(theme.color_label_inner_hover)
            color_pressed: uniform(theme.color_label_inner_down)

            color: theme.color_label_inner
            text_style: theme.font_regular{
                font_size: theme.font_size_p
            }
            get_color: fn() {
                return mix(
                    mix(
                        self.color,
                        self.color_hover,
                        self.hover
                    ),
                    self.color_pressed,
                    self.pressed
                )
            }
        }
    }

    mod.widgets.Markdown = set_type_default() do mod.widgets.MarkdownBase{
        width: Fill height: Fit
        flow: Flow.Right{wrap: true}
        padding: theme.mspace_1

        font_size: theme.font_size_p
        font_color: theme.color_label_inner

        paragraph_spacing: 16
        pre_code_spacing: 8
        inline_code_padding: theme.mspace_1
        inline_code_margin: theme.mspace_1
        heading_base_scale: 1.8

        draw_text +: {
            color: theme.color_label_inner
        }

        text_style_normal: theme.font_regular{
            font_size: theme.font_size_p
        }

        text_style_italic: theme.font_italic{
            font_size: theme.font_size_p
        }

        text_style_bold: theme.font_bold{
            font_size: theme.font_size_p
        }

        text_style_bold_italic: theme.font_bold_italic{
            font_size: theme.font_size_p
        }

        text_style_fixed: theme.font_code{
            font_size: theme.font_size_p
        }

        code_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{left: theme.space_3, right: theme.space_3, top: theme.space_2, bottom: 10}
        }
        code_walk: Walk{width: Fill height: Fit}

        quote_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{left: theme.space_3, right: theme.space_3, top: theme.space_2, bottom: theme.space_2}
        }
        quote_walk: Walk{width: Fill height: Fit}

        list_item_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: theme.mspace_1
        }
        list_item_walk: Walk{
            height: Fit width: Fill
        }

        sep_walk: Walk{
            width: Fill height: 4.
            margin: theme.mspace_v_1
        }

        draw_block +: {
            line_color: theme.color_label_inner
            sep_color: theme.color_shadow
            quote_bg_color: theme.color_bg_highlight
            quote_fg_color: theme.color_label_inner
            code_color: theme.color_bg_highlight
            selection_color: theme.color_selection_focus
            table_header_bg_color: theme.color_bg_highlight
            table_border_color: theme.color_shadow
            space_1: uniform(theme.space_1)
            space_2: uniform(theme.space_2)
        }

        link := mod.widgets.MarkdownLink{}
    }
}

/// The state of a list at a given nesting level.
struct ListState {
    // Current item number for ordered lists.
    current_number: u64,
    // Start number for ordered lists, None for unordered.
    start_number: Option<u64>,
}

fn heading_slug(
    text: &str,
    prior: &mut HashMap<String, usize>,
    emitted: &mut HashSet<String>,
) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            if pending_dash && !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch);
        } else if ch.is_whitespace() {
            pending_dash = true;
        }
    }
    let suffix = prior.entry(slug.clone()).or_insert(0);
    loop {
        let candidate = if *suffix == 0 {
            slug.clone()
        } else {
            format!("{slug}-{suffix}")
        };
        *suffix += 1;
        if emitted.insert(candidate.clone()) {
            return candidate;
        }
    }
}

fn fragment_scroll_y(anchors: &[(String, f64)], fragment: &str) -> Option<f64> {
    anchors
        .iter()
        .find(|(slug, _)| slug == fragment)
        .map(|(_, y)| *y)
}

fn content_local_heading_y(heading_y: f64, scroll_y: f64, content_origin_y: f64) -> f64 {
    heading_y + scroll_y - content_origin_y
}

#[derive(Script, ScriptHook, Widget)]
pub struct Markdown {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    pub text_flow: TextFlow,
    #[live]
    body: ArcStringMut,
    #[live]
    paragraph_spacing: f64,
    #[live]
    pre_code_spacing: f64,
    #[live(false)]
    use_code_block_widget: bool,
    #[rust]
    in_code_block: bool,
    #[rust]
    code_block_string: String,
    #[rust]
    in_splash_block: bool,
    #[rust]
    splash_block_string: String,
    #[live(false)]
    use_math_widget: bool,
    #[rust]
    auto_id: u64,
    #[live]
    heading_base_scale: f64,
    #[live]
    scroll_bars: ScrollBars,
    #[rust]
    heading_anchors: Vec<(String, f64)>,
}

impl Widget for Markdown {
    fn is_interactive(&self) -> bool {
        false
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.scroll_bars.handle_event(cx, event, scope);
        self.text_flow.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.auto_id = 0;

        self.scroll_bars.begin(cx, walk, Layout::default());
        self.text_flow.begin(cx, Walk::fill_fit());
        self.process_markdown_doc(cx);
        self.text_flow.end(cx);
        self.scroll_bars.end(cx);

        DrawStep::done()
    }

    fn text(&self) -> String {
        self.body.as_ref().to_string()
    }

    fn set_text(&mut self, cx: &mut Cx, v: &str) {
        if self.body.as_ref() != v {
            self.body.set(v);
            self.redraw(cx);
        }
    }
}

impl Markdown {
    fn fragment_y(&self, fragment: &str) -> Option<f64> {
        fragment_scroll_y(&self.heading_anchors, fragment)
    }

    fn process_markdown_doc(&mut self, cx: &mut Cx2d) {
        self.heading_anchors.clear();
        let scroll_y = self.scroll_bars.get_scroll_pos().y;
        let content_origin_y = cx.turtle().pos().y + scroll_y;
        let tf = &mut self.text_flow;
        // Track state for nested formatting
        let mut list_stack: Vec<ListState> = Vec::new();
        let mut is_first_block = true;
        let mut heading_state: Option<(String, f64)> = None;
        let mut heading_slugs = HashMap::new();
        let mut emitted_heading_slugs = HashSet::new();
        // Per-column alignments for the current table, and the current cell's
        // column index within its row. Both are reset when a new table starts.
        let mut table_alignments: Vec<Alignment> = Vec::new();
        let mut table_cell_index: usize = 0;

        let parser = Parser::new_ext(
            self.body.as_ref(),
            Options::ENABLE_TABLES | Options::ENABLE_MATH,
        );

        for event in parser.into_iter() {
            match event {
                MdEvent::Start(Tag::Heading { level, .. }) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    let heading_base = self.heading_base_scale;
                    let scale = match level {
                        HeadingLevel::H1 => heading_base,
                        HeadingLevel::H2 => heading_base * 0.75,
                        HeadingLevel::H3 => heading_base * 0.58,
                        HeadingLevel::H4 => heading_base * 0.5,
                        HeadingLevel::H5 => heading_base * 0.42,
                        HeadingLevel::H6 => heading_base * 0.33,
                    };
                    tf.push_size_abs_scale(scale);
                    tf.bold.push();
                    let y =
                        content_local_heading_y(cx.turtle().pos().y, scroll_y, content_origin_y);
                    heading_state = Some((String::new(), y));
                }
                MdEvent::End(TagEnd::Heading(_level)) => {
                    if let Some((text, y)) = heading_state.take() {
                        let slug = heading_slug(
                            &text,
                            &mut heading_slugs,
                            &mut emitted_heading_slugs,
                        );
                        self.heading_anchors.push((slug, y));
                    }
                    tf.bold.pop();
                    tf.font_sizes.pop();
                    tf.new_line_collapsed(cx);
                }
                MdEvent::Start(Tag::Paragraph) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                }
                MdEvent::End(TagEnd::Paragraph) => {
                    // No special handling needed, turtle position is managed by content/following blocks
                }
                MdEvent::Start(Tag::BlockQuote(_)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.begin_quote(cx);
                }
                MdEvent::End(TagEnd::BlockQuote(_quote_kind)) => {
                    tf.end_quote(cx);
                }
                MdEvent::Start(Tag::List(first_number)) => {
                    list_stack.push(ListState {
                        start_number: first_number,
                        current_number: first_number.unwrap_or(1),
                    });
                }
                MdEvent::End(TagEnd::List(_is_ordered)) => {
                    list_stack.pop();
                }
                MdEvent::Start(Tag::Item) => {
                    if !is_first_block {
                        tf.new_line_collapsed(cx);
                    }
                    is_first_block = false;
                    let marker = if let Some(state) = list_stack.last_mut() {
                        if state.start_number.is_some() {
                            // Ordered list - use and increment the counter
                            let num = state.current_number;
                            state.current_number += 1;
                            format!("{}.", num)
                        } else {
                            // Unordered list - use bullet
                            "•".to_string()
                        }
                    } else {
                        "•".to_string()
                    };
                    tf.begin_list_item(cx, &marker, 2.5);
                }
                MdEvent::End(TagEnd::Item) => {
                    tf.end_list_item(cx);
                }
                MdEvent::Start(Tag::Emphasis) => {
                    tf.italic.push();
                }
                MdEvent::End(TagEnd::Emphasis) => {
                    tf.italic.pop();
                }
                MdEvent::Start(Tag::Strong) => {
                    tf.bold.push();
                }
                MdEvent::End(TagEnd::Strong) => {
                    tf.bold.pop();
                }
                MdEvent::Start(Tag::Strikethrough) => {
                    tf.underline.push();
                }
                MdEvent::End(TagEnd::Strikethrough) => {
                    tf.underline.pop();
                }
                MdEvent::Start(Tag::Link { dest_url, .. }) => {
                    self.auto_id += 1;
                    let item = tf.item(cx, LiveId(self.auto_id), live_id!(link));
                    item.as_markdown_link().set_href(&dest_url);
                    item.draw_all_unscoped(cx);
                }
                MdEvent::End(TagEnd::Link) => {
                    // Link handling is done in Start event
                }
                MdEvent::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    tf.draw_text(cx, "Image[name:");
                    tf.draw_text(cx, &title);
                    tf.draw_text(cx, ", url:");
                    tf.draw_text(cx, &dest_url);
                    tf.draw_text(cx, "]");
                }
                MdEvent::Start(Tag::CodeBlock(kind)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.pre_code_spacing);
                    }
                    is_first_block = false;
                    // Check if this is a runsplash block
                    let is_runsplash = matches!(&kind, CodeBlockKind::Fenced(lang) if lang.as_ref() == "runsplash");
                    if is_runsplash {
                        self.in_splash_block = true;
                        self.splash_block_string.clear();
                    } else if self.use_code_block_widget {
                        self.in_code_block = true;
                        self.code_block_string.clear();
                    } else {
                        const FIXED_FONT_SIZE_SCALE: f64 = 0.85;
                        tf.push_size_rel_scale(FIXED_FONT_SIZE_SCALE);
                        tf.combine_spaces.push(false);
                        tf.fixed.push();
                        tf.begin_code(cx);
                    }
                }
                MdEvent::End(TagEnd::CodeBlock) => {
                    if self.in_splash_block {
                        self.in_splash_block = false;
                        let entry_id = tf.new_counted_id();
                        let sbs = &self.splash_block_string;

                        // Draw the splash block using the $splash_block template
                        tf.item_with(cx, entry_id, id!(splash_block), |cx, item, _tf| {
                            //let tree = item.widget_tree();
                            //cx.with_vm(|vm| {
                            //    log!("$splash_block widget tree:\n{}", tree.display(vm.heap()));
                            //});
                            item.widget(cx, ids!(splash_view)).set_text(cx, sbs);
                            item.draw_all_unscoped(cx);
                        });
                    } else if self.in_code_block {
                        self.in_code_block = false;
                        let entry_id = tf.new_counted_id();
                        let cbs = &self.code_block_string;

                        // Draw the code block and capture the CodeView widget ref
                        let mut code_view_ref = WidgetRef::empty();
                        tf.item_with(cx, entry_id, id!(code_block), |cx, item, _tf| {
                            item.widget(cx, ids!(code_view)).set_text(cx, cbs);
                            item.draw_all_unscoped(cx);
                            code_view_ref = item.widget(cx, ids!(code_view));
                        });

                        // Register the code view widget for cross-child selection
                        // (its area will be queried at event time, not draw time)
                        tf.push_widget_text_for_selection(code_view_ref, &self.code_block_string);
                    } else {
                        tf.font_sizes.pop();
                        tf.fixed.pop();
                        tf.combine_spaces.pop();
                        tf.end_code(cx);
                    }
                }
                // Inline code
                MdEvent::Code(text) => {
                    if let Some((heading_text, _)) = heading_state.as_mut() {
                        heading_text.push_str(&text);
                    }
                    const FIXED_FONT_SIZE_SCALE: f64 = 0.85;
                    tf.push_size_rel_scale(FIXED_FONT_SIZE_SCALE);
                    tf.fixed.push();
                    tf.inline_code.push();
                    tf.draw_text(cx, &text);
                    tf.font_sizes.pop();
                    tf.fixed.pop();
                    tf.inline_code.pop();
                }
                // Inline math ($...$)
                MdEvent::InlineMath(text) => {
                    if self.use_math_widget {
                        let entry_id = tf.new_counted_id();
                        tf.item_with(cx, entry_id, live_id!(inline_math), |cx, item, _tf| {
                            item.set_text(cx, &text);
                            item.draw_all_unscoped(cx);
                        });
                    } else {
                        // Fallback: render as inline code style
                        const FIXED_FONT_SIZE_SCALE: f64 = 0.85;
                        tf.push_size_rel_scale(FIXED_FONT_SIZE_SCALE);
                        tf.fixed.push();
                        tf.inline_code.push();
                        tf.draw_text(cx, &text);
                        tf.font_sizes.pop();
                        tf.fixed.pop();
                        tf.inline_code.pop();
                    }
                }
                // Display math ($$...$$)
                MdEvent::DisplayMath(text) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;

                    if self.use_math_widget {
                        let entry_id = tf.new_counted_id();
                        tf.item_with(cx, entry_id, live_id!(display_math), |cx, item, _tf| {
                            item.set_text(cx, &text);
                            item.draw_all_unscoped(cx);
                        });
                    } else {
                        // Fallback: render as code block style
                        tf.begin_code(cx);
                        tf.fixed.push();
                        tf.draw_text(cx, &text);
                        tf.fixed.pop();
                        tf.end_code(cx);
                    }
                }
                MdEvent::Text(text) => {
                    if let Some((heading_text, _)) = heading_state.as_mut() {
                        heading_text.push_str(&text);
                    }
                    if self.in_splash_block {
                        self.splash_block_string.push_str(&text);
                    } else if self.in_code_block {
                        self.code_block_string.push_str(&text);
                    } else {
                        tf.draw_text(cx, &text.trim_end_matches("\n"));
                    }
                }
                MdEvent::SoftBreak => {
                    if let Some((heading_text, _)) = heading_state.as_mut() {
                        heading_text.push(' ');
                    }
                    if self.in_splash_block {
                        self.splash_block_string.push('\n');
                    } else if self.in_code_block {
                        self.code_block_string.push('\n');
                    } else {
                        tf.draw_text(cx, " ");
                    }
                }
                MdEvent::HardBreak => {
                    if let Some((heading_text, _)) = heading_state.as_mut() {
                        heading_text.push(' ');
                    }
                    if self.in_splash_block {
                        self.splash_block_string.push('\n');
                    } else if self.in_code_block {
                        self.code_block_string.push('\n');
                    } else {
                        tf.new_line_collapsed(cx);
                    }
                }
                MdEvent::Rule => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.sep(cx);
                    tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                }
                MdEvent::TaskListMarker(_) => {
                    // TODO: Implement task list markers
                }
                MdEvent::Start(Tag::Table(alignments)) => {
                    if !is_first_block {
                        tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    }
                    is_first_block = false;
                    tf.begin_table(cx, alignments.len());
                    table_alignments = alignments;
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::Table) => {
                    tf.end_table(cx);
                    tf.new_line_collapsed_with_spacing(cx, self.paragraph_spacing);
                    table_alignments.clear();
                    table_cell_index = 0;
                }
                MdEvent::Start(Tag::TableHead) => {
                    tf.begin_table_header_row(cx);
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::TableHead) => {
                    tf.end_table_row(cx);
                    tf.in_table_header = false;
                }
                MdEvent::Start(Tag::TableRow) => {
                    tf.begin_table_row(cx);
                    table_cell_index = 0;
                }
                MdEvent::End(TagEnd::TableRow) => {
                    tf.end_table_row(cx);
                }
                MdEvent::Start(Tag::TableCell) => {
                    let align_x = table_alignments
                        .get(table_cell_index)
                        .map(alignment_to_x)
                        .unwrap_or(0.0);
                    tf.begin_table_cell(cx, align_x);
                    if tf.in_table_header {
                        tf.bold.push();
                    }
                }
                MdEvent::End(TagEnd::TableCell) => {
                    if tf.in_table_header {
                        tf.bold.pop();
                    }
                    tf.end_table_cell(cx);
                    table_cell_index += 1;
                }
                MdEvent::InlineHtml(text) => {
                    // Support a handful of inline HTML tags that have no
                    // CommonMark equivalent. Anything not matched is ignored,
                    // matching the pre-existing behavior.
                    match text.trim().to_ascii_lowercase().as_str() {
                        "<sub>" => {
                            tf.push_size_rel_scale(0.7);
                            tf.y_shift_scales.push(0.55);
                        }
                        "</sub>" => {
                            tf.font_sizes.pop();
                            tf.y_shift_scales.pop();
                        }
                        "<sup>" => {
                            tf.push_size_rel_scale(0.7);
                            tf.y_shift_scales.push(-0.2);
                        }
                        "</sup>" => {
                            tf.font_sizes.pop();
                            tf.y_shift_scales.pop();
                        }
                        _ => {}
                    }
                }
                _ => {} // Unimplemented or unnecessary events
            }
        }
    }
}

/// Maps pulldown_cmark table-column alignment to `Layout::align.x`.
fn alignment_to_x(alignment: &Alignment) -> f64 {
    match alignment {
        Alignment::None | Alignment::Left => 0.0,
        Alignment::Center => 0.5,
        Alignment::Right => 1.0,
    }
}

impl MarkdownRef {
    pub fn scroll_y(&self) -> f64 {
        self.borrow()
            .map(|inner| inner.scroll_bars.get_scroll_pos().y)
            .unwrap_or(0.0)
    }

    pub fn set_scroll_y(&self, cx: &mut Cx, y: f64) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        let y = if y.is_finite() { y.max(0.0) } else { 0.0 };
        let x = inner.scroll_bars.get_scroll_pos().x;
        inner.scroll_bars.set_scroll_pos(cx, dvec2(x, y));
        inner.redraw(cx);
    }

    pub fn scroll_to_fragment(&self, cx: &mut Cx, fragment: &str) -> bool {
        let Some(mut inner) = self.borrow_mut() else {
            return false;
        };
        let Some(y) = inner.fragment_y(fragment) else {
            return false;
        };
        let x = inner.scroll_bars.get_scroll_pos().x;
        inner.scroll_bars.set_scroll_pos(cx, dvec2(x, y));
        inner.redraw(cx);
        true
    }

    pub fn set_text(&mut self, cx: &mut Cx, v: &str) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.set_text(cx, v)
    }

    /// Start streaming text animation with fade-in effect.
    pub fn start_streaming_animation(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.text_flow.start_streaming_animation();
        }
    }

    /// Reset and start streaming animation (for reused widgets).
    pub fn reset_streaming_animation(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.text_flow.reset_streaming_animation();
        }
    }

    /// Stop streaming animation (fade will complete naturally).
    pub fn stop_streaming_animation(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.text_flow.stop_streaming_animation();
        }
    }

    /// Check if streaming animation is completely done.
    pub fn is_streaming_animation_done(&self) -> bool {
        if let Some(inner) = self.borrow() {
            inner.text_flow.is_streaming_animation_done()
        } else {
            true
        }
    }

    /// Reset all streaming animations (text fade).
    pub fn reset_all_streaming_animations(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.text_flow.reset_all_streaming_animations();
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
struct MarkdownLink {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    link: LinkLabel,
    #[live]
    href: String,
}

impl WidgetMatchEvent for MarkdownLink {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        if self.link.clicked(actions) {
            cx.widget_action(
                self.widget_uid(),
                MarkdownAction::LinkNavigated(self.href.clone()),
            );
        }
    }
}

impl Widget for MarkdownLink {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.link.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope)
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.link.draw_walk(cx, scope, walk)
    }

    fn text(&self) -> String {
        self.link.text()
    }

    fn set_text(&mut self, cx: &mut Cx, v: &str) {
        self.link.set_text(cx, v);
    }
}

impl MarkdownLinkRef {
    pub fn set_href(&self, v: &str) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.href = v.to_string();
    }
}

#[derive(Clone, Debug, Default)]
pub enum MarkdownAction {
    #[default]
    None,
    LinkNavigated(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn github_heading_slugs_are_stable_and_suffix_duplicates() {
        let mut prior = HashMap::new();
        let mut emitted = HashSet::new();
        assert_eq!(
            heading_slug("Customer Overview!", &mut prior, &mut emitted),
            "customer-overview"
        );
        assert_eq!(
            heading_slug("Customer   Overview", &mut prior, &mut emitted),
            "customer-overview-1"
        );
        assert_eq!(
            heading_slug("API: `create()`", &mut prior, &mut emitted),
            "api-create"
        );
    }

    #[test]
    fn heading_slugs_skip_natural_suffix_collisions() {
        let mut prior = HashMap::new();
        let mut emitted = HashSet::new();
        assert_eq!(heading_slug("Foo", &mut prior, &mut emitted), "foo");
        assert_eq!(
            heading_slug("Foo-1", &mut prior, &mut emitted),
            "foo-1"
        );
        assert_eq!(heading_slug("Foo", &mut prior, &mut emitted), "foo-2");
    }

    #[test]
    fn fragment_lookup_uses_the_recorded_slug() {
        let anchors = vec![
            ("overview".into(), 12.0),
            ("overview-1".into(), 84.0),
        ];
        assert_eq!(fragment_scroll_y(&anchors, "overview-1"), Some(84.0));
        assert_eq!(fragment_scroll_y(&anchors, "missing"), None);
    }

    #[test]
    fn fragment_scroll_finds_anchor_and_missing_anchor_is_harmless() {
        let anchors = vec![("intro".into(), 0.0), ("details".into(), 140.0)];
        assert_eq!(fragment_scroll_y(&anchors, "details"), Some(140.0));
        assert_eq!(fragment_scroll_y(&anchors, "unknown"), None);
    }

    #[test]
    fn markdown_link_action_preserves_the_original_href() {
        let action = MarkdownAction::LinkNavigated("../customer.md#orders".into());
        assert!(matches!(
            action,
            MarkdownAction::LinkNavigated(href) if href == "../customer.md#orders"
        ));
    }

    #[test]
    fn fragment_scroll_on_an_empty_markdown_ref_is_harmless() {
        let markdown = WidgetRef::empty().as_markdown();
        let mut cx = Cx::new(Box::new(|_, _| {}));
        assert!(!markdown.scroll_to_fragment(&mut cx, "missing"));
    }

    fn draw_markdown_headless(
        cx: &mut Cx,
        draw_event: &DrawEvent,
        pass: &DrawPass,
        draw_list: &mut DrawList2d,
        markdown: &WidgetRef,
    ) {
        let mut cx_draw = CxDraw::new(cx, draw_event);
        let mut cx_2d = Cx2d::new(&mut cx_draw);
        cx_2d.begin_pass(pass, None);
        draw_list.begin_always(&mut cx_2d);
        let size = cx_2d.current_pass_size();
        cx_2d.begin_root_turtle(size, Layout::flow_down());
        markdown.draw_walk_all(&mut cx_2d, &mut Scope::empty(), Walk::fill());
        cx_2d.end_pass_sized_turtle();
        draw_list.end(&mut cx_2d);
        cx_2d.end_pass(pass);
    }

    #[test]
    fn heading_anchor_stays_stable_after_real_scrolled_redraw() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            crate::script_mod(vm);
            let mut inner = Markdown::script_new_with_default(vm);
            inner.body.set(
                "# Intro\n\n\
                 Paragraph one.\n\n\
                 Paragraph two.\n\n\
                 Paragraph three.\n\n\
                 Paragraph four.\n\n\
                 Paragraph five.\n\n\
                 # Details\n",
            );
            let markdown = WidgetRef::new_with_inner(Box::new(inner));

            vm.with_cx_mut(|cx| {
                let pass = DrawPass::new_with_name(cx, "markdown_anchor_stability_test");
                pass.set_size(cx, dvec2(240.0, 80.0));
                let mut draw_list = DrawList2d::new(cx);
                let draw_event = DrawEvent {
                    redraw_all: true,
                    ..Default::default()
                };

                draw_markdown_headless(cx, &draw_event, &pass, &mut draw_list, &markdown);
                let first_y = markdown
                    .borrow::<Markdown>()
                    .and_then(|inner| inner.fragment_y("details"))
                    .expect("first draw should record the details anchor");
                assert!(first_y > 0.0);

                let markdown_ref = markdown.clone().as_markdown();
                assert!(markdown_ref.scroll_to_fragment(cx, "details"));
                let first_scroll_y = markdown
                    .borrow::<Markdown>()
                    .expect("markdown should remain borrowable")
                    .scroll_bars
                    .get_scroll_pos()
                    .y;
                assert!(first_scroll_y > 0.0);

                draw_markdown_headless(cx, &draw_event, &pass, &mut draw_list, &markdown);
                let redrawn_y = markdown
                    .borrow::<Markdown>()
                    .and_then(|inner| inner.fragment_y("details"))
                    .expect("redraw should preserve the details anchor");
                assert!((redrawn_y - first_y).abs() < 0.001);

                assert!(markdown_ref.scroll_to_fragment(cx, "details"));
                let second_scroll_y = markdown
                    .borrow::<Markdown>()
                    .expect("markdown should remain borrowable")
                    .scroll_bars
                    .get_scroll_pos()
                    .y;
                assert!((second_scroll_y - first_scroll_y).abs() < 0.001);
            });
        });
    }

    #[test]
    fn markdown_ref_round_trips_vertical_scroll_state() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            crate::script_mod(vm);
            let mut inner = Markdown::script_new_with_default(vm);
            inner.body.set(
                "# Intro\n\n\
                 Paragraph one.\n\n\
                 Paragraph two.\n\n\
                 Paragraph three.\n\n\
                 Paragraph four.\n\n\
                 Paragraph five.\n\n\
                 # Details\n",
            );
            let markdown = WidgetRef::new_with_inner(Box::new(inner));

            vm.with_cx_mut(|cx| {
                let pass = DrawPass::new_with_name(cx, "markdown_scroll_state_test");
                pass.set_size(cx, dvec2(240.0, 80.0));
                let mut draw_list = DrawList2d::new(cx);
                let draw_event = DrawEvent {
                    redraw_all: true,
                    ..Default::default()
                };
                draw_markdown_headless(cx, &draw_event, &pass, &mut draw_list, &markdown);

                let markdown_ref = markdown.as_markdown();
                markdown_ref.set_scroll_y(cx, 24.0);
                assert!((markdown_ref.scroll_y() - 24.0).abs() < 0.001);

                markdown_ref.set_scroll_y(cx, f64::NAN);
                assert_eq!(markdown_ref.scroll_y(), 0.0);
            });
        });
    }
}
