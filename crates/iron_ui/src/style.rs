//! Applying style properties from React to a node.
//!
//! Names are React-style camelCase. Numbers are logical pixels; strings are CSS
//! values parsed with Taffy's own CSS parser (layout) or [`Rgba::parse`] (colours).
//! The same functions will back CSS files in M3.

use std::str::FromStr;

use iron_core::Value;
use taffy::{
    AlignContent, AlignItems, Dimension, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTemplateComponent, GridTemplateTracks, LengthPercentage,
    LengthPercentageAuto, Line, Position, Rect, Size, Style,
};

use std::sync::Arc;

use crate::Rgba;
use crate::paint::{
    BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize, Corners, Len, Shadow,
    parse_background_image, parse_background_position, parse_background_repeat,
    parse_background_size, parse_blur_filter, parse_radii, parse_shadows, split_top_level,
};

/// Paint-only properties (layout properties live in Taffy's `Style`).
#[derive(Clone, Debug, PartialEq)]
pub struct Visual {
    pub background: Rgba,
    /// Shared so building the draw list each frame doesn't copy gradient stops.
    pub background_image: Option<Arc<BackgroundImage>>,
    pub background_size: BackgroundSize,
    pub background_position: BackgroundPosition,
    pub background_repeat: BackgroundRepeat,
    pub border_color: Rgba,
    pub border_radius: Corners<Len>,
    pub shadows: Vec<Shadow>,
    /// `filter: blur(..)` radius; 0 means none.
    pub filter_blur: f32,
    /// `backdropFilter: blur(..)` radius; 0 means none.
    pub backdrop_blur: f32,
    pub opacity: f32,
}

impl Default for Visual {
    fn default() -> Self {
        Self {
            background: Rgba::TRANSPARENT,
            background_image: None,
            background_size: BackgroundSize::default(),
            background_position: BackgroundPosition::default(),
            background_repeat: BackgroundRepeat::default(),
            border_color: Rgba::TRANSPARENT,
            border_radius: Corners::all(Len::Px(0.0)),
            shadows: Vec::new(),
            filter_blur: 0.0,
            backdrop_blur: 0.0,
            opacity: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StyleError {
    #[error("unknown style property `{0}`")]
    UnknownProperty(String),
    #[error("invalid value {value:?} for `{prop}`")]
    InvalidValue { prop: String, value: Value },
}

/// Sets one property. `Value::Null` resets it to its default. On error nothing changes.
pub fn apply_style(
    style: &mut Style,
    visual: &mut Visual,
    prop: &str,
    value: &Value,
) -> Result<(), StyleError> {
    let invalid = || StyleError::InvalidValue {
        prop: prop.to_owned(),
        value: value.clone(),
    };
    if *value == Value::Null {
        return reset(style, visual, prop);
    }
    let d = |v: &Value| dimension(v).ok_or_else(invalid);
    let lpa = |v: &Value| length_percentage_auto(v).ok_or_else(invalid);
    let lp = |v: &Value| length_percentage(v).ok_or_else(invalid);
    let num = |v: &Value| v.as_f64().map(|n| n as f32).ok_or_else(invalid);
    let kw = || value.as_str().ok_or_else(invalid);

    match prop {
        "display" => style.display = parse_kw::<Display>(kw()?).ok_or_else(invalid)?,
        "position" => style.position = parse_kw::<Position>(kw()?).ok_or_else(invalid)?,

        "width" => style.size.width = d(value)?,
        "height" => style.size.height = d(value)?,
        "minWidth" => style.min_size.width = lpa(value)?,
        "minHeight" => style.min_size.height = lpa(value)?,
        "maxWidth" => style.max_size.width = lpa(value)?,
        "maxHeight" => style.max_size.height = lpa(value)?,
        "aspectRatio" => style.aspect_ratio = Some(num(value)?),

        "top" => style.inset.top = lpa(value)?,
        "right" => style.inset.right = lpa(value)?,
        "bottom" => style.inset.bottom = lpa(value)?,
        "left" => style.inset.left = lpa(value)?,
        "inset" => style.inset = sides(value, length_percentage_auto).ok_or_else(invalid)?,

        "margin" => style.margin = sides(value, length_percentage_auto).ok_or_else(invalid)?,
        "marginTop" => style.margin.top = lpa(value)?,
        "marginRight" => style.margin.right = lpa(value)?,
        "marginBottom" => style.margin.bottom = lpa(value)?,
        "marginLeft" => style.margin.left = lpa(value)?,

        "padding" => style.padding = sides(value, length_percentage).ok_or_else(invalid)?,
        "paddingTop" => style.padding.top = lp(value)?,
        "paddingRight" => style.padding.right = lp(value)?,
        "paddingBottom" => style.padding.bottom = lp(value)?,
        "paddingLeft" => style.padding.left = lp(value)?,

        "borderWidth" => style.border = sides(value, length_percentage).ok_or_else(invalid)?,
        "borderTopWidth" => style.border.top = lp(value)?,
        "borderRightWidth" => style.border.right = lp(value)?,
        "borderBottomWidth" => style.border.bottom = lp(value)?,
        "borderLeftWidth" => style.border.left = lp(value)?,

        "gap" => {
            // CSS order: row gap, then column gap.
            let [row, column] = pair(value, length_percentage).ok_or_else(invalid)?;
            style.gap = Size {
                width: column,
                height: row,
            };
        }
        "rowGap" => style.gap.height = lp(value)?,
        "columnGap" => style.gap.width = lp(value)?,

        "flexDirection" => {
            style.flex_direction = parse_kw::<FlexDirection>(kw()?).ok_or_else(invalid)?
        }
        "flexWrap" => style.flex_wrap = parse_kw::<FlexWrap>(kw()?).ok_or_else(invalid)?,
        "flexGrow" => style.flex_grow = num(value)?,
        "flexShrink" => style.flex_shrink = num(value)?,
        "flexBasis" => style.flex_basis = d(value)?,
        "flex" => {
            let (grow, shrink, basis) = flex_shorthand(value).ok_or_else(invalid)?;
            style.flex_grow = grow;
            style.flex_shrink = shrink;
            style.flex_basis = basis;
        }

        "alignItems" => style.align_items = parse_kw::<AlignItems>(kw()?).ok_or_else(invalid)?,
        "justifyItems" => {
            style.justify_items = parse_kw::<AlignItems>(kw()?).ok_or_else(invalid)?
        }
        "alignSelf" => style.align_self = align_self(kw()?).ok_or_else(invalid)?,
        "justifySelf" => style.justify_self = align_self(kw()?).ok_or_else(invalid)?,
        "alignContent" => {
            style.align_content = parse_kw::<AlignContent>(kw()?).ok_or_else(invalid)?
        }
        "justifyContent" => {
            style.justify_content = parse_kw::<AlignContent>(kw()?).ok_or_else(invalid)?
        }

        "gridTemplateColumns" => {
            let tracks = grid_template(kw()?).ok_or_else(invalid)?;
            style.grid_template_columns = tracks.tracks;
            style.grid_template_column_names = tracks.line_names;
        }
        "gridTemplateRows" => {
            let tracks = grid_template(kw()?).ok_or_else(invalid)?;
            style.grid_template_rows = tracks.tracks;
            style.grid_template_row_names = tracks.line_names;
        }
        "gridAutoFlow" => {
            style.grid_auto_flow = parse_kw::<GridAutoFlow>(kw()?).ok_or_else(invalid)?
        }
        "gridColumn" => style.grid_column = grid_line(value).ok_or_else(invalid)?,
        "gridRow" => style.grid_row = grid_line(value).ok_or_else(invalid)?,

        "backgroundColor" => visual.background = color(value).ok_or_else(invalid)?,
        "background" => {
            // Shorthand subset: a colour or a single image/gradient.
            let s = kw()?;
            if let Some(c) = Rgba::parse(s) {
                visual.background = c;
                visual.background_image = None;
            } else {
                visual.background_image =
                    parse_background_image(s).ok_or_else(invalid)?.map(Arc::new);
                visual.background = Rgba::TRANSPARENT;
            }
        }
        "backgroundImage" => {
            visual.background_image = parse_background_image(kw()?)
                .ok_or_else(invalid)?
                .map(Arc::new)
        }
        "backgroundSize" => {
            visual.background_size = parse_background_size(kw()?).ok_or_else(invalid)?
        }
        "backgroundPosition" => {
            visual.background_position = parse_background_position(kw()?).ok_or_else(invalid)?
        }
        "backgroundRepeat" => {
            visual.background_repeat = parse_background_repeat(kw()?).ok_or_else(invalid)?
        }
        "borderColor" => visual.border_color = color(value).ok_or_else(invalid)?,
        "border" => {
            let (width, color) = border_shorthand(value).ok_or_else(invalid)?;
            style.border = Rect {
                top: width,
                right: width,
                bottom: width,
                left: width,
            };
            visual.border_color = color;
        }
        "borderRadius" => {
            visual.border_radius = match value {
                Value::Number(n) if *n >= 0.0 => Corners::all(Len::Px(*n as f32)),
                Value::String(s) => parse_radii(s).ok_or_else(invalid)?,
                _ => return Err(invalid()),
            }
        }
        "borderTopLeftRadius" => {
            visual.border_radius.top_left = radius(value).ok_or_else(invalid)?
        }
        "borderTopRightRadius" => {
            visual.border_radius.top_right = radius(value).ok_or_else(invalid)?
        }
        "borderBottomRightRadius" => {
            visual.border_radius.bottom_right = radius(value).ok_or_else(invalid)?
        }
        "borderBottomLeftRadius" => {
            visual.border_radius.bottom_left = radius(value).ok_or_else(invalid)?
        }
        "boxShadow" => visual.shadows = parse_shadows(kw()?).ok_or_else(invalid)?,
        "filter" => visual.filter_blur = parse_blur_filter(kw()?).ok_or_else(invalid)?,
        "backdropFilter" => visual.backdrop_blur = parse_blur_filter(kw()?).ok_or_else(invalid)?,
        "opacity" => visual.opacity = num(value)?.clamp(0.0, 1.0),

        _ => return Err(StyleError::UnknownProperty(prop.to_owned())),
    }
    Ok(())
}

/// Resets `prop` to its default by applying the default style's value.
fn reset(style: &mut Style, visual: &mut Visual, prop: &str) -> Result<(), StyleError> {
    let d = Style::default();
    let v = Visual::default();
    match prop {
        "display" => style.display = d.display,
        "position" => style.position = d.position,
        "width" => style.size.width = d.size.width,
        "height" => style.size.height = d.size.height,
        "minWidth" => style.min_size.width = d.min_size.width,
        "minHeight" => style.min_size.height = d.min_size.height,
        "maxWidth" => style.max_size.width = d.max_size.width,
        "maxHeight" => style.max_size.height = d.max_size.height,
        "aspectRatio" => style.aspect_ratio = d.aspect_ratio,
        "top" => style.inset.top = d.inset.top,
        "right" => style.inset.right = d.inset.right,
        "bottom" => style.inset.bottom = d.inset.bottom,
        "left" => style.inset.left = d.inset.left,
        "inset" => style.inset = d.inset,
        "margin" => style.margin = d.margin,
        "marginTop" => style.margin.top = d.margin.top,
        "marginRight" => style.margin.right = d.margin.right,
        "marginBottom" => style.margin.bottom = d.margin.bottom,
        "marginLeft" => style.margin.left = d.margin.left,
        "padding" => style.padding = d.padding,
        "paddingTop" => style.padding.top = d.padding.top,
        "paddingRight" => style.padding.right = d.padding.right,
        "paddingBottom" => style.padding.bottom = d.padding.bottom,
        "paddingLeft" => style.padding.left = d.padding.left,
        "borderWidth" => style.border = d.border,
        "borderTopWidth" => style.border.top = d.border.top,
        "borderRightWidth" => style.border.right = d.border.right,
        "borderBottomWidth" => style.border.bottom = d.border.bottom,
        "borderLeftWidth" => style.border.left = d.border.left,
        "gap" => style.gap = d.gap,
        "rowGap" => style.gap.height = d.gap.height,
        "columnGap" => style.gap.width = d.gap.width,
        "flexDirection" => style.flex_direction = d.flex_direction,
        "flexWrap" => style.flex_wrap = d.flex_wrap,
        "flexGrow" => style.flex_grow = d.flex_grow,
        "flexShrink" => style.flex_shrink = d.flex_shrink,
        "flexBasis" => style.flex_basis = d.flex_basis,
        "flex" => {
            style.flex_grow = d.flex_grow;
            style.flex_shrink = d.flex_shrink;
            style.flex_basis = d.flex_basis;
        }
        "alignItems" => style.align_items = d.align_items,
        "justifyItems" => style.justify_items = d.justify_items,
        "alignSelf" => style.align_self = d.align_self,
        "justifySelf" => style.justify_self = d.justify_self,
        "alignContent" => style.align_content = d.align_content,
        "justifyContent" => style.justify_content = d.justify_content,
        "gridTemplateColumns" => {
            style.grid_template_columns = d.grid_template_columns;
            style.grid_template_column_names = d.grid_template_column_names;
        }
        "gridTemplateRows" => {
            style.grid_template_rows = d.grid_template_rows;
            style.grid_template_row_names = d.grid_template_row_names;
        }
        "gridAutoFlow" => style.grid_auto_flow = d.grid_auto_flow,
        "gridColumn" => style.grid_column = d.grid_column,
        "gridRow" => style.grid_row = d.grid_row,
        "backgroundColor" => visual.background = v.background,
        "borderColor" => visual.border_color = v.border_color,
        "background" => {
            visual.background = v.background;
            visual.background_image = v.background_image;
        }
        "backgroundImage" => visual.background_image = v.background_image,
        "backgroundSize" => visual.background_size = v.background_size,
        "backgroundPosition" => visual.background_position = v.background_position,
        "backgroundRepeat" => visual.background_repeat = v.background_repeat,
        "border" => {
            style.border = d.border;
            visual.border_color = v.border_color;
        }
        "borderRadius" => visual.border_radius = v.border_radius,
        "borderTopLeftRadius" => visual.border_radius.top_left = v.border_radius.top_left,
        "borderTopRightRadius" => visual.border_radius.top_right = v.border_radius.top_right,
        "borderBottomRightRadius" => {
            visual.border_radius.bottom_right = v.border_radius.bottom_right
        }
        "borderBottomLeftRadius" => visual.border_radius.bottom_left = v.border_radius.bottom_left,
        "boxShadow" => visual.shadows = v.shadows,
        "filter" => visual.filter_blur = v.filter_blur,
        "backdropFilter" => visual.backdrop_blur = v.backdrop_blur,
        "opacity" => visual.opacity = v.opacity,
        _ => return Err(StyleError::UnknownProperty(prop.to_owned())),
    }
    Ok(())
}

fn parse_kw<T: FromStr>(s: &str) -> Option<T> {
    s.trim().parse().ok()
}

fn dimension(v: &Value) -> Option<Dimension> {
    match v {
        Value::Number(n) => Some(Dimension::length(*n as f32)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn length_percentage_auto(v: &Value) -> Option<LengthPercentageAuto> {
    match v {
        Value::Number(n) => Some(LengthPercentageAuto::length(*n as f32)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn length_percentage(v: &Value) -> Option<LengthPercentage> {
    match v {
        Value::Number(n) => Some(LengthPercentage::length(*n as f32)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn color(v: &Value) -> Option<Rgba> {
    Rgba::parse(v.as_str()?)
}

/// One corner radius: a number (px), `"8px"` or `"50%"`.
fn radius(v: &Value) -> Option<Len> {
    let len = match v {
        Value::Number(n) => Len::Px(*n as f32),
        Value::String(s) => Len::parse(s)?,
        _ => return None,
    };
    match len {
        Len::Px(n) | Len::Percent(n) if n < 0.0 => None,
        ok => Some(ok),
    }
}

/// CSS `border` shorthand: width, style and colour in any order, e.g. `"2px solid #fff"`.
/// Like CSS, a missing style means `none` (no border) and a missing colour is black
/// (CSS uses `currentColor`, which arrives with text colours in M3). Only `solid`,
/// `none` and `hidden` styles are supported; other styles are rejected.
fn border_shorthand(v: &Value) -> Option<(LengthPercentage, Rgba)> {
    let s = v.as_str()?;
    let (mut width, mut solid, mut color) = (None, None, None);
    for token in split_top_level(s, ' ') {
        let w = match token {
            "thin" => Some(1.0),
            "medium" => Some(3.0),
            "thick" => Some(5.0),
            t => match Len::parse(t) {
                Some(Len::Px(px)) if px >= 0.0 => Some(px),
                _ => None,
            },
        };
        if let Some(w) = w
            && width.is_none()
        {
            width = Some(w);
        } else if matches!(token, "solid" | "none" | "hidden") && solid.is_none() {
            solid = Some(token == "solid");
        } else if color.is_none() {
            color = Some(Rgba::parse(token)?);
        } else {
            return None;
        }
    }
    let width = if solid == Some(true) {
        width.unwrap_or(3.0)
    } else {
        0.0
    };
    Some((
        LengthPercentage::length(width),
        color.unwrap_or(Rgba::new(0.0, 0.0, 0.0, 1.0)),
    ))
}

/// CSS box shorthand: one number, or 1–4 space-separated values (top right bottom left).
fn sides<T: Copy>(v: &Value, parse: fn(&Value) -> Option<T>) -> Option<Rect<T>> {
    let values: Vec<T> = match v {
        Value::Number(_) => vec![parse(v)?],
        Value::String(s) => s
            .split_whitespace()
            .map(|part| parse(&Value::String(part.to_owned())))
            .collect::<Option<_>>()?,
        _ => return None,
    };
    let (top, right, bottom, left) = match values[..] {
        [all] => (all, all, all, all),
        [vertical, horizontal] => (vertical, horizontal, vertical, horizontal),
        [top, horizontal, bottom] => (top, horizontal, bottom, horizontal),
        [top, right, bottom, left] => (top, right, bottom, left),
        _ => return None,
    };
    Some(Rect {
        top,
        right,
        bottom,
        left,
    })
}

/// One or two values (`gap: 10` or `gap: "10px 20px"`).
fn pair<T: Copy>(v: &Value, parse: fn(&Value) -> Option<T>) -> Option<[T; 2]> {
    let values: Vec<T> = match v {
        Value::Number(_) => vec![parse(v)?],
        Value::String(s) => s
            .split_whitespace()
            .map(|part| parse(&Value::String(part.to_owned())))
            .collect::<Option<_>>()?,
        _ => return None,
    };
    match values[..] {
        [both] => Some([both, both]),
        [first, second] => Some([first, second]),
        _ => None,
    }
}

/// `flex: <number>` (grow, shrink 1, basis 0), `"none"` or `"auto"`.
fn flex_shorthand(v: &Value) -> Option<(f32, f32, Dimension)> {
    match v {
        Value::Number(n) => Some((*n as f32, 1.0, Dimension::length(0.0))),
        Value::String(s) => match s.trim() {
            "none" => Some((0.0, 0.0, Dimension::auto())),
            "auto" => Some((1.0, 1.0, Dimension::auto())),
            other => other
                .parse::<f32>()
                .ok()
                .map(|n| (n, 1.0, Dimension::length(0.0))),
        },
        _ => None,
    }
}

/// `alignSelf`/`justifySelf`: `"auto"` means "inherit from the parent" (None).
fn align_self(s: &str) -> Option<Option<AlignItems>> {
    match s.trim() {
        "auto" => Some(None),
        other => other.parse().ok().map(Some),
    }
}

fn grid_template(s: &str) -> Option<GridTemplateTracks<String, GridTemplateComponent<String>>> {
    s.trim().parse().ok()
}

/// `gridColumn: 2`, `"2"`, `"1 / 3"`, `"span 2"`, `"2 / span 2"`.
fn grid_line(v: &Value) -> Option<Line<GridPlacement<String>>> {
    let placement = |s: &str| s.trim().parse::<GridPlacement<String>>().ok();
    match v {
        Value::Number(n) => Some(Line {
            start: placement(&n.to_string())?,
            end: GridPlacement::Auto,
        }),
        Value::String(s) => match s.split_once('/') {
            Some((start, end)) => Some(Line {
                start: placement(start)?,
                end: placement(end)?,
            }),
            None => Some(Line {
                start: placement(s)?,
                end: GridPlacement::Auto,
            }),
        },
        _ => None,
    }
}

// M2-STY-01
#[cfg(test)]
mod tests {
    use super::*;

    fn apply(prop: &str, value: Value) -> Result<(Style, Visual), StyleError> {
        let mut style = Style::default();
        let mut visual = Visual::default();
        apply_style(&mut style, &mut visual, prop, &value)?;
        Ok((style, visual))
    }

    #[test]
    fn numbers_are_pixels_and_strings_are_css() {
        assert_eq!(
            apply("width", 10.0.into()).unwrap().0.size.width,
            Dimension::length(10.0)
        );
        assert_eq!(
            apply("width", "50%".into()).unwrap().0.size.width,
            Dimension::percent(0.5)
        );
        assert_eq!(
            apply("width", "auto".into()).unwrap().0.size.width,
            Dimension::auto()
        );
        assert_eq!(
            apply("height", "12px".into()).unwrap().0.size.height,
            Dimension::length(12.0)
        );
    }

    #[test]
    fn keywords_parse_with_taffy() {
        let (s, _) = apply("justifyContent", "space-between".into()).unwrap();
        assert_eq!(s.justify_content, AlignContent::SPACE_BETWEEN);
        let (s, _) = apply("display", "grid".into()).unwrap();
        assert_eq!(s.display, Display::Grid);
        let (s, _) = apply("alignSelf", "auto".into()).unwrap();
        assert_eq!(s.align_self, None);
        let (s, _) = apply("flexDirection", "column".into()).unwrap();
        assert_eq!(s.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn grid_templates_and_lines() {
        let (s, _) = apply("gridTemplateColumns", "1fr 2fr".into()).unwrap();
        assert_eq!(s.grid_template_columns.len(), 2);
        let (s, _) = apply("gridColumn", "1 / 3".into()).unwrap();
        assert_eq!(s.grid_column.start, "1".parse().unwrap());
        assert_eq!(s.grid_column.end, "3".parse().unwrap());
        let (s, _) = apply("gridRow", "span 2".into()).unwrap();
        assert_eq!(s.grid_row.start, GridPlacement::Span(2));
    }

    #[test]
    fn shorthands_follow_css_order() {
        let (s, _) = apply("padding", "1px 2px 3px 4px".into()).unwrap();
        assert_eq!(
            (
                s.padding.top,
                s.padding.right,
                s.padding.bottom,
                s.padding.left
            ),
            (
                LengthPercentage::length(1.0),
                LengthPercentage::length(2.0),
                LengthPercentage::length(3.0),
                LengthPercentage::length(4.0)
            )
        );
        let (s, _) = apply("margin", 5.0.into()).unwrap();
        assert_eq!(s.margin.left, LengthPercentageAuto::length(5.0));
        let (s, _) = apply("gap", "10px 20px".into()).unwrap();
        assert_eq!(s.gap.height, LengthPercentage::length(10.0));
        assert_eq!(s.gap.width, LengthPercentage::length(20.0));
        let (s, _) = apply("flex", 2.0.into()).unwrap();
        assert_eq!(
            (s.flex_grow, s.flex_shrink, s.flex_basis),
            (2.0, 1.0, Dimension::length(0.0))
        );
    }

    #[test]
    fn visual_properties() {
        let (_, v) = apply("backgroundColor", "#ff0000".into()).unwrap();
        assert_eq!(v.background, Rgba::new(1.0, 0.0, 0.0, 1.0));
        let (_, v) = apply("borderRadius", "8px".into()).unwrap();
        assert_eq!(v.border_radius, Corners::all(Len::Px(8.0)));
        let (_, v) = apply("borderRadius", 6.0.into()).unwrap();
        assert_eq!(v.border_radius, Corners::all(Len::Px(6.0)));
        let (_, v) = apply("borderTopRightRadius", "50%".into()).unwrap();
        assert_eq!(v.border_radius.top_right, Len::Percent(0.5));
        assert_eq!(v.border_radius.top_left, Len::Px(0.0));
        let (_, v) = apply("boxShadow", "0 2px 4px black".into()).unwrap();
        assert_eq!(v.shadows.len(), 1);
        let (_, v) = apply("filter", "blur(3px)".into()).unwrap();
        assert_eq!(v.filter_blur, 3.0);
        let (_, v) = apply("backdropFilter", "blur(12px)".into()).unwrap();
        assert_eq!(v.backdrop_blur, 12.0);
        let (_, v) = apply("background", "linear-gradient(red, blue)".into()).unwrap();
        assert!(v.background_image.is_some());
        let (_, v) = apply("background", "teal".into()).unwrap();
        assert!(v.background_image.is_none());
        assert_eq!(v.background, Rgba::parse("teal").unwrap());
        let (_, v) = apply("opacity", 1.5.into()).unwrap();
        assert_eq!(v.opacity, 1.0, "opacity is clamped");
    }

    #[test]
    fn unknown_properties_and_bad_values_change_nothing() {
        let mut style = Style::default();
        let mut visual = Visual::default();
        apply_style(&mut style, &mut visual, "width", &10.0.into()).unwrap();
        apply_style(&mut style, &mut visual, "backgroundColor", &"red".into()).unwrap();

        assert_eq!(
            apply_style(&mut style, &mut visual, "colour", &"red".into()),
            Err(StyleError::UnknownProperty("colour".into()))
        );
        assert!(apply_style(&mut style, &mut visual, "width", &"ten".into()).is_err());
        assert!(apply_style(&mut style, &mut visual, "backgroundColor", &"#zz".into()).is_err());
        assert!(apply_style(&mut style, &mut visual, "flexGrow", &"lots".into()).is_err());

        assert_eq!(style.size.width, Dimension::length(10.0));
        assert_eq!(visual.background, Rgba::parse("red").unwrap());
    }

    #[test]
    fn null_resets_to_default() {
        let mut style = Style::default();
        let mut visual = Visual::default();
        apply_style(&mut style, &mut visual, "width", &10.0.into()).unwrap();
        apply_style(&mut style, &mut visual, "opacity", &0.5.into()).unwrap();
        apply_style(&mut style, &mut visual, "width", &Value::Null).unwrap();
        apply_style(&mut style, &mut visual, "opacity", &Value::Null).unwrap();
        assert_eq!(style.size.width, Style::<String>::default().size.width);
        assert_eq!(visual.opacity, 1.0);
    }

    // M2-VIS-02
    #[test]
    fn border_shorthand_sets_widths_and_colour() {
        let (s, v) = apply("border", "2px solid red".into()).unwrap();
        assert_eq!(s.border.left, LengthPercentage::length(2.0));
        assert_eq!(s.border.top, LengthPercentage::length(2.0));
        assert_eq!(v.border_color, Rgba::new(1.0, 0.0, 0.0, 1.0));

        // Any order; keyword widths; style without width is `medium` (3px).
        let (s, v) = apply("border", "#00f thick solid".into()).unwrap();
        assert_eq!(s.border.right, LengthPercentage::length(5.0));
        assert_eq!(v.border_color, Rgba::new(0.0, 0.0, 1.0, 1.0));
        let (s, _) = apply("border", "solid".into()).unwrap();
        assert_eq!(s.border.bottom, LengthPercentage::length(3.0));

        // Like CSS: `none`, or no style at all, means no border.
        let (s, _) = apply("border", "none".into()).unwrap();
        assert_eq!(s.border.top, LengthPercentage::length(0.0));
        let (s, _) = apply("border", "2px red".into()).unwrap();
        assert_eq!(s.border.top, LengthPercentage::length(0.0));

        // Unsupported styles and junk are rejected (and reported as warnings).
        for bad in ["2px dashed red", "2px solid red blue", "wide solid"] {
            assert!(apply("border", bad.into()).is_err(), "{bad:?}");
        }
    }
}
