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
pub const HOOKS: [&str; 9] = [
    "onAwake",
    "onStart",
    "onUpdate",
    "onLateUpdate",
    "onFixedUpdate",
    "onPreRender",
    "onDestroy",
    "onApplicationFocus",
    "onApplicationPause",
];

/// The contact hooks, called by name when the physics step reports a contact. Each takes a
/// `Collision` (the other side). Their thunks take the pieces, the host passes plain numbers.
pub const CONTACT_HOOKS: [&str; 6] = [
    "onCollisionEnter",
    "onCollisionExit",
    "onCollisionStay",
    "onTriggerEnter",
    "onTriggerExit",
    "onTriggerStay",
];

/// The base class. `gameObject` is there when the manifest has the `GameObject` handle; so is
/// `Collision`, whose `collider` is a `Collider` handle when the manifest has one.
pub fn prelude(has_game_object: bool, has_collider: bool) -> String {
    let go = if has_game_object {
        "  gameObject: GameObject | null = null;\n  get transform(): Transform {\n    return this.gameObject!.transform;\n  }\n"
    } else {
        ""
    };
    let collision = if has_game_object {
        let ct = if has_collider {
            "Collider | null"
        } else {
            "number"
        };
        format!(
            "class Collision {{\n  other: GameObject | null;\n  collider: {ct};\n  speed: number;\n  sensor: boolean;\n  constructor(other: GameObject | null, collider: {ct}, speed: number, sensor: boolean) {{\n    this.other = other;\n    this.collider = collider;\n    this.speed = speed;\n    this.sensor = sensor;\n  }}\n  get gameObject(): GameObject | null {{\n    return this.other;\n  }}\n  get relativeSpeed(): number {{\n    return this.speed;\n  }}\n}}\ntype Collision2D = Collision;\n"
        )
    } else {
        String::new()
    };
    // `Collision2D` is the same data; a trigger hook that names `Collider` gets one holding the object.
    let extra = if has_game_object {
        let collider = if has_collider {
            ""
        } else {
            "class Collider {\n  gameObject: GameObject;\n  constructor(gameObject: GameObject) {\n    this.gameObject = gameObject;\n  }\n}\n"
        };
        format!("type Collision2D = Collision;\n{collider}")
    } else {
        String::new()
    };
    format!("{collision}{extra}class Component {{\n  id: number = 0;\n{go}}}\n")
}

/// Helper functions for `getComponentInChildren(Class)` calls: one per class named, appended to
/// the program (a class value is not a runtime value here, so the walk is written per class).
pub fn children_helpers(src: &str, has_game_object: bool) -> Option<String> {
    if !has_game_object || !src.contains("getComponentInChildren(") {
        return None;
    }
    let mut names: Vec<String> = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("getComponentInChildren(") {
        rest = &rest[i + "getComponentInChildren(".len()..];
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    let mut out = String::from("\n");
    for n in names {
        out += &format!(
            "function __cic_{n}(go: GameObject): {n} | undefined {{\n  const own = go.getComponent({n});\n  if (own) return own;\n  for (let i = 0; i < go.childCount; i++) {{\n    const found = __cic_{n}(go.childAt(i));\n    if (found) return found;\n  }}\n  return undefined;\n}}\n"
        );
    }
    Some(out)
}

/// Helpers for the `GameObject` properties that are arrays (`children`, `GameObject.roots`), written
/// in TypeScript over the engine's count/index functions and appended when the program names them.
pub fn tree_helpers(src: &str, has_game_object: bool) -> Option<String> {
    if !has_game_object {
        return None;
    }
    let mut out = String::new();
    if src.contains(".children") {
        out += "\nfunction __go_children(go: GameObject): GameObject[] {\n  const out: GameObject[] = [];\n  for (let i = 0; i < go.childCount; i++) out.push(go.childAt(i));\n  return out;\n}\n";
    }
    if src.contains("GameObject.roots") {
        out += "\nfunction __go_roots(): GameObject[] {\n  const out: GameObject[] = [];\n  const n = GameObject.rootCount();\n  for (let i = 0; i < n; i++) out.push(GameObject.rootAt(i));\n  return out;\n}\n";
    }
    (!out.is_empty()).then_some(out)
}

/// Does the program use components? (`register(` is checked after parsing.)
pub fn wanted(src: &str) -> bool {
    src.contains("extends Component") || src.contains("register(")
}

/// The program with every top-level `register(Class)` removed (blanked, so lines stay) and the
/// thunks appended. `None` when there is nothing to register.
pub fn desugar(src: &str, has_game_object: bool, has_collider: bool) -> R<Option<String>> {
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
    // (class, method) -> the type its first parameter names.
    let mut param_types: HashMap<(String, String), String> = HashMap::new();
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
                    // The type its first parameter names (`other: Collider`).
                    let ann = m
                        .value
                        .params
                        .items
                        .first()
                        .and_then(|p| p.type_annotation.as_ref())
                        .and_then(|t| match &t.type_annotation {
                            TSType::TSTypeReference(r) => match &r.type_name {
                                TSTypeName::IdentifierReference(i) => Some(i.name.to_string()),
                                _ => None,
                            },
                            _ => None,
                        });
                    if let Some(a) = ann {
                        param_types.insert((id.name.to_string(), k.name.to_string()), a);
                    }
                }
            }
            classes.insert(id.name.to_string(), (parent, methods));
        }
        if let Statement::ExpressionStatement(es) = st
            && let Expression::CallExpression(call) = &es.expression
            && let Expression::Identifier(f) = &call.callee
            && f.name == "register"
        {
            let sp = es.span();
            match call.arguments.first() {
                // `register(Player)`
                Some(Argument::Identifier(arg)) => {
                    registered.push((arg.name.to_string(), (sp.start, sp.end)));
                }
                // `register({ Player, Enemy })`: the engine's form, several classes at once.
                Some(Argument::ObjectExpression(o)) => {
                    for p in &o.properties {
                        let ObjectPropertyKind::ObjectProperty(p) = p else {
                            return Err("register({ .. }) takes class names".to_string());
                        };
                        let PropertyKey::StaticIdentifier(k) = &p.key else {
                            return Err("register({ .. }) takes class names".to_string());
                        };
                        registered.push((k.name.to_string(), (sp.start, sp.end)));
                    }
                }
                _ => return Err("register(..) takes a class name".to_string()),
            }
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
        let mut contact_impl: HashMap<&str, (String, Option<String>)> = HashMap::new();
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
                    // `onCollisionEnter2D` is the same slot as `onCollisionEnter`.
                    for h in CONTACT_HOOKS {
                        let two_d = format!("{h}2D");
                        let found = [h.to_string(), two_d]
                            .into_iter()
                            .find(|n| methods.iter().any(|m| m == n));
                        if let Some(n) = found
                            && !have.contains(&h)
                        {
                            have.push(h);
                            let ty = param_types.get(&(cn.clone(), n.clone())).cloned();
                            contact_impl.insert(h, (n, ty));
                        }
                    }
                    chain = parent.clone();
                }
                None => break,
            }
        }
        if has_game_object {
            for h in CONTACT_HOOKS {
                if have.contains(&h) {
                    let (method, ty) = contact_impl[h].clone();
                    tail += &contact_thunk(name, h, &method, ty.as_deref(), has_collider);
                }
            }
        }
        for h in HOOKS {
            if have.contains(&h) {
                // The frame hooks get the frame length; the focus and pause hooks a flag.
                let call = if matches!(h, "onUpdate" | "onLateUpdate" | "onFixedUpdate") {
                    "dt"
                } else if matches!(h, "onApplicationFocus" | "onApplicationPause") {
                    "dt !== 0"
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

/// The thunk of a contact hook: the host passes the other object, its collider, the speed and
/// the sensor flag as plain values and the thunk builds the `Collision`.
fn contact_thunk(
    name: &str,
    h: &str,
    method: &str,
    param: Option<&str>,
    has_collider: bool,
) -> String {
    let ct = if has_collider {
        "Collider | null"
    } else {
        "number"
    };
    // A hook that takes the other `Collider` (a trigger's `other`) gets it; the rest a `Collision`.
    let arg = if param == Some("Collider") && !has_collider {
        "new Collider(other!)"
    } else {
        "new Collision(other, collider, speed, sensor !== 0)"
    };
    format!(
        "export function iron_hook_{name}_{h}(c: {name}, other: GameObject | null, collider: {ct}, speed: number, sensor: number): void {{ c.{method}({arg}); }}\n"
    )
}
