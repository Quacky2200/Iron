// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! SPDX licence expressions (`MIT OR Apache-2.0`, `(MIT OR Apache-2.0) AND Unicode-3.0`), and the
//! old `MIT/Apache-2.0` form, as their alternatives.

/// The alternatives of an expression: the crate may be used under any one of them, and each is
/// the licences that all apply to it (`AND`). `WITH` stays part of its licence's name.
pub fn alternatives(expr: &str) -> Result<Vec<Vec<String>>, String> {
    let spaced = expr
        .replace('/', " OR ")
        .replace('(', " ( ")
        .replace(')', " ) ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut pos = 0;
    let alts = or_expr(&tokens, &mut pos)?;
    match tokens.get(pos) {
        None => Ok(alts),
        Some(t) => Err(format!("unexpected `{t}` in `{expr}`")),
    }
}

fn is(token: Option<&&str>, word: &str) -> bool {
    token.is_some_and(|t| t.eq_ignore_ascii_case(word))
}

fn or_expr(t: &[&str], pos: &mut usize) -> Result<Vec<Vec<String>>, String> {
    let mut alts = and_expr(t, pos)?;
    while is(t.get(*pos), "OR") {
        *pos += 1;
        for a in and_expr(t, pos)? {
            if !alts.contains(&a) {
                alts.push(a);
            }
        }
    }
    Ok(alts)
}

fn and_expr(t: &[&str], pos: &mut usize) -> Result<Vec<Vec<String>>, String> {
    let mut alts = atom(t, pos)?;
    while is(t.get(*pos), "AND") {
        *pos += 1;
        let right = atom(t, pos)?;
        let mut product = Vec::new();
        for a in &alts {
            for b in &right {
                let mut both = a.clone();
                for l in b {
                    if !both.contains(l) {
                        both.push(l.clone());
                    }
                }
                product.push(both);
            }
        }
        alts = product;
    }
    Ok(alts)
}

fn atom(t: &[&str], pos: &mut usize) -> Result<Vec<Vec<String>>, String> {
    match t.get(*pos) {
        Some(&"(") => {
            *pos += 1;
            let inner = or_expr(t, pos)?;
            if t.get(*pos) != Some(&")") {
                return Err("a `(` is not closed".into());
            }
            *pos += 1;
            Ok(inner)
        }
        Some(id)
            if !["OR", "AND", "WITH", ")"]
                .iter()
                .any(|k| id.eq_ignore_ascii_case(k)) =>
        {
            *pos += 1;
            let mut id = id.to_string();
            if is(t.get(*pos), "WITH") {
                let exception = t.get(*pos + 1).ok_or("`WITH` needs an exception")?;
                id = format!("{id} WITH {exception}");
                *pos += 2;
            }
            Ok(vec![vec![id]])
        }
        Some(other) => Err(format!("unexpected `{other}`")),
        None => Err("the expression ends too soon".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alts(e: &str) -> Vec<Vec<String>> {
        alternatives(e).unwrap()
    }

    // M9-LIC-03
    #[test]
    fn or_gives_alternatives_and_and_joins_them() {
        assert_eq!(alts("MIT OR Apache-2.0"), [["MIT"], ["Apache-2.0"]]);
        assert_eq!(alts("Apache-2.0/MIT"), [["Apache-2.0"], ["MIT"]]);
        assert_eq!(alts("Apache-2.0 / MIT"), [["Apache-2.0"], ["MIT"]]);
        assert_eq!(alts("Apache-2.0 AND MIT"), [["Apache-2.0", "MIT"]]);
        assert_eq!(
            alts("(MIT OR Apache-2.0) AND Unicode-3.0"),
            [["MIT", "Unicode-3.0"], ["Apache-2.0", "Unicode-3.0"]]
        );
        assert_eq!(
            alts("Apache-2.0 WITH LLVM-exception OR BSL-1.0"),
            [["Apache-2.0 WITH LLVM-exception"], ["BSL-1.0"]]
        );
        assert!(alternatives("MIT OR").is_err());
        assert!(alternatives("(MIT").is_err());
    }
}
