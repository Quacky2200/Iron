//! Selector matching against the UI tree and the CSS cascade.

use std::fmt;

use iron_core::{NodeId, Value};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::context::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};
use selectors::matching::{ElementSelectorFlags, matches_selector};
use selectors::{Element, OpaqueElement};

use crate::css::{CssString, IronSelectors, PseudoClass, PseudoElement, Stylesheet};
use crate::layout::LayoutEngine;
use crate::tree::{ROOT, TEXT_NODE, UiTree};

/// A UI element as `selectors` sees it. Text nodes are never elements.
pub(crate) struct NodeRef<'a, L: LayoutEngine> {
    pub tree: &'a UiTree<L>,
    pub id: NodeId,
}

impl<L: LayoutEngine> Clone for NodeRef<'_, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: LayoutEngine> Copy for NodeRef<'_, L> {}

impl<L: LayoutEngine> fmt::Debug for NodeRef<'_, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{} {}>",
            self.tree.kind(self.id).unwrap_or("?"),
            self.id
        )
    }
}

impl<'a, L: LayoutEngine> NodeRef<'a, L> {
    fn at(&self, id: NodeId) -> Self {
        NodeRef {
            tree: self.tree,
            id,
        }
    }

    fn is_element(tree: &UiTree<L>, id: NodeId) -> bool {
        tree.kind(id).is_some_and(|k| k != TEXT_NODE) && !tree.is_generated(id)
    }

    fn sibling(&self, step: isize) -> Option<Self> {
        let parent = self.tree.parent(self.id)?;
        let siblings = self.tree.children(parent);
        let mut i = siblings.iter().position(|c| *c == self.id)? as isize;
        loop {
            i += step;
            let id = *siblings.get(usize::try_from(i).ok()?)?;
            if Self::is_element(self.tree, id) {
                return Some(self.at(id));
            }
        }
    }

    /// No element children, and text children are all empty (or all whitespace for `:blank`).
    fn is_empty_like(&self, allow_whitespace: bool) -> bool {
        self.tree.children(self.id).iter().all(|c| {
            if Self::is_element(self.tree, *c) {
                return false;
            }
            match self.tree.text(*c) {
                Some(t) if allow_whitespace => t.trim().is_empty(),
                Some(t) => t.is_empty(),
                None => true,
            }
        })
    }

    fn focus_within(&self) -> bool {
        let mut stack = vec![self.id];
        while let Some(id) = stack.pop() {
            if self.tree.state(id).focus {
                return true;
            }
            stack.extend_from_slice(self.tree.children(id));
        }
        false
    }
}

impl<'a, L: LayoutEngine> Element for NodeRef<'a, L> {
    type Impl = IronSelectors;

    fn opaque(&self) -> OpaqueElement {
        // Attributes live in a boxed node, so their address is stable and unique.
        match self.tree.attrs(self.id) {
            Some(attrs) => OpaqueElement::new(attrs),
            None => OpaqueElement::new(self.tree),
        }
    }

    fn parent_element(&self) -> Option<Self> {
        self.tree.parent(self.id).map(|p| self.at(p))
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.sibling(-1)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.sibling(1)
    }

    fn first_element_child(&self) -> Option<Self> {
        self.tree
            .children(self.id)
            .iter()
            .find(|c| Self::is_element(self.tree, **c))
            .map(|c| self.at(*c))
    }

    fn is_html_element_in_html_document(&self) -> bool {
        // Element names are matched case-insensitively, like HTML.
        true
    }

    fn has_local_name(&self, name: &CssString) -> bool {
        self.tree.kind(self.id) == Some(name.0.as_str())
    }

    fn has_namespace(&self, _ns: &CssString) -> bool {
        true
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.tree.kind(self.id) == other.tree.kind(other.id)
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&CssString>,
        local_name: &CssString,
        operation: &AttrSelectorOperation<&CssString>,
    ) -> bool {
        let Some(attrs) = self.tree.attrs(self.id) else {
            return false;
        };
        let class_list;
        let value = match local_name.0.as_str() {
            "class" if !attrs.classes.is_empty() => {
                class_list = attrs.classes.join(" ");
                Some(class_list.as_str())
            }
            "disabled" if attrs.disabled => Some(""),
            "checked" if attrs.checked => Some(""),
            name => attrs.get(name),
        };
        value.is_some_and(|v| operation.eval_str(v))
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &PseudoClass,
        _context: &mut MatchingContext<IronSelectors>,
    ) -> bool {
        let state = self.tree.state(self.id);
        let disabled = self.tree.attrs(self.id).is_some_and(|a| a.disabled);
        match pc {
            PseudoClass::Hover => state.hover,
            PseudoClass::Active => state.active,
            PseudoClass::Focus => state.focus,
            PseudoClass::FocusVisible => state.focus_visible,
            PseudoClass::FocusWithin => self.focus_within(),
            PseudoClass::Enabled => !disabled,
            PseudoClass::Disabled => disabled,
            PseudoClass::Checked => self.tree.attrs(self.id).is_some_and(|a| a.checked),
            PseudoClass::Blank => self.is_empty_like(true),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut MatchingContext<IronSelectors>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        false
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(&self, id: &CssString, case: CaseSensitivity) -> bool {
        self.tree
            .attrs(self.id)
            .and_then(|a| a.id.as_deref())
            .is_some_and(|v| case.eq(v.as_bytes(), id.0.as_bytes()))
    }

    fn has_class(&self, name: &CssString, case: CaseSensitivity) -> bool {
        self.tree.attrs(self.id).is_some_and(|a| {
            a.classes
                .iter()
                .any(|c| case.eq(c.as_bytes(), name.0.as_bytes()))
        })
    }

    fn has_custom_state(&self, _name: &CssString) -> bool {
        false
    }

    fn imported_part(&self, _name: &CssString) -> Option<CssString> {
        None
    }

    fn is_part(&self, _name: &CssString) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.is_empty_like(false)
    }

    fn is_root(&self) -> bool {
        self.id == ROOT
    }

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }
}

/// Which rules to match: the element itself or one of its pseudo-elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Element,
    Pseudo(PseudoElement),
}

/// A declaration that applies to an element, with its cascade position.
#[derive(Clone, Debug)]
pub(crate) struct Matched {
    pub specificity: u32,
    /// Stylesheet order, then rule order, then declaration order.
    pub order: (usize, usize, usize),
    pub prop: String,
    pub value: Value,
}

/// Collects the declarations of every rule in `sheets` that matches `id`,
/// sorted into cascade order (lowest priority first). Normal and `!important`
/// declarations are returned separately (inline styles go between them).
pub(crate) fn matched_declarations<L: LayoutEngine>(
    tree: &UiTree<L>,
    sheets: &[&Stylesheet],
    id: NodeId,
    target: Target,
    caches: &mut SelectorCaches,
) -> (Vec<Matched>, Vec<Matched>) {
    let element = NodeRef { tree, id };
    let mode = match target {
        Target::Element => MatchingMode::Normal,
        Target::Pseudo(_) => MatchingMode::ForStatelessPseudoElement,
    };
    let mut normal = Vec::new();
    let mut important = Vec::new();
    for (sheet_index, sheet) in sheets.iter().enumerate() {
        for (rule_index, rule) in sheet.rules.iter().enumerate() {
            let mut best = None;
            for selector in rule.selectors.slice() {
                let wanted = match target {
                    Target::Element => selector.pseudo_element().is_none(),
                    Target::Pseudo(p) => selector.pseudo_element() == Some(&p),
                };
                if !wanted {
                    continue;
                }
                let mut context = MatchingContext::new(
                    mode,
                    None,
                    caches,
                    QuirksMode::NoQuirks,
                    NeedsSelectorFlags::No,
                    MatchingForInvalidation::No,
                );
                if matches_selector(selector, 0, None, &element, &mut context) {
                    best = best.max(Some(selector.specificity()));
                }
            }
            let Some(specificity) = best else {
                continue;
            };
            for (decl_index, decl) in rule.declarations.iter().enumerate() {
                let m = Matched {
                    specificity,
                    order: (sheet_index, rule_index, decl_index),
                    prop: decl.prop.clone(),
                    value: css_value(&decl.value),
                };
                if decl.important {
                    important.push(m);
                } else {
                    normal.push(m);
                }
            }
        }
    }
    let key = |m: &Matched| (m.specificity, m.order);
    normal.sort_by_key(key);
    important.sort_by_key(key);
    (normal, important)
}

/// CSS text as an Iron style value: plain numbers become numbers (so `opacity: .5`
/// and `flex-grow: 1` work like their inline equivalents); everything else stays text.
pub(crate) fn css_value(text: &str) -> Value {
    match text.trim().parse::<f64>() {
        Ok(n) => Value::Number(n),
        Err(_) => Value::String(text.trim().to_owned()),
    }
}
