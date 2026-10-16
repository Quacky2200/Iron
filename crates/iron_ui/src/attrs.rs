//! Element attributes and interaction state, as seen by CSS selectors.

use iron_core::Value;

/// Attributes from React props: `className`, `id`, `disabled`, `checked`, `tabIndex`, `data-*`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs {
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub disabled: bool,
    pub checked: bool,
    /// `tabIndex`: focusable when set; negative values are skipped by Tab.
    pub tab_index: Option<i32>,
    /// `data-*` attributes, by full name (`data-kind`), in insertion order.
    pub data: Vec<(String, String)>,
}

impl Attrs {
    /// Sets one attribute; `Value::Null` removes it. Returns an error for unknown names.
    pub fn set(&mut self, name: &str, value: &Value) -> Result<(), String> {
        let text = || match value {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) if n.fract() == 0.0 => Some(format!("{}", *n as i64)),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        };
        let truthy = match value {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty() && s != "false",
            Value::Array(_) => true,
        };
        match name {
            "id" => self.id = text().filter(|s| !s.is_empty()),
            "className" => {
                self.classes = text()
                    .map(|s| s.split_whitespace().map(str::to_owned).collect())
                    .unwrap_or_default()
            }
            "disabled" => self.disabled = truthy,
            "checked" => self.checked = truthy,
            "tabIndex" => {
                self.tab_index = match value {
                    Value::Null => None,
                    Value::Number(n) => Some(*n as i32),
                    Value::String(s) => Some(
                        s.trim()
                            .parse()
                            .map_err(|_| format!("invalid tabIndex {s:?}"))?,
                    ),
                    _ => return Err(format!("invalid tabIndex {value:?}")),
                }
            }
            n if n.starts_with("data-") => {
                self.data.retain(|(k, _)| k != n);
                if let Some(v) = text() {
                    self.data.push((n.to_owned(), v));
                }
            }
            other => return Err(format!("unknown attribute `{other}`")),
        }
        Ok(())
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == class)
    }

    /// An attribute's value as selectors see it (`[data-kind="x"]`, `[id]`, ...).
    pub fn get(&self, name: &str) -> Option<&str> {
        match name {
            "id" => self.id.as_deref(),
            "class" => None,
            n => self
                .data
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, v)| v.as_str()),
        }
    }
}

/// Interaction state for `:hover`, `:active` and `:focus*`, set by the engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElementState {
    pub hover: bool,
    pub active: bool,
    pub focus: bool,
    /// Focus that should show a focus ring (keyboard navigation).
    pub focus_visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_and_removes_attributes() {
        let mut a = Attrs::default();
        a.set("className", &"card  primary".into()).unwrap();
        a.set("id", &"main".into()).unwrap();
        a.set("data-kind", &"enemy".into()).unwrap();
        a.set("data-level", &3.0.into()).unwrap();
        a.set("disabled", &true.into()).unwrap();
        assert!(a.has_class("card") && a.has_class("primary") && !a.has_class("car"));
        assert_eq!(a.get("id"), Some("main"));
        assert_eq!(a.get("data-kind"), Some("enemy"));
        assert_eq!(a.get("data-level"), Some("3"));
        assert!(a.disabled && !a.checked);

        a.set("data-kind", &Value::Null).unwrap();
        a.set("className", &Value::Null).unwrap();
        a.set("disabled", &Value::Null).unwrap();
        assert_eq!(a.get("data-kind"), None);
        assert!(a.classes.is_empty() && !a.disabled);
        assert!(a.set("onClick", &"x".into()).is_err());
    }
}
