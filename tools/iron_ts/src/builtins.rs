//! What the compiler itself gives a program, beyond the engine manifests: generic members it
//! lowers per use (`Random.pick<T>`), the typed window messages, the component base class, timers.
//! One entry per declaration (its TypeScript signature and doc); `types_dts.rs` prints them into
//! `types/iron.d.ts`, inside the generated class or namespace of the same owner. The lowering of
//! each lives in the compiler (named in the entry's comment).

/// Where a built-in declaration goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A static member of a generated class, or a member of a generated namespace. The signature
    /// is written as a class member (`pick<T>(..): T;`, `readonly roots: T;`); inside a namespace it
    /// becomes `function ..` / `const ..`.
    Static(&'static str),
}

/// One declaration.
#[derive(Clone, Copy, Debug)]
pub struct Builtin {
    pub place: Place,
    pub sig: &'static str,
    pub doc: &'static str,
}

pub const BUILTINS: &[Builtin] = &[
    // `lower/facade.rs` `lower_random_pick`: the item at `Random.rangeInt(0, length)`.
    Builtin {
        place: Place::Static("Random"),
        sig: "pick<T>(items: readonly T[]): T | undefined;",
        doc: "A random item from a list (`undefined` when empty).",
    },
];

/// The name a member signature declares (`pick<T>(..)` is `pick`, `readonly roots: T` is `roots`).
pub fn member_name(sig: &str) -> &str {
    let s = sig.trim_start();
    let s = s.strip_prefix("readonly ").unwrap_or(s);
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(s.len());
    &s[..end]
}

/// The built-ins of an owner's static side.
pub fn statics(owner: &str) -> impl Iterator<Item = &'static Builtin> + '_ {
    BUILTINS
        .iter()
        .filter(move |b| matches!(b.place, Place::Static(o) if o == owner))
}

/// A class member signature as a namespace member (`function pick<T>(..)`, `const roots: T`).
pub fn as_namespace_member(sig: &str) -> String {
    match sig.strip_prefix("readonly ") {
        Some(rest) => format!("const {rest}"),
        None if member_is_property(sig) => format!("let {sig}"),
        None => format!("function {sig}"),
    }
}

fn member_is_property(sig: &str) -> bool {
    let name = member_name(sig);
    sig.trim_start()[name.len()..].starts_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_signatures_become_namespace_members() {
        assert_eq!(member_name("pick<T>(a: T[]): T;"), "pick");
        assert_eq!(member_name("readonly roots: X[];"), "roots");
        assert_eq!(
            as_namespace_member("pick<T>(a: T[]): T;"),
            "function pick<T>(a: T[]): T;"
        );
        assert_eq!(
            as_namespace_member("readonly roots: X[];"),
            "const roots: X[];"
        );
        assert_eq!(as_namespace_member("count: number;"), "let count: number;");
    }
}
