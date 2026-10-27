// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The elements a script can create: `<box>`, `<text>`, ... and the capitalised components
//! (`Text`, `Span`, ...) that stand for them.

/// An element type of the UI tree.
#[derive(Clone, Copy, Debug)]
pub struct Element {
    /// The tag (`<box>`).
    pub tag: &'static str,
    /// The component function that creates it (`Text` for `<text>`); `None` when there is none.
    pub component: Option<&'static str>,
    pub doc: &'static str,
}

/// The built-in elements, in the order `JSX.IntrinsicElements` lists them.
pub const ELEMENTS: &[Element] = &[
    Element {
        tag: "box",
        component: None,
        doc: "A rectangle that lays out its children (a `div`).",
    },
    Element {
        tag: "text",
        component: Some("Text"),
        doc: "A block of text.",
    },
    Element {
        tag: "span",
        component: Some("Span"),
        doc: "Inline text inside a `text`.",
    },
    Element {
        tag: "input",
        component: Some("Input"),
        doc: "A single-line text field.",
    },
    Element {
        tag: "textarea",
        component: Some("TextArea"),
        doc: "A multi-line text field.",
    },
    Element {
        tag: "image",
        component: Some("Image"),
        doc: "A picture (`src`: a path in the game folder or a `data:` URL).",
    },
    Element {
        tag: "list",
        component: Some("List"),
        doc: "A list with markers (`list-style`).",
    },
    Element {
        tag: "dialog",
        component: Some("Dialog"),
        doc: "A dialog; `modal` puts it in the top layer with a `::backdrop`.",
    },
    Element {
        tag: "svg",
        component: None,
        doc: "Inline SVG; the shapes inside are the SVG elements of [`SVG_KINDS`].",
    },
];

/// SVG element types (besides `text`, which is SVG text inside an `svg`).
pub const SVG_KINDS: &[&str] = &[
    "svg",
    "g",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "path",
    "tspan",
    "defs",
    "use",
    "symbol",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "mask",
    "pattern",
    "title",
];

/// The tag a capitalised component name stands for.
pub fn tag_of_component(name: &str) -> Option<&'static str> {
    ELEMENTS
        .iter()
        .find(|e| e.component == Some(name))
        .map(|e| e.tag)
}
