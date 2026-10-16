//! CSS stylesheets: parsing with `cssparser` and selectors with Servo's `selectors`
//! crate (DESIGN.md §5.2). Declarations are kept as CSS text and applied with the
//! same `apply_style` as inline styles, after kebab-case → camelCase.

use std::borrow::Borrow;
use std::fmt;

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser as CssParser, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, ToCss,
    match_ignore_ascii_case, serialize_identifier,
};
use precomputed_hash::PrecomputedHash;
use selectors::parser::{
    NonTSPseudoClass, ParseRelative, PseudoElement as PseudoElementTrait, SelectorImpl,
    SelectorParseErrorKind,
};
use selectors::{Parser as SelectorParser, SelectorList};

// --- Selector implementation -----------------------------------------------------

/// Our `selectors::SelectorImpl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IronSelectors;

/// A string used for identifiers, element names and attribute values.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CssString(pub String);

impl ToCss for CssString {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        serialize_identifier(&self.0, dest)
    }
}

impl From<&str> for CssString {
    fn from(s: &str) -> Self {
        CssString(s.to_owned())
    }
}

impl From<String> for CssString {
    fn from(s: String) -> Self {
        CssString(s)
    }
}

impl AsRef<str> for CssString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PrecomputedHash for CssString {
    fn precomputed_hash(&self) -> u32 {
        // Must depend only on the contents (used by the ancestor bloom filter).
        self.0.bytes().fold(0x811c_9dc5_u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        })
    }
}

/// Pseudo-classes beyond the tree-structural ones `selectors` handles itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PseudoClass {
    Hover,
    Active,
    Focus,
    FocusVisible,
    FocusWithin,
    Enabled,
    Disabled,
    Checked,
    /// Selectors Level 4 `:blank`: no element children and only whitespace text.
    Blank,
}

impl ToCss for PseudoClass {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(match self {
            PseudoClass::Hover => ":hover",
            PseudoClass::Active => ":active",
            PseudoClass::Focus => ":focus",
            PseudoClass::FocusVisible => ":focus-visible",
            PseudoClass::FocusWithin => ":focus-within",
            PseudoClass::Enabled => ":enabled",
            PseudoClass::Disabled => ":disabled",
            PseudoClass::Checked => ":checked",
            PseudoClass::Blank => ":blank",
        })
    }
}

impl NonTSPseudoClass for PseudoClass {
    fn is_active_or_hover(&self) -> bool {
        matches!(self, PseudoClass::Active | PseudoClass::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(
            self,
            PseudoClass::Active
                | PseudoClass::Hover
                | PseudoClass::Focus
                | PseudoClass::FocusVisible
                | PseudoClass::FocusWithin
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    Before,
    After,
}

impl ToCss for PseudoElement {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(match self {
            PseudoElement::Before => "::before",
            PseudoElement::After => "::after",
        })
    }
}

impl PseudoElementTrait for PseudoElement {
    fn is_before_or_after(&self) -> bool {
        true
    }
}

impl SelectorImpl for IronSelectors {
    type ExtraMatchingData<'a> = std::marker::PhantomData<&'a ()>;
    type AttrValue = CssString;
    type Identifier = CssString;
    type LocalName = CssString;
    type NamespaceUrl = CssString;
    type NamespacePrefix = CssString;
    type BorrowedLocalName = CssString;
    type BorrowedNamespaceUrl = CssString;
    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;
}

impl Borrow<str> for CssString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

struct IronSelectorParser;

impl<'i> SelectorParser<'i> for IronSelectorParser {
    type Impl = IronSelectors;
    type Error = SelectorParseErrorKind;

    fn parse_nth_child_of(&self) -> bool {
        true
    }

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }

    fn parse_non_ts_pseudo_class(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoClass, ParseError<Self::Error>> {
        let class = match_ignore_ascii_case! { &name,
            "hover" => PseudoClass::Hover,
            "active" => PseudoClass::Active,
            "focus" => PseudoClass::Focus,
            "focus-visible" => PseudoClass::FocusVisible,
            "focus-within" => PseudoClass::FocusWithin,
            "enabled" => PseudoClass::Enabled,
            "disabled" => PseudoClass::Disabled,
            "checked" => PseudoClass::Checked,
            "blank" => PseudoClass::Blank,
            _ => return Err(ParseError::custom(
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            )),
        };
        Ok(class)
    }

    fn parse_pseudo_element(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoElement, ParseError<Self::Error>> {
        match_ignore_ascii_case! { &name,
            "before" => Ok(PseudoElement::Before),
            "after" => Ok(PseudoElement::After),
            _ => Err(ParseError::custom(
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            )),
        }
    }
}

/// Parses a selector list such as `.card > text:first-child, #main::before`.
pub fn parse_selectors(text: &str) -> Result<SelectorList<IronSelectors>, String> {
    let mut parser = CssParser::new(text);
    SelectorList::parse(&IronSelectorParser, &mut parser, ParseRelative::No)
        .map_err(|e| format!("invalid selector `{text}`: {:?}", e.kind))
}

// --- Stylesheets -------------------------------------------------------------------

/// One `prop: value` declaration. `prop` is camelCase (custom properties keep
/// their `--name`); `value` is the CSS text, e.g. `"1px solid red"`.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub prop: String,
    pub value: String,
    pub important: bool,
}

#[derive(Clone, Debug)]
pub struct StyleRule {
    pub selectors: SelectorList<IronSelectors>,
    pub declarations: Vec<Declaration>,
}

/// An `@font-face` rule, loaded by the engine like `loadFont()`.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
    pub family: String,
    /// The first `url(...)` in `src`, relative to the game folder.
    pub src: String,
    pub weight: Option<f32>,
    pub italic: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
    pub font_faces: Vec<FontFace>,
    /// Problems found while parsing (invalid selectors, unsupported at-rules, ...).
    /// The rest of the sheet still applies.
    pub warnings: Vec<String>,
}

enum Item {
    Rule(StyleRule),
    FontFace(FontFace),
    Skipped(String),
}

enum AtPrelude {
    FontFace,
    Media(String),
    Unsupported(String),
}

struct SheetParser;

impl<'i> QualifiedRuleParser<'i> for SheetParser {
    type Prelude = SelectorList<IronSelectors>;
    type QualifiedRule = Item;
    type Error = SelectorParseErrorKind;

    fn parse_prelude(
        &mut self,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        SelectorList::parse(&IronSelectorParser, input, ParseRelative::No)
    }

    fn parse_block(
        &mut self,
        selectors: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Item, ParseError<Self::Error>> {
        Ok(Item::Rule(StyleRule {
            selectors,
            declarations: parse_declarations(input),
        }))
    }
}

impl<'i> AtRuleParser<'i> for SheetParser {
    type Prelude = AtPrelude;
    type AtRule = Item;
    type Error = SelectorParseErrorKind;

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
    ) -> Result<AtPrelude, ParseError<Self::Error>> {
        let start = input.position();
        while input.next().is_ok() {}
        let rest = input.slice_from(start).trim().to_owned();
        Ok(match_ignore_ascii_case! { &name,
            "font-face" => AtPrelude::FontFace,
            "media" => AtPrelude::Media(rest),
            _ => AtPrelude::Unsupported(name.to_string()),
        })
    }

    fn rule_without_block(&mut self, prelude: AtPrelude, _start: &ParserState) -> Result<Item, ()> {
        Ok(Item::Skipped(match prelude {
            AtPrelude::Unsupported(name) => format!("@{name} isn't supported; skipped"),
            _ => "at-rule without a block; skipped".into(),
        }))
    }

    fn parse_block(
        &mut self,
        prelude: AtPrelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Item, ParseError<Self::Error>> {
        match prelude {
            AtPrelude::FontFace => {
                let decls = parse_declarations(input);
                let get = |p: &str| decls.iter().find(|d| d.prop == p).map(|d| d.value.as_str());
                let family = get("fontFamily").map(unquote);
                let src = get("src").and_then(first_url);
                match (family, src) {
                    (Some(family), Some(src)) => Ok(Item::FontFace(FontFace {
                        family,
                        src,
                        weight: get("fontWeight").and_then(|w| match w {
                            "normal" => Some(400.0),
                            "bold" => Some(700.0),
                            other => other.parse().ok(),
                        }),
                        italic: get("fontStyle").map(|s| s == "italic" || s == "oblique"),
                    })),
                    _ => Ok(Item::Skipped(
                        "@font-face needs `font-family` and a `src: url(...)`".into(),
                    )),
                }
            }
            AtPrelude::Media(query) => {
                // Kept for later: @media isn't supported yet (TODO.md, "Later").
                while input.next().is_ok() {}
                Ok(Item::Skipped(format!(
                    "@media ({query}) isn't supported yet; its rules are ignored"
                )))
            }
            AtPrelude::Unsupported(name) => {
                while input.next().is_ok() {}
                Ok(Item::Skipped(format!("@{name} isn't supported; skipped")))
            }
        }
    }
}

/// Parses `prop: value [!important];` declarations in a rule block.
fn parse_declarations(input: &mut CssParser<'_>) -> Vec<Declaration> {
    let mut parser = DeclParser;
    RuleBodyParser::new(input, &mut parser)
        .filter_map(Result::ok)
        .collect()
}

struct DeclParser;

impl<'i> DeclarationParser<'i> for DeclParser {
    type Declaration = Declaration;
    type Error = ();

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        _start: &ParserState,
    ) -> Result<Declaration, ParseError<()>> {
        let start = input.position();
        while input.next_including_whitespace_and_comments().is_ok() {}
        let mut value = input.slice_from(start).trim();
        let mut important = false;
        if let Some(bang) = value.rfind('!')
            && value[bang + 1..].trim().eq_ignore_ascii_case("important")
        {
            important = true;
            value = value[..bang].trim();
        }
        Ok(Declaration {
            prop: camel_case(&name),
            value: value.to_owned(),
            important,
        })
    }
}

impl<'i> AtRuleParser<'i> for DeclParser {
    type Prelude = ();
    type AtRule = Declaration;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for DeclParser {
    type Prelude = ();
    type QualifiedRule = Declaration;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Declaration, ()> for DeclParser {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false
    }
}

/// `background-color` → `backgroundColor`; custom properties (`--x`) are unchanged.
pub fn camel_case(prop: &str) -> String {
    if prop.starts_with("--") {
        return prop.to_owned();
    }
    let mut out = String::with_capacity(prop.len());
    let mut upper = false;
    for c in prop.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(['"', '\'']).to_owned()
}

/// The first `url(...)` in an `@font-face` `src` list.
fn first_url(src: &str) -> Option<String> {
    let start = src.find("url(")? + 4;
    let end = src[start..].find(')')? + start;
    let url = unquote(&src[start..end]);
    (!url.is_empty()).then_some(url)
}

/// Parses a stylesheet. Invalid rules are skipped with a warning; the rest applies.
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    let mut input = CssParser::new(css);
    let mut parser = SheetParser;
    let mut sheet = Stylesheet::default();
    for item in StyleSheetParser::new(&mut input, &mut parser) {
        match item {
            Ok(Item::Rule(rule)) => sheet.rules.push(rule),
            Ok(Item::FontFace(face)) => sheet.font_faces.push(face),
            Ok(Item::Skipped(why)) => sheet.warnings.push(why),
            Err((error, text, location)) => sheet.warnings.push(format!(
                "line {}: skipped `{}` ({:?})",
                location.line + 1,
                text.trim(),
                error.kind
            )),
        }
    }
    sheet
}

// M3-CSS-01
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rules_and_declarations() {
        let sheet = parse_stylesheet(
            r#"
            /* a comment */
            .card, #main > text { background-color: #fff; padding: 4px 8px !important; }
            box:first-child::before { content: "x"; --accent: red; }
            "#,
        );
        assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].selectors.slice().len(), 2);
        assert_eq!(
            sheet.rules[0].declarations,
            [
                Declaration {
                    prop: "backgroundColor".into(),
                    value: "#fff".into(),
                    important: false
                },
                Declaration {
                    prop: "padding".into(),
                    value: "4px 8px".into(),
                    important: true
                },
            ]
        );
        assert_eq!(sheet.rules[1].declarations[0].value, "\"x\"");
        assert_eq!(sheet.rules[1].declarations[1].prop, "--accent");
    }

    #[test]
    fn parses_font_faces() {
        let sheet = parse_stylesheet(
            r#"@font-face { font-family: "Charm"; src: url("fonts/Charm-Bold.ttf") format("truetype"); font-weight: bold; }"#,
        );
        assert_eq!(
            sheet.font_faces,
            [FontFace {
                family: "Charm".into(),
                src: "fonts/Charm-Bold.ttf".into(),
                weight: Some(700.0),
                italic: None
            }]
        );
    }

    #[test]
    fn bad_rules_warn_and_the_rest_still_applies() {
        let sheet = parse_stylesheet(
            r#"
            .ok { width: 10px; }
            .broken:no-such-pseudo { width: 1px; }
            @media (min-width: 600px) { .ok { width: 20px; } }
            @keyframes spin { from { opacity: 0 } }
            .also-ok { height: 5px }
            "#,
        );
        assert_eq!(sheet.rules.len(), 2, "{:?}", sheet.warnings);
        assert_eq!(sheet.warnings.len(), 3, "{:?}", sheet.warnings);
        assert!(sheet.warnings.iter().any(|w| w.contains("@media")));
    }

    #[test]
    fn selectors_cover_the_planned_set() {
        for ok in [
            "*",
            "box",
            ".a.b",
            "#id",
            "[data-kind]",
            r#"[data-kind="enemy"]"#,
            "a > b + c ~ d e",
            ":first-child, :last-child, :only-child, :nth-child(2n+1), :nth-child(odd of .x)",
            ":nth-last-child(2), :first-of-type, :last-of-type, :nth-of-type(3), :root, :empty, :blank",
            ":not(.a), :is(.a, .b), :where(.c), :has(> .child), :has(+ .sib)",
            ":hover, :active, :focus, :focus-visible, :focus-within, :enabled, :disabled, :checked",
            ".x::before, .y::after",
        ] {
            assert!(
                parse_selectors(ok).is_ok(),
                "{ok}: {:?}",
                parse_selectors(ok).err()
            );
        }
        assert!(parse_selectors(":visited-later").is_err());
        assert!(parse_selectors("::marker").is_err());
    }

    #[test]
    fn camel_cases_property_names() {
        assert_eq!(camel_case("background-color"), "backgroundColor");
        assert_eq!(camel_case("border-top-left-radius"), "borderTopLeftRadius");
        assert_eq!(camel_case("--my-var"), "--my-var");
        assert_eq!(camel_case("width"), "width");
    }
}
