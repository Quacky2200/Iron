// SPDX-License-Identifier: Apache-2.0

//! Text layout with Parley: fonts (bundled Inter, system fallback, game fonts),
//! building a Parley `Layout` for each text block, and measuring it for Taffy.

use std::borrow::Cow;

use iron_core::NodeId;

use parley::fontique::{
    Blob, Collection, CollectionOptions, FontInfoOverride, GenericFamily, SourceCache,
};
use parley::{
    Alignment, AlignmentOptions, BaseDirection, FontContext, FontFamily, FontStyle, FontWeight,
    Layout, LayoutContext, LineHeight as PLineHeight, PositionedLayoutItem, TextStyle,
    TextWrapMode, WhiteSpaceCollapse, YieldData,
};

use crate::Rgba;
use crate::draw::{GlyphRun, PositionedGlyph, Rect, TextCmd, TextDecoration};
use crate::text_style::{
    ComputedText, Decoration, DecorationStyle, Direction, LineHeight, TextAlign, WhiteSpace,
};

/// The family name the bundled font is registered under.
pub const DEFAULT_FAMILY: &str = "Inter";

/// The bundled font (also given to the SVG renderer for `<text>`).
pub const INTER: &[u8] = include_bytes!("../fonts/InterVariable.ttf");
pub const INTER_ITALIC: &[u8] = include_bytes!("../fonts/InterVariable-Italic.ttf");

/// What each glyph run carries through Parley: its colour and its decorations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brush {
    pub color: Rgba,
    pub lines: Decoration,
    pub line_style: DecorationStyle,
    /// `None`: the text colour.
    pub line_color: Option<Rgba>,
    pub thickness: Option<f32>,
    pub underline_offset: Option<f32>,
    pub stroke_width: f32,
    /// `None`: the text colour.
    pub stroke_color: Option<Rgba>,
    pub smooth: bool,
    /// `visibility: hidden` text keeps its place but isn't drawn.
    pub hidden: bool,
    /// The node these characters came from, for hit testing spans.
    pub node: Option<NodeId>,
}

/// The initial text style's brush (anti-aliased, black).
impl Default for Brush {
    fn default() -> Self {
        Self::of(&ComputedText::default())
    }
}

impl Brush {
    pub fn of(style: &ComputedText) -> Self {
        Self {
            color: style.color,
            lines: style.decoration,
            line_style: style.decoration_style,
            line_color: style.decoration_color,
            thickness: style.decoration_thickness,
            underline_offset: style.underline_offset,
            stroke_width: style.stroke_width,
            stroke_color: style.stroke_color,
            smooth: style.smooth,
            hidden: !style.visible,
            node: None,
        }
    }
}

/// Width constraint for measuring text, mirroring Taffy's available space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AvailableWidth {
    Definite(f32),
    MinContent,
    MaxContent,
}

/// Fonts plus Parley's reusable scratch space.
pub struct TextEngine {
    font_cx: FontContext,
    layout_cx: LayoutContext<Brush>,
}

impl TextEngine {
    /// `system_fonts` adds the platform's fonts for fallback (CJK, emoji, ...).
    /// Tests turn it off so results don't depend on the machine.
    pub fn new(system_fonts: bool) -> Self {
        let collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts,
        });
        let mut engine = Self {
            font_cx: FontContext {
                collection,
                source_cache: SourceCache::default(),
            },
            layout_cx: LayoutContext::new(),
        };
        let mut ids = Vec::new();
        for data in [INTER, INTER_ITALIC] {
            let override_family = FontInfoOverride {
                family_name: Some(DEFAULT_FAMILY),
                ..Default::default()
            };
            let registered = engine
                .font_cx
                .collection
                .register_fonts(Blob::new(std::sync::Arc::new(data)), Some(override_family));
            ids.extend(registered.into_iter().map(|(id, _)| id));
        }
        ids.dedup();
        // Generic sans-serif families resolve to the bundled font first, then any
        // system fallbacks fontique already knows about.
        for generic in [
            GenericFamily::SansSerif,
            GenericFamily::SystemUi,
            GenericFamily::UiSansSerif,
        ] {
            let existing: Vec<_> = engine
                .font_cx
                .collection
                .generic_families(generic)
                .collect();
            let merged: Vec<_> = ids
                .iter()
                .copied()
                .chain(existing.into_iter().filter(|id| !ids.contains(id)))
                .collect();
            engine
                .font_cx
                .collection
                .set_generic_families(generic, merged.into_iter());
        }
        engine
    }

    /// Registers font data (TTF/OTF, or a collection). `family`, `weight` and
    /// `italic` override what the font file says, like `@font-face` descriptors.
    /// Returns the family names that were registered.
    pub fn register_font(
        &mut self,
        data: Vec<u8>,
        family: Option<&str>,
        weight: Option<f32>,
        italic: Option<bool>,
    ) -> Result<Vec<String>, String> {
        let data = decode_web_font(data)?;
        let info = FontInfoOverride {
            family_name: family,
            weight: weight.map(FontWeight::new),
            style: italic.map(|i| {
                if i {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                }
            }),
            ..Default::default()
        };
        let registered = self
            .font_cx
            .collection
            .register_fonts(Blob::from(data), Some(info));
        if registered.is_empty() {
            return Err("no usable fonts in the data".into());
        }
        // Parley's layout context caches family-name lookups (including misses),
        // so start a fresh one for the new family to be found.
        self.layout_cx = LayoutContext::new();
        let mut names = Vec::new();
        for (id, _) in registered {
            if let Some(name) = self.font_cx.collection.family_name(id) {
                names.push(name.to_owned());
            }
        }
        names.dedup();
        Ok(names)
    }

    /// The font and layout contexts, for Parley editors.
    pub(crate) fn contexts(&mut self) -> (&mut FontContext, &mut LayoutContext<Brush>) {
        (&mut self.font_cx, &mut self.layout_cx)
    }

    /// Lays out one text block: `runs` are its characters with the computed style
    /// of the node each run came from, in order; `block` is the block's own style.
    pub fn build(
        &mut self,
        block: &ComputedText,
        runs: &[(String, ComputedText)],
    ) -> Layout<Brush> {
        self.build_with_text(block, runs, None).0
    }

    /// [`Self::build`], also returning the text as laid out (white space collapsed),
    /// which selection byte ranges index. `nodes`, when given, holds each run's node,
    /// kept in its brush for [`node_at`].
    pub fn build_with_text(
        &mut self,
        block: &ComputedText,
        runs: &[(String, ComputedText)],
        nodes: Option<&[NodeId]>,
    ) -> (Layout<Brush>, String) {
        let root = to_parley(block);
        let mut builder = self
            .layout_cx
            .tree_builder(&mut self.font_cx, 1.0, true, &root);
        builder.set_base_direction(base_direction(block.direction));
        for (i, (text, style)) in runs.iter().enumerate() {
            let mut style = to_parley(style);
            style.brush.node = nodes.and_then(|n| n.get(i).copied());
            builder.push_style_span(style);
            builder.push_text(text);
            builder.pop_style_span();
        }
        builder.build()
    }
}

/// The node whose characters are under layout point (x, y), if any.
pub fn node_at(layout: &Layout<Brush>, x: f32, y: f32) -> Option<NodeId> {
    let (cluster, _) = parley::Cluster::from_point_exact(layout, x, y)?;
    cluster.style().brush.node
}

/// WOFF and WOFF2 fonts (recognized by their signature) decompressed to plain TTF/OTF
/// data; other data is passed through.
pub fn decode_web_font(data: Vec<u8>) -> Result<Vec<u8>, String> {
    match data.get(..4) {
        Some(b"wOFF") => wuff::decompress_woff1(&data).map_err(|e| format!("bad WOFF data: {e:?}")),
        Some(b"wOF2") => {
            wuff::decompress_woff2(&data).map_err(|e| format!("bad WOFF2 data: {e:?}"))
        }
        _ => Ok(data),
    }
}

/// A strip of a text block's content box where floats leave room: lines whose top is at
/// or below `y` (and above the next band's `y`) start at `x` and are at most `width` wide.
/// The first band starts at 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub y: f32,
    pub x: f32,
    pub width: f32,
}

/// Measures `layout` for a width constraint, returning (width, height). Also
/// leaves the layout broken into lines for that width. `bands` (see [`Band`]) make the
/// lines avoid floats; they only apply to a definite width.
pub fn measure(
    layout: &mut Layout<Brush>,
    known_width: Option<f32>,
    available: AvailableWidth,
    bands: &[Band],
) -> (f32, f32) {
    let max_width = match (known_width, available) {
        (Some(w), _) => Some(w),
        (None, AvailableWidth::Definite(w)) => Some(w),
        (None, AvailableWidth::MinContent) => Some(0.0),
        (None, AvailableWidth::MaxContent) => None,
    };
    let around_floats = !bands.is_empty() && matches!(available, AvailableWidth::Definite(_));
    match max_width {
        Some(w) if around_floats => break_in_bands(layout, w, bands),
        _ => layout.break_all_lines(max_width),
    }
    let width = match known_width {
        Some(w) => w,
        // Shrink-to-fit: never wider than the constraint, never narrower than the
        // longest word.
        None => layout.width().ceil(),
    };
    (width, text_height(layout).ceil())
}

/// Breaks lines at the block's final content width, around the floats `bands` describe,
/// and aligns them (each line within its own band).
pub fn finish(layout: &mut Layout<Brush>, content_width: f32, align: TextAlign, bands: &[Band]) {
    if bands.is_empty() {
        layout.break_all_lines(Some(content_width));
    } else {
        break_in_bands(layout, content_width, bands);
    }
    layout.align(alignment(align), AlignmentOptions::default());
}

/// The bottom of the last line. Lines pushed below a float leave a gap Parley's own
/// height (the sum of the line heights) doesn't count.
fn text_height(layout: &Layout<Brush>) -> f32 {
    let last = layout
        .lines()
        .last()
        .map_or(0.0, |l| l.metrics().block_max_coord);
    layout.height().max(last)
}

/// Line breaking with a different start and width for each band. A line that doesn't fit
/// the band it starts in (too tall, or a word wider than the room) moves down to the next
/// one, as in CSS, unless the band is as wide as the whole block.
fn break_in_bands(layout: &mut Layout<Brush>, width: f32, bands: &[Band]) {
    let mut lines = layout.break_lines();
    lines.state_mut().set_layout_max_advance(width);
    while !lines.is_done() {
        let y = lines.committed_y() as f32;
        let i = bands.iter().rposition(|b| b.y <= y + 0.01).unwrap_or(0);
        let band = bands[i];
        let below = bands.get(i + 1).map(|b| b.y);
        let saved = {
            let state = lines.state_mut();
            state.set_line_x(band.x);
            state.set_line_max_advance(band.width.min(width));
            state.set_line_max_height(below.map_or(f32::MAX, |b| b - y));
            state.clone()
        };
        let moved = match lines.break_next() {
            None => break,
            Some(YieldData::LineBreak(data)) => {
                let used = data.metrics.advance - data.metrics.hanging_advance;
                used > band.width + 0.5 && band.width + 0.5 < width
            }
            Some(YieldData::MaxHeightExceeded(_)) => true,
            // Only in-flow inline content exists, so the breaker never yields a box.
            Some(YieldData::InlineBoxBreak(_)) => false,
        };
        if let (true, Some(below)) = (moved, below) {
            lines.revert_to(saved);
            lines.state_mut().set_line_y(f64::from(below));
        }
    }
    lines.finish();
}

/// Parley's alignment for `text-align`; `start` / `end` follow the layout's base direction.
pub(crate) fn alignment(align: TextAlign) -> Alignment {
    match align {
        TextAlign::Start => Alignment::Start,
        TextAlign::End => Alignment::End,
        TextAlign::Left => Alignment::Left,
        TextAlign::Right => Alignment::Right,
        TextAlign::Center => Alignment::Center,
        TextAlign::Justify => Alignment::Justify,
    }
}

/// The paragraph direction for CSS `direction`. Never `Auto`: as in CSS, Arabic text in
/// an `ltr` element keeps its right-to-left order but the paragraph runs left to right.
pub(crate) fn base_direction(direction: Direction) -> BaseDirection {
    match direction {
        Direction::Ltr => BaseDirection::Ltr,
        Direction::Rtl => BaseDirection::Rtl,
    }
}

/// Draw command for a laid-out block whose content box starts at `origin`.
pub fn draw(layout: &Layout<Brush>, origin: (f32, f32)) -> TextCmd {
    draw_lines(layout, origin, None)
}

/// Where lines are cut short for `text-overflow: ellipsis`.
pub struct Ellipsis<'a> {
    /// The `…` laid out in the block's style.
    pub layout: &'a Layout<Brush>,
    /// The content box's width: lines wider than this are cut.
    pub width: f32,
    /// Cut at the left (right-to-left text) instead of the right.
    pub rtl: bool,
}

/// Like [`draw`], but lines wider than the content box end in an ellipsis: glyphs
/// that would overlap it are dropped (whole glyphs, as browsers do).
pub fn draw_ellipsized(layout: &Layout<Brush>, origin: (f32, f32), e: Ellipsis<'_>) -> TextCmd {
    draw_lines(layout, origin, Some(e))
}

fn draw_lines(
    layout: &Layout<Brush>,
    origin: (f32, f32),
    ellipsis: Option<Ellipsis<'_>>,
) -> TextCmd {
    let (ox, oy) = origin;
    let mut runs = Vec::new();
    let mut decorations = Vec::new();
    for line in layout.lines() {
        // The kept span of a cut line: glyphs must end before `limit` (ltr) or start
        // after it (rtl).
        let cut = ellipsis.as_ref().filter(|e| {
            let m = line.metrics();
            m.offset + m.advance > e.width + 0.5 || m.offset < -0.5
        });
        let ellipsis_width = cut.map_or(0.0, |e| e.layout.full_width());
        let (lo, hi) = (
            ox + ellipsis_width,
            ox + cut.map_or(f32::MAX, |e| e.width) - ellipsis_width,
        );
        let keep = |g: &PositionedGlyph| match cut {
            None => true,
            Some(e) if e.rtl => g.x >= lo - 0.01,
            Some(_) => g.x + g.advance <= hi + 0.01,
        };
        let (mut kept_lo, mut kept_hi) = (f32::MAX, f32::MIN);
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let run = glyph_run.run();
            let style = glyph_run.style();
            let synthesis = run.synthesis();
            let glyphs: Vec<PositionedGlyph> = glyph_run
                .positioned_glyphs()
                .map(|g| PositionedGlyph {
                    id: g.id,
                    x: ox + g.x,
                    y: oy + g.y,
                    advance: g.advance,
                })
                .filter(|g| keep(g))
                .collect();
            for g in &glyphs {
                kept_lo = kept_lo.min(g.x);
                kept_hi = kept_hi.max(g.x + g.advance);
            }
            if !glyphs.is_empty() && !style.brush.hidden {
                runs.push(GlyphRun {
                    font: run.font().clone(),
                    font_size: run.font_size(),
                    normalized_coords: run
                        .normalized_coords()
                        .iter()
                        .map(|c| c.to_bits())
                        .collect(),
                    embolden: synthesis.embolden(),
                    skew: synthesis.skew(),
                    color: style.brush.color,
                    smooth: style.brush.smooth,
                    stroke: (style.brush.stroke_width > 0.0).then(|| {
                        let b = style.brush;
                        (b.stroke_width, b.stroke_color.unwrap_or(b.color))
                    }),
                    glyphs,
                });
            }
            // Decorations span the run; font metrics give offsets from the baseline
            // (positive is up).
            let metrics = run.font_metrics();
            let brush = style.brush;
            let baseline = oy + glyph_run.baseline();
            let mut x = ox + glyph_run.offset();
            let mut width = glyph_run.advance();
            // Decorations stop where the kept glyphs do.
            match cut {
                Some(e) if e.rtl => {
                    let start = x.max(lo);
                    width = (x + width - start).max(0.0);
                    x = start;
                }
                Some(_) => width = width.min(hi - x).max(0.0),
                None => {}
            }
            // Offsets are up from the baseline; the thickness and colour can be
            // set, or come from the font and the text colour.
            // Hidden runs draw no decorations either.
            let color = if brush.hidden {
                crate::Rgba::TRANSPARENT
            } else {
                brush.line_color.unwrap_or(brush.color)
            };
            let mut decorate = |offset: f32, size: f32| {
                let size = brush.thickness.unwrap_or(size).max(1.0);
                decorations.push(TextDecoration {
                    rect: Rect::new(x, baseline - offset, width, size),
                    color,
                    style: brush.line_style,
                });
            };
            // Parley's own underline is the editor's IME pre-edit marker.
            if brush.lines.underline || style.underline.is_some() {
                let offset = brush
                    .underline_offset
                    .map_or(metrics.underline_offset, |o| -o);
                decorate(offset, metrics.underline_size);
            }
            if brush.lines.overline {
                decorate(metrics.ascent, metrics.underline_size);
            }
            if brush.lines.line_through {
                decorate(metrics.strikethrough_offset, metrics.strikethrough_size);
            }
        }
        if let Some(e) = cut {
            // The ellipsis goes right after the kept glyphs (before them for rtl), on
            // this line's baseline.
            let baseline = oy + line.metrics().baseline;
            let at = if kept_lo > kept_hi {
                if e.rtl { ox + e.width } else { ox }
            } else if e.rtl {
                kept_lo
            } else {
                kept_hi
            };
            let start = if e.rtl { at - ellipsis_width } else { at };
            if let Some(dots) = e.layout.lines().next() {
                let dy = baseline - dots.metrics().baseline;
                let cmd = draw_lines(e.layout, (start, dy), None);
                runs.extend(cmd.runs);
            }
        }
    }
    let bounds = Rect::new(
        ox,
        oy,
        layout.full_width().max(layout.width()),
        layout.height(),
    );
    TextCmd {
        bounds,
        runs,
        decorations,
        fill: None,
    }
}

pub(crate) fn to_parley(style: &ComputedText) -> TextStyle<'static, 'static, Brush> {
    let (collapse, wrap) = match style.white_space {
        WhiteSpace::Normal => (WhiteSpaceCollapse::Collapse, TextWrapMode::Wrap),
        WhiteSpace::NoWrap => (WhiteSpaceCollapse::Collapse, TextWrapMode::NoWrap),
        WhiteSpace::Pre => (WhiteSpaceCollapse::Preserve, TextWrapMode::NoWrap),
        WhiteSpace::PreWrap => (WhiteSpaceCollapse::Preserve, TextWrapMode::Wrap),
        WhiteSpace::PreLine => (WhiteSpaceCollapse::PreserveBreaks, TextWrapMode::Wrap),
        WhiteSpace::BreakSpaces => (WhiteSpaceCollapse::BreakSpaces, TextWrapMode::Wrap),
    };
    TextStyle {
        font_family: FontFamily::Source(Cow::Owned(style.font_family.clone())),
        font_size: style.font_size,
        font_style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        font_weight: FontWeight::new(style.font_weight),
        brush: Brush::of(style),
        line_height: match style.line_height {
            LineHeight::Normal => PLineHeight::MetricsRelative(1.0),
            LineHeight::Factor(f) => PLineHeight::FontSizeRelative(f),
            LineHeight::Len(_) => PLineHeight::Absolute(style.line_height_px().unwrap_or(0.0)),
        },
        letter_spacing: style.letter_spacing,
        word_spacing: style.word_spacing,
        white_space_collapse: collapse,
        text_wrap_mode: wrap,
        font_features: match style.font_features() {
            f if f.is_empty() => parley::FontFeatures::empty(),
            f => parley::FontFeatures::Source(Cow::Owned(f)),
        },
        word_break: match style.word_break {
            crate::text_style::WordBreak::Normal => parley::WordBreak::Normal,
            crate::text_style::WordBreak::BreakAll => parley::WordBreak::BreakAll,
            crate::text_style::WordBreak::KeepAll => parley::WordBreak::KeepAll,
        },
        overflow_wrap: match style.overflow_wrap {
            crate::text_style::OverflowWrap::Normal => parley::OverflowWrap::Normal,
            crate::text_style::OverflowWrap::Anywhere => parley::OverflowWrap::Anywhere,
            crate::text_style::OverflowWrap::BreakWord => parley::OverflowWrap::BreakWord,
        },
        ..TextStyle::default()
    }
}
