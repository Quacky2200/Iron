//! Inline SVG (M3d step 5.1b): an `svg` element is a box; the SVG elements inside it
//! are tree nodes (so selectors, inheritance of styles and transitions work), which
//! the tree serializes back to SVG markup with their computed presentation properties
//! when drawing. The renderer rasterizes that markup at the drawn size (resvg).

/// SVG element types (besides `text`, which is SVG text inside an `svg`).
pub const SVG_KINDS: &[&str] = crate::tables::elements::SVG_KINDS;

pub fn is_svg_kind(kind: &str) -> bool {
    SVG_KINDS.contains(&kind)
}

/// CSS properties that only SVG uses (presentation attributes and geometry). Stored
/// as written and emitted on the element.
const SVG_PROPS: &[&str] = &[
    "fill",
    "fillOpacity",
    "fillRule",
    "stroke",
    "strokeWidth",
    "strokeOpacity",
    "strokeLinecap",
    "strokeLinejoin",
    "strokeDasharray",
    "strokeDashoffset",
    "strokeMiterlimit",
    "stopColor",
    "stopOpacity",
    "clipRule",
    "markerStart",
    "markerMid",
    "markerEnd",
    "paintOrder",
    "vectorEffect",
    "shapeRendering",
    "textAnchor",
    "dominantBaseline",
    "floodColor",
    "floodOpacity",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "x",
    "y",
    "d",
];

/// Ordinary CSS properties that also mean something in SVG: kept as written too, so
/// they can be emitted as presentation attributes.
const MIRRORED: &[&str] = &[
    "opacity",
    "transform",
    "transformOrigin",
    "visibility",
    "display",
    "clipPath",
    "mask",
    "fontSize",
    "fontWeight",
    "fontStyle",
    "letterSpacing",
];

pub fn is_svg_prop(prop: &str) -> bool {
    SVG_PROPS.contains(&prop)
}

/// Mirrored properties the `svg` element's own box already applies (its layer, transform,
/// visibility, clip and mask): written on the root they would apply twice.
pub fn box_applies(prop: &str) -> bool {
    matches!(
        prop,
        "opacity"
            | "transform"
            | "transformOrigin"
            | "visibility"
            | "display"
            | "clipPath"
            | "mask"
    )
}

/// Whether `prop`'s written value is kept for SVG output.
pub fn keeps_raw(prop: &str) -> bool {
    is_svg_prop(prop) || MIRRORED.contains(&prop)
}

/// Attribute names SVG spells in camelCase; others are kebab-case (`strokeWidth` →
/// `stroke-width`), as React DOM writes them.
const CAMEL: &[&str] = &[
    "viewBox",
    "preserveAspectRatio",
    "gradientUnits",
    "gradientTransform",
    "patternUnits",
    "patternContentUnits",
    "patternTransform",
    "clipPathUnits",
    "maskUnits",
    "maskContentUnits",
    "textLength",
    "lengthAdjust",
    "markerWidth",
    "markerHeight",
    "markerUnits",
    "refX",
    "refY",
    "startOffset",
    "spreadMethod",
    "pathLength",
    "stdDeviation",
];

/// The SVG attribute name for a React prop or CSS property name.
pub fn attr_name(name: &str) -> String {
    match name {
        "className" => return "class".into(),
        "xlinkHref" => return "href".into(),
        n if CAMEL.contains(&n) => return n.into(),
        _ => {}
    }
    crate::motion::kebab_case(name)
}

/// A CSS `transform` in SVG attribute syntax: units dropped (angles in degrees,
/// lengths in user units), `rotateZ`/`translateX`/... spelled with their 2D forms.
/// Percentages and 3D functions have no SVG form and are left as written.
pub fn svg_transform(css: &str) -> String {
    let mut out = String::new();
    for f in css.split_inclusive(')') {
        let Some((name, args)) = f.trim().strip_suffix(')').and_then(|f| f.split_once('(')) else {
            out.push_str(f);
            continue;
        };
        let nums: Vec<String> = args
            .split([',', ' '])
            .filter(|a| !a.is_empty())
            .map(|a| {
                let a = a.trim();
                let scaled = |n: &str, k: f64| n.trim().parse::<f64>().map(|v| (v * k).to_string());
                let v = if let Some(n) = a.strip_suffix("deg") {
                    scaled(n, 1.0)
                } else if let Some(n) = a.strip_suffix("grad") {
                    scaled(n, 0.9)
                } else if let Some(n) = a.strip_suffix("rad") {
                    scaled(n, 180.0 / std::f64::consts::PI)
                } else if let Some(n) = a.strip_suffix("turn") {
                    scaled(n, 360.0)
                } else if let Some(n) = a.strip_suffix("px") {
                    scaled(n, 1.0)
                } else {
                    Ok(a.to_owned())
                };
                v.unwrap_or_else(|_| a.to_owned())
            })
            .collect();
        let name = name.trim();
        let (name, nums) = match (name, &nums[..]) {
            ("rotateZ", _) => ("rotate", nums.clone()),
            ("translateX", [x]) => ("translate", vec![x.clone(), "0".into()]),
            ("translateY", [y]) => ("translate", vec!["0".into(), y.clone()]),
            ("scaleX", [x]) => ("scale", vec![x.clone(), "1".into()]),
            ("scaleY", [y]) => ("scale", vec!["1".into(), y.clone()]),
            _ => (name, nums.clone()),
        };
        out.push_str(&format!("{name}({}) ", nums.join(" ")));
    }
    out.trim().to_owned()
}

/// Escapes text for an attribute value or element content.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Markup as a `data:` URL (only `%` needs escaping for Iron's decoder).
pub fn data_url(markup: &str) -> String {
    format!("data:image/svg+xml,{}", markup.replace('%', "%25"))
}

/// An `svg` element's natural size from its `width` / `height` attributes and
/// `viewBox`: one known side keeps the viewBox's aspect ratio; with neither,
/// browsers' 300 × 150.
pub fn natural_size(attrs: &[(String, String)]) -> (f32, f32) {
    let get = |n: &str| {
        attrs
            .iter()
            .rev()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.as_str())
    };
    let len = |n: &str| {
        get(n)
            .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
    };
    let aspect = get("viewBox").and_then(|v| {
        let n: Vec<f32> = v
            .split([' ', ','])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        (n.len() == 4 && n[2] > 0.0 && n[3] > 0.0).then(|| (n[2], n[3]))
    });
    match (len("width"), len("height"), aspect) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some((vw, vh))) => (w, w * vh / vw),
        (None, Some(h), Some((vw, vh))) => (h * vw / vh, h),
        (None, None, Some((vw, vh))) => (vw, vh),
        (w, h, None) => (w.unwrap_or(300.0), h.unwrap_or(150.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M3-SVG-02
    #[test]
    fn names_sizes_and_escaping() {
        assert_eq!(attr_name("strokeWidth"), "stroke-width");
        assert_eq!(attr_name("viewBox"), "viewBox");
        assert_eq!(attr_name("className"), "class");
        assert_eq!(attr_name("x1"), "x1");
        assert_eq!(escape(r#"a<b & "c""#), "a&lt;b &amp; &quot;c&quot;");
        let a = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            natural_size(&a(&[("width", "40"), ("height", "20px")])),
            (40.0, 20.0)
        );
        assert_eq!(
            natural_size(&a(&[("width", "50"), ("viewBox", "0 0 10 20")])),
            (50.0, 100.0)
        );
        assert_eq!(natural_size(&a(&[("viewBox", "0,0,24,24")])), (24.0, 24.0));
        assert_eq!(natural_size(&[]), (300.0, 150.0));
        assert_eq!(
            data_url("<svg>50%</svg>"),
            "data:image/svg+xml,<svg>50%25</svg>"
        );
        assert_eq!(
            svg_transform("rotate(15deg) translateX(10px) scale(2, 1.5) rotate(0.5turn)"),
            "rotate(15) translate(10 0) scale(2 1.5) rotate(180)"
        );
    }
}
