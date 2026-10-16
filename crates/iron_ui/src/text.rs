//! Text layout with Parley: fonts (bundled Inter, system fallback, game fonts),
//! building a Parley `Layout` for each text block, and measuring it for Taffy.

use std::borrow::Cow;

use parley::fontique::{
    Blob, Collection, CollectionOptions, FontInfoOverride, GenericFamily, SourceCache,
};
use parley::{
    Alignment, AlignmentOptions, BaseDirection, FontContext, FontFamily, FontStyle, FontWeight,
    Layout, LayoutContext, LineHeight as PLineHeight, PositionedLayoutItem, TextStyle,
    TextWrapMode, WhiteSpaceCollapse,
};

use crate::Rgba;
use crate::draw::{GlyphRun, PositionedGlyph, Rect, TextCmd};
use crate::text_style::{ComputedText, Direction, LineHeight, TextAlign, WhiteSpace};

/// The family name the bundled font is registered under.
pub const DEFAULT_FAMILY: &str = "Inter";

const INTER: &[u8] = include_bytes!("../fonts/InterVariable.ttf");
const INTER_ITALIC: &[u8] = include_bytes!("../fonts/InterVariable-Italic.ttf");

/// The brush carried by each glyph run: its text colour.
pub type Brush = Rgba;

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
        let root = to_parley(block);
        let mut builder = self
            .layout_cx
            .tree_builder(&mut self.font_cx, 1.0, true, &root);
        builder.set_base_direction(base_direction(block.direction));
        for (text, style) in runs {
            builder.push_style_span(to_parley(style));
            builder.push_text(text);
            builder.pop_style_span();
        }
        let (layout, _) = builder.build();
        layout
    }
}

/// Measures `layout` for a width constraint, returning (width, height). Also
/// leaves the layout broken into lines for that width.
pub fn measure(
    layout: &mut Layout<Brush>,
    known_width: Option<f32>,
    available: AvailableWidth,
) -> (f32, f32) {
    let max_width = match (known_width, available) {
        (Some(w), _) => Some(w),
        (None, AvailableWidth::Definite(w)) => Some(w),
        (None, AvailableWidth::MinContent) => Some(0.0),
        (None, AvailableWidth::MaxContent) => None,
    };
    layout.break_all_lines(max_width);
    let width = match known_width {
        Some(w) => w,
        // Shrink-to-fit: never wider than the constraint, never narrower than the
        // longest word.
        None => layout.width().ceil(),
    };
    (width, layout.height().ceil())
}

/// Breaks lines at the block's final content width and aligns them.
pub fn finish(layout: &mut Layout<Brush>, content_width: f32, align: TextAlign) {
    layout.break_all_lines(Some(content_width));
    layout.align(alignment(align), AlignmentOptions::default());
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
    let (ox, oy) = origin;
    let mut runs = Vec::new();
    let mut decorations = Vec::new();
    for line in layout.lines() {
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
                })
                .collect();
            if !glyphs.is_empty() {
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
                    color: style.brush,
                    glyphs,
                });
            }
            // Decorations span the run; font metrics give offsets from the baseline
            // (positive is up).
            let metrics = run.font_metrics();
            let baseline = oy + glyph_run.baseline();
            let x = ox + glyph_run.offset();
            let width = glyph_run.advance();
            let mut decorate = |deco: &parley::Decoration<Brush>, offset: f32, size: f32| {
                let offset = deco.offset.unwrap_or(offset);
                let size = deco.size.unwrap_or(size).max(1.0);
                decorations.push((Rect::new(x, baseline - offset, width, size), deco.brush));
            };
            if let Some(u) = &style.underline {
                decorate(u, metrics.underline_offset, metrics.underline_size);
            }
            if let Some(st) = &style.strikethrough {
                decorate(st, metrics.strikethrough_offset, metrics.strikethrough_size);
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
        brush: style.color,
        has_underline: style.decoration.underline,
        has_strikethrough: style.decoration.line_through,
        line_height: match style.line_height {
            LineHeight::Normal => PLineHeight::MetricsRelative(1.0),
            LineHeight::Factor(f) => PLineHeight::FontSizeRelative(f),
            LineHeight::Len(_) => PLineHeight::Absolute(style.line_height_px().unwrap_or(0.0)),
        },
        letter_spacing: style.letter_spacing,
        word_spacing: style.word_spacing,
        white_space_collapse: collapse,
        text_wrap_mode: wrap,
        ..TextStyle::default()
    }
}
