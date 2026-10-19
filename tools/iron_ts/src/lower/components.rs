//! Components for the host: `class Player extends Component` and `register(Player)`.
//!
//! `Component` is a prelude class (an `id` and the object it is on). `register(Player)` is erased
//! from the program and replaced by exported functions the engine calls without any script engine:
//!
//! - `iron_new_Player(id, gameObject)` makes the instance and fills the base fields;
//! - `iron_hook_Player_onUpdate(this, dt)` for every hook the class has, its own or inherited.
//!
//! Every hook thunk takes `(this, dt)` so the host calls them all alike; `layout.rs` turns these
//! exports into a class table. Design: `docs/research/iron-ts-engine-boundary.md` section 5.

use super::R;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::collections::HashMap;

/// The hooks the engine calls, in the order of the host's table.
pub const HOOKS: [&str; 7] = [
    "onAwake",
    "onStart",
    "onUpdate",
    "onLateUpdate",
    "onFixedUpdate",
    "onPreRender",
    "onDestroy",
];

/// The base class. `gameObject` is there when the manifest has the `GameObject` handle.
pub fn prelude(has_game_object: bool) -> String {
    let go = if has_game_object {
        "  gameObject: GameObject | null = null;\n"
    } else {
        ""
    };
    format!("class Component {{\n  id: number = 0;\n{go}}}\n")
}

/// Does the program use components? (`register(` is checked after parsing.)
pub fn wanted(src: &str) -> bool {
    src.contains("extends Component") || src.contains("register(")
}

/// The program with every top-level `register(Class)` removed (blanked, so lines stay) and the
/// thunks appended. `None` when there is nothing to register.
pub fn desugar(src: &str, has_game_object: bool) -> R<Option<String>> {
    if !src.contains("register(") {
        return Ok(None);
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::ts()).parse();
    if !ret.diagnostics.is_empty() {
        return Ok(None); // the real parse reports it
    }
    let mut classes: HashMap<String, (Option<String>, Vec<String>)> = HashMap::new();
    let mut registered: Vec<(String, (u32, u32))> = Vec::new();
    for st in &ret.program.body {
        let class = match st {
            Statement::ClassDeclaration(c) => Some(c),
            Statement::ExportDeclaration(x) => match &x.declaration {
                Declaration::ClassDeclaration(c) => Some(c),
                _ => None,
            },
            _ => None,
        };
        if let Some(c) = class
            && let Some(id) = &c.id
        {
            let parent = match c.heritage.as_ref().map(|h| &h.expression) {
                Some(Expression::Identifier(p)) => Some(p.name.to_string()),
                _ => None,
            };
            let mut methods = Vec::new();
            for el in &c.body.body {
                if let ClassElement::MethodDefinition(m) = el
                    && m.kind == MethodDefinitionKind::Method
                    && !m.r#static
                    && let PropertyKey::StaticIdentifier(k) = &m.key
                {
                    methods.push(k.name.to_string());
                }
            }
            classes.insert(id.name.to_string(), (parent, methods));
        }
        if let Statement::ExpressionStatement(es) = st
            && let Expression::CallExpression(call) = &es.expression
            && let Expression::Identifier(f) = &call.callee
            && f.name == "register"
        {
            let Some(Argument::Identifier(arg)) = call.arguments.first() else {
                return Err("register(..) takes a class name".to_string());
            };
            let sp = es.span();
            registered.push((arg.name.to_string(), (sp.start, sp.end)));
        }
    }
    if registered.is_empty() {
        return Ok(None);
    }
    let mut out = src.to_string();
    let mut tail = String::from("\n");
    for (name, (a, b)) in &registered {
        let Some(_) = classes.get(name) else {
            return Err(format!(
                "register({name}): `{name}` is not a class of this program"
            ));
        };
        // Blank the call, keeping its newlines.
        let blank: String = src[*a as usize..*b as usize]
            .chars()
            .map(|c| if c == '\n' { '\n' } else { ' ' })
            .collect();
        out.replace_range(*a as usize..*b as usize, &blank);
        let go = if has_game_object {
            ", go: GameObject | null"
        } else {
            ""
        };
        let set_go = if has_game_object {
            " c.gameObject = go;"
        } else {
            ""
        };
        tail += &format!(
            "export function iron_new_{name}(id: number{go}): {name} {{ const c = new {name}(); c.id = id;{set_go} return c; }}\n"
        );
        // The hooks the class or an ancestor defines.
        let mut chain = Some(name.clone());
        let mut have: Vec<&str> = Vec::new();
        let mut guard = 0;
        while let Some(cn) = chain {
            guard += 1;
            if guard > 64 {
                return Err(format!("class {name}: inheritance cycle"));
            }
            match classes.get(&cn) {
                Some((parent, methods)) => {
                    for h in HOOKS {
                        if methods.iter().any(|m| m == h) && !have.contains(&h) {
                            have.push(h);
                        }
                    }
                    chain = parent.clone();
                }
                None => break,
            }
        }
        for h in HOOKS {
            if have.contains(&h) {
                let call = if matches!(h, "onUpdate" | "onLateUpdate" | "onFixedUpdate") {
                    "dt"
                } else {
                    ""
                };
                tail += &format!(
                    "export function iron_hook_{name}_{h}(c: {name}, dt: number): void {{ c.{h}({call}); }}\n"
                );
            }
        }
    }
    out.push_str(&tail);
    Ok(Some(out))
}
