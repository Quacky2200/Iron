//! The TypeScript `@iron/engine` shapes that the native API holds differently, expanded at
//! compile time over the engine API: object-valued namespace properties
//! (`RenderSettings.skybox = { .. }`) become per-field setters.

use super::expr::mk;
use super::*;

/// Namespace properties the TypeScript API holds as one object that the native API keeps as
/// per-field properties: (namespace, property) -> (the on/off property, the prefix of the field
/// properties, so `zenith` is `skyZenith`).
fn split_rule(ns: &str, prop: &str) -> Option<(&'static str, &'static str)> {
    match (ns, prop) {
        ("RenderSettings", "skybox") => Some(("skyboxEnabled", "sky")),
        ("RenderSettings", "bloom") => Some(("bloomEnabled", "bloom")),
        _ => None,
    }
}

const FACADE_TS: &str = include_str!("facade.ts");

/// `old` as a whole word (not part of a longer name).
fn replace_word(src: &str, old: &str, new: &str) -> String {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find(old) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + old.len()..].chars().next();
        out.push_str(&rest[..i]);
        if before.is_some_and(word) || after.is_some_and(word) {
            out.push_str(old);
        } else {
            out.push_str(new);
        }
        rest = &rest[i + old.len()..];
    }
    out.push_str(rest);
    out
}

/// Whether argument `i` of `c` is a payload to send: present and not a literal `undefined` / `null`.
fn has_payload(c: &CallExpression<'_>, i: usize) -> bool {
    match c.arguments.get(i).and_then(|a| a.as_expression()) {
        None | Some(Expression::NullLiteral(_)) => false,
        Some(Expression::Identifier(id)) => id.name != "undefined",
        Some(_) => true,
    }
}

/// Whether argument `i` of `c` is a function written without parameters.
fn zero_param_closure(c: &CallExpression<'_>, i: usize) -> bool {
    match c.arguments.get(i).and_then(|a| a.as_expression()) {
        Some(Expression::ArrowFunctionExpression(f)) => {
            f.params.items.is_empty() && f.params.rest.is_none()
        }
        Some(Expression::FunctionExpression(f)) => {
            f.params.items.is_empty() && f.params.rest.is_none()
        }
        _ => false,
    }
}

/// Hash of a type's written text without its spaces: names the `request<R>` facade of that type.
pub(crate) fn request_key(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes().filter(|b| !b.is_ascii_whitespace()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// The reply types written in `.request<R>(` / `.ask<R>(` calls of `src` (`call` is the text up to `<`) (the text between the angle brackets).
fn request_types(src: &str, call: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find(call) {
        let after = &rest[i + call.len()..];
        let bytes = after.as_bytes();
        let (mut depth, mut end) = (1usize, None);
        for (j, &c) in bytes.iter().enumerate() {
            match c {
                b'<' => depth += 1,
                // `=>` is not a closing bracket.
                b'>' if j > 0 && bytes[j - 1] == b'=' => {}
                b'>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(j);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else { break };
        let ty = after[..end].trim().to_string();
        if !ty.is_empty() && !out.contains(&ty) {
            out.push(ty);
        }
        rest = &after[end..];
    }
    out
}

/// `request<R>` resolves with the reply parsed as `R`. A promise class exists per written type, so
/// the function is written per reply type (`@R@` is the text, `@K@` its key).
const REQUEST_TEMPLATE: &str = r#"
function __xm_Window_request_@K@<T>(self: Window, data: T, timeoutMs?: number): Promise<@R@> {
  __win_listen_replies();
  return new Promise<@R@>((resolve, reject) => {
    const id = self.postRequestJson(JSON.stringify(data), timeoutMs ?? NaN);
    if (id === 0) {
      reject(new Error("request refused: the window is closed or the message is over 1 MiB"));
      return;
    }
    __win_pending.set(id, (text: string, error: string) => {
      if (error !== "") {
        reject(new Error(error));
      } else {
        const value: @R@ = JSON.parse(text);
        resolve(value);
      }
    });
  });
}
"#;

/// `ask<R>` is `request<R>` of a named event (see the named events chunk of `facade.ts`): the text
/// function holds the promise, the two generic ones build the event text (`ask0` has no payload;
/// its type parameter is a dummy the lowerer fills in).
const ASK_TEMPLATE: &str = r#"
function __xm_Window_asktext_@K@(self: Window, text: string, timeoutMs?: number): Promise<@R@> {
  __win_listen_replies();
  return new Promise<@R@>((resolve, reject) => {
    const id = self.postRequestJson(text, timeoutMs ?? NaN);
    if (id === 0) {
      reject(new Error("request refused: the window is closed or the message is over 1 MiB"));
      return;
    }
    __win_pending.set(id, (reply: string, error: string) => {
      if (error !== "") {
        reject(new Error(error));
      } else {
        const value: @R@ = JSON.parse(reply);
        resolve(value);
      }
    });
  });
}
function __xm_Window_ask_@K@<T>(self: Window, name: string, payload: T, timeoutMs?: number): Promise<@R@> {
  return __xm_Window_asktext_@K@(self, __win_event_text(name, JSON.stringify(payload)), timeoutMs);
}
function __xm_Window_ask0_@K@<U>(self: Window, name: string, timeoutMs?: number): Promise<@R@> {
  return __xm_Window_asktext_@K@(self, __win_event_text(name, null), timeoutMs);
}
"#;

/// The program with the TypeScript names that are another name natively (`AnimatorController` is
/// `AnimationController`) renamed, and the facade chunks it needs appended.
pub fn extend_source(src: &str, manifest: &crate::manifest::Manifest) -> Option<String> {
    let mut out = src.to_string();
    if manifest.classes.contains_key("AnimationController") && src.contains("AnimatorController") {
        out = replace_word(&out, "AnimatorController", "AnimationController");
    }
    // The chunks the program names: only when the engine API has the classes they are written over.
    let mut extra = String::new();
    for chunk in FACADE_TS.split("//# when ").skip(1) {
        let (header, body) = chunk.split_once('\n').unwrap_or((chunk, ""));
        // `//# when a|b @ Vector3 GameObject`: only for programs whose engine API has the names
        // after `@` (a comment that mentions a trigger does not pull a chunk in).
        let (triggers, needs) = header.split_once(" @ ").unwrap_or((header, ""));
        let have = |n: &str| {
            manifest.classes.contains_key(n)
                || manifest.values.contains_key(n)
                || manifest.handles.contains(n)
                || manifest.namespaces.contains_key(n)
        };
        if needs.split_whitespace().all(have)
            && triggers
                .split('|')
                .any(|t| t.split('&').all(|p| src.contains(p.trim_end())))
        {
            extra.push_str(body);
        }
    }
    for r in request_types(src, ".request<") {
        extra.push_str(
            &REQUEST_TEMPLATE
                .replace("@K@", &request_key(&r))
                .replace("@R@", &r),
        );
    }
    for r in request_types(src, ".ask<") {
        extra.push_str(
            &ASK_TEMPLATE
                .replace("@K@", &request_key(&r))
                .replace("@R@", &r),
        );
    }
    if !extra.is_empty() {
        out.push('\n');
        out.push_str(&extra);
    }
    (out != src).then_some(out)
}

// The facade: members of the TypeScript `@iron/engine` that the native API has in another shape
// are written once in TypeScript (`facade.ts`, appended to a program that names them) as
// functions called `__x<kind>_<Class>_<member>`:
//
//   __xm_  method          `__xm_Cloth_foo(self: Cloth, a: number)`      for `cloth.foo(a)`
//   __xg_  property read   `__xg_Camera_rect(self: Camera): Rect`         for `cam.rect`
//   __xs_  property write  `__xs_Camera_rect(self: Camera, v: Rect)`      for `cam.rect = r`, `{ rect: r }`
//   __xt_  static call     `__xt_Sprite_grid(texture: Texture, ..)`        for `Sprite.grid(..)`
//   __xp_  static read     `__xp_Screen_windowSize(): Size`               for `Screen.windowSize`
//   __xw_  static write    `__xw_RenderSettings_heightFog(v: HeightFogSettings | null)`  for `RenderSettings.heightFog = v`
//
// A method also has an object-argument variant `__xm_..$o` used when a call passes an object
// literal. These win over the native member of the same name.
impl<'a> Lowerer<'a> {
    fn x_name(kind: &str, class: &str, member: &str) -> String {
        format!("__x{kind}_{class}_{member}")
    }

    /// `recv.m(args)` through a facade method.
    pub(crate) fn ext_method_call(
        &mut self,
        class: &str,
        m: &str,
        recv: &Expr,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        if class == "Window" && m == "request" {
            return self.window_request(recv, c).map(Some);
        }
        if class == "Window" && m == "ask" {
            return self.window_ask(recv, c).map(Some);
        }
        // A handler without parameters has no payload type to decode.
        if class == "Window" && matches!(m, "on" | "answer") && zero_param_closure(c, 1) {
            let name = format!("__xm_Window_{m}0");
            if m == "on" {
                let inst = self.instantiate_fn(&name, vec![Ty::Num])?;
                return self
                    .call_with_receiver(&inst, recv.clone(), &c.arguments)
                    .map(Some);
            }
            let what = "Window.answer";
            let g = self.generic_fns[&name].clone();
            let targs = self.infer_targs_skip(
                what,
                &g,
                c.type_arguments.as_deref(),
                &c.arguments,
                None,
                None,
                1,
            )?;
            let (name, targs) = self.async_variant(&name, targs);
            self.check_message_types(what, class, m, &targs)?;
            let inst = self.instantiate_fn(&name, targs)?;
            return self
                .call_with_receiver(&inst, recv.clone(), &c.arguments)
                .map(Some);
        }
        // `win.emit("name")`: no payload, so no type to infer (the dummy type argument is unused).
        if class == "Window" && m == "emit" && !has_payload(c, 1) {
            return self
                .window_no_payload(recv, c, "__xm_Window_emit0")
                .map(Some);
        }
        let mut name = Self::x_name("m", class, m);
        let obj_variant = format!("{name}$o");
        if self.fids.contains_key(&obj_variant)
            && c.arguments
                .iter()
                .any(|a| matches!(a, Argument::ObjectExpression(_)))
        {
            name = obj_variant;
        }
        if !self.fids.contains_key(&name) {
            // A generic facade method (`win.postMessage(data)`, `win.onMessage<T>(f)`): the type
            // arguments come from the call or the arguments after the receiver.
            if self.generic_fns.contains_key(&name) {
                let what = format!("{class}.{m}");
                let g = self.generic_fns[&name].clone();
                let targs = self.infer_targs_skip(
                    &what,
                    &g,
                    c.type_arguments.as_deref(),
                    &c.arguments,
                    None,
                    None,
                    1,
                )?;
                let (name, targs) = self.async_variant(&name, targs);
                self.check_message_types(&what, class, m, &targs)?;
                let inst = self.instantiate_fn(&name, targs)?;
                return self
                    .call_with_receiver(&inst, recv.clone(), &c.arguments)
                    .map(Some);
            }
            return Ok(None);
        }
        self.call_with_receiver(&name, recv.clone(), &c.arguments)
            .map(Some)
    }

    /// `win.request<R>(data, timeoutMs?)`: the facade written for the reply type `R` (see
    /// `REQUEST_TEMPLATE`), with the type of `data` inferred.
    fn window_request(&mut self, recv: &Expr, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let what = "Window.request";
        let Some(ta) = c.type_arguments.as_deref().and_then(|t| t.params.first()) else {
            bail!("{at}: {what} needs the reply type: `request<{{ n: number }}>(data)`")
        };
        let span = oxc_span::GetSpan::span(ta);
        let text = self
            .src
            .get(span.start as usize..span.end as usize)
            .unwrap_or("")
            .to_string();
        let name = format!("__xm_Window_request_{}", request_key(&text));
        if !self.generic_fns.contains_key(&name) {
            bail!("{at}: {what}<{text}>: no facade was written for this reply type")
        }
        let rty = self.ty(ta, what)?;
        self.message_type_ok(what, &rty, "reply", &mut Vec::new())?;
        let g = self.generic_fns[&name].clone();
        let targs = self.infer_targs_skip(what, &g, None, &c.arguments, None, None, 1)?;
        for t in &targs {
            self.message_type_ok(what, t, "message", &mut Vec::new())?;
        }
        let inst = self.instantiate_fn(&name, targs)?;
        self.call_with_receiver(&inst, recv.clone(), &c.arguments)
    }

    /// `win.ask<R>(name, payload?, timeoutMs?)`: the facade written for the reply type `R` (see
    /// `ASK_TEMPLATE`), with the type of `payload` inferred.
    fn window_ask(&mut self, recv: &Expr, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let what = "Window.ask";
        let Some(ta) = c.type_arguments.as_deref().and_then(|t| t.params.first()) else {
            bail!("{at}: {what} needs the reply type: `ask<{{ n: number }}>(\"name\", payload)`")
        };
        let span = oxc_span::GetSpan::span(ta);
        let text = self
            .src
            .get(span.start as usize..span.end as usize)
            .unwrap_or("")
            .to_string();
        let key = request_key(&text);
        let rty = self.ty(ta, what)?;
        self.message_type_ok(what, &rty, "reply", &mut Vec::new())?;
        if !has_payload(c, 1) {
            let name = format!("__xm_Window_ask0_{key}");
            if !self.generic_fns.contains_key(&name) {
                bail!("{at}: {what}<{text}>: no facade was written for this reply type")
            }
            // `ask("n", undefined, 500)`: the timeout is the third argument.
            let args: Vec<&'a Argument<'a>> = c
                .arguments
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != 1)
                .map(|(_, a)| a)
                .collect();
            let inst = self.instantiate_fn(&name, vec![Ty::Num])?;
            return self.call_with_receiver_refs(&inst, recv.clone(), &args);
        }
        let name = format!("__xm_Window_ask_{key}");
        if !self.generic_fns.contains_key(&name) {
            bail!("{at}: {what}<{text}>: no facade was written for this reply type")
        }
        let g = self.generic_fns[&name].clone();
        let targs = self.infer_targs_skip(what, &g, None, &c.arguments, None, None, 1)?;
        for t in &targs {
            self.message_type_ok(what, t, "message", &mut Vec::new())?;
        }
        let inst = self.instantiate_fn(&name, targs)?;
        self.call_with_receiver(&inst, recv.clone(), &c.arguments)
    }

    /// `win.emit("name")`: the facade function without a payload (its type parameter is a dummy).
    fn window_no_payload(&mut self, recv: &Expr, c: &'a CallExpression<'a>, name: &str) -> R<Expr> {
        let args: Vec<&'a Argument<'a>> = c.arguments.iter().take(1).collect();
        let inst = self.instantiate_fn(name, vec![Ty::Num])?;
        self.call_with_receiver_refs(&inst, recv.clone(), &args)
    }

    /// A window handler that returns a promise (`answer(name, async (..) => ..)`, an async
    /// `onRequest` handler) goes to the facade function written for promises (`..P`), with the
    /// promise's value as the type argument.
    fn async_variant(&self, name: &str, targs: Vec<Ty>) -> (String, Vec<Ty>) {
        let value = |t: &Ty| self.promise_of(t).map(|(_, v)| v);
        let promised = match (name, targs.as_slice()) {
            ("__xm_Window_answer", [t, r]) => value(r).map(|v| vec![t.clone(), v]),
            ("__xm_Window_answer0", [r]) => value(r).map(|v| vec![v]),
            ("__xm_Window_onRequest" | "__xt_Window_onRequest", [t, r, u]) => {
                value(u).map(|_| vec![t.clone(), r.clone()])
            }
            _ => None,
        };
        match promised {
            Some(targs) => (format!("{name}P"), targs),
            None => (name.to_string(), targs),
        }
    }

    /// Messages between windows travel as JSON of their static type: no engine handle (a
    /// `GameObject`, a `Window`, ..), function or `Map` / `Set` may be part of it. The error names
    /// the path to the offending field.
    pub(crate) fn check_message_types(
        &self,
        what: &str,
        class: &str,
        m: &str,
        targs: &[Ty],
    ) -> R<()> {
        if class != "Window"
            || !matches!(
                m,
                "postMessage"
                    | "onMessage"
                    | "onRequest"
                    | "broadcast"
                    | "emit"
                    | "emitAll"
                    | "on"
                    | "answer"
            )
        {
            return Ok(());
        }
        for t in targs {
            self.message_type_ok(what, t, "message", &mut Vec::new())?;
        }
        Ok(())
    }

    fn message_type_ok(&self, what: &str, ty: &Ty, path: &str, seen: &mut Vec<StructId>) -> R<()> {
        match ty {
            Ty::Handle(n) => bail!(
                "{what}: `{path}` is a {n} handle; handles cannot be sent in messages (send a number such as `win.handle` and look it up on the other side)"
            ),
            Ty::Fn(_) => {
                bail!("{what}: `{path}` is a function; functions cannot be sent in messages")
            }
            Ty::Map(..) | Ty::Set(_) => {
                bail!("{what}: `{path}` is a Map or Set; use an array or an object in messages")
            }
            Ty::Arr(e) => self.message_type_ok(what, e, &format!("{path}[]"), seen),
            Ty::Nullable(e) | Ty::Cell(e) => self.message_type_ok(what, e, path, seen),
            Ty::Obj(sid) => {
                if seen.contains(sid) {
                    return Ok(());
                }
                seen.push(*sid);
                for f in &self.m.structs[*sid].fields {
                    if f.name.starts_with("_p_") {
                        continue;
                    }
                    self.message_type_ok(what, &f.ty, &format!("{path}.{}", f.name), seen)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A call of the facade function `name(self, ..)` with `recv` as `self`: the arguments fill
    /// the parameters after it (a missing one takes its default, or null for `x?: T`).
    fn call_with_receiver(&mut self, name: &str, recv: Expr, args: &'a [Argument<'a>]) -> R<Expr> {
        let refs: Vec<&'a Argument<'a>> = args.iter().collect();
        self.call_with_receiver_refs(name, recv, &refs)
    }

    fn call_with_receiver_refs(
        &mut self,
        name: &str,
        recv: Expr,
        args: &[&'a Argument<'a>],
    ) -> R<Expr> {
        let fid = self.fids[name];
        self.ensure_lowered(name)?;
        let (ptys, ret) = self.fsig[fid].clone();
        let Some(ret) = ret else {
            bail!("{name}: return type unknown")
        };
        let fp: &'a FormalParameters<'a> = self.fsrc[name].params;
        let n_fixed = fp.items.len() - 1;
        if args.len() > n_fixed {
            bail!(
                "{name}: {} arguments, at most {n_fixed} expected",
                args.len()
            );
        }
        let t = self.new_local(recv.ty.clone());
        let mut pre = vec![Stmt::Assign(t, recv)];
        let mut vals = vec![self.var_expr(t)];
        let mut names: HashMap<String, Binding> = HashMap::new();
        for i in 0..n_fixed {
            let (p, pty) = (&fp.items[i + 1], ptys[i + 1].clone());
            let v = if let Some(&a) = args.get(i) {
                let Some(e) = a.as_expression() else {
                    bail!("{name}: spread argument")
                };
                let v = self.lower_expr(e, Some(&pty))?;
                self.coerce(v, &pty)?
            } else if let Some(init) = &p.initializer {
                let saved = std::mem::replace(&mut self.cur.scopes, vec![names.clone()]);
                let r = self.lower_expr(init, Some(&pty));
                self.cur.scopes = saved;
                self.coerce(r?, &pty)?
            } else if p.optional {
                self.coerce(mk(ExprKind::Null, Ty::Null), &pty)?
            } else {
                bail!("{name}: {} arguments, {n_fixed} expected", args.len())
            };
            let tmp = self.new_local(pty);
            pre.push(Stmt::Assign(tmp, v));
            vals.push(self.var_expr(tmp));
            if let BindingPattern::BindingIdentifier(b) = &p.pattern {
                names.insert(b.name.to_string(), Binding::Direct(tmp));
            }
        }
        let call = Self::target_call(&classes::Target::Direct(fid), vals, ret.clone());
        Ok(mk(ExprKind::Seq(pre, Box::new(call)), ret))
    }

    /// `setPoints(this, points)` as the setter of `points` (`{ points: [..] }`, `line.positions = ..`):
    /// a method named `set<Prop>` taking the receiver and one value.
    pub(crate) fn set_method(&self, class: &str, prop: &str) -> Option<crate::manifest::ExternFn> {
        let mut cap = prop.to_string();
        cap[..1].make_ascii_uppercase();
        self.manifest
            .classes
            .get(class)?
            .methods
            .get(&format!("set{cap}"))
            .filter(|f| f.params.len() == 2)
            .cloned()
    }

    /// `obj.prop` through a facade getter.
    pub(crate) fn ext_getter(&mut self, class: &str, prop: &str, obj: &Expr) -> R<Option<Expr>> {
        let name = Self::x_name("g", class, prop);
        if !self.fids.contains_key(&name) {
            // `camera.transform`: a component's object's transform.
            if prop == "transform"
                && class != "GameObject"
                && let Some(go) = self
                    .manifest
                    .classes
                    .get(class)
                    .and_then(|k| k.getters.get("gameObject"))
                    .cloned()
                && let Some(tr) = self
                    .manifest
                    .classes
                    .get("GameObject")
                    .and_then(|k| k.getters.get("transform"))
                    .cloned()
            {
                let owner = self.emit_extern(&go, vec![obj.clone()]);
                return Ok(Some(self.emit_extern(&tr, vec![owner])));
            }
            return Ok(None);
        }
        self.call_helper(&name, vec![obj.clone()]).map(Some)
    }

    /// The facade function that sets `obj.prop`, and the type of the value it takes.
    pub(crate) fn ext_setter(&mut self, class: &str, prop: &str) -> R<Option<(FuncId, Ty)>> {
        let name = Self::x_name("s", class, prop);
        let Some(&fid) = self.fids.get(&name) else {
            return Ok(None);
        };
        self.ensure_lowered(&name)?;
        let ty = self.fsig[fid].0[1].clone();
        Ok(Some((fid, ty)))
    }

    /// `Class.m(args)` through a facade static function.
    pub(crate) fn ext_static_call(
        &mut self,
        class: &str,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        if class == "GameObject" && m == "createPrimitive" {
            return self.lower_create_primitive(c);
        }
        if m.ends_with("$n") {
            return Ok(None);
        }
        // `AnimationCurve.fromPoints([[t, v], ..])` is the facade's; a `Vector2[]` is the native one.
        if class == "AnimationCurve"
            && m == "fromPoints"
            && let Some(first) = c.arguments.first().and_then(|a| a.as_expression())
            && let Ok(v) = self.lower_expr(first, None)
            && matches!(v.ty.strip(), Ty::Arr(e) if matches!(e.strip(), Ty::Obj(_)))
        {
            return Ok(None);
        }
        let mut name = Self::x_name("t", class, m);
        // `Window.emitAll("name")`: no payload to infer a type from.
        if class == "Window" && m == "emitAll" && !has_payload(c, 1) {
            let inst = self.instantiate_fn("__xt_Window_emitAll0", vec![Ty::Num])?;
            let one = &c.arguments[..c.arguments.len().min(1)];
            return self.call_declared(&inst, one, None).map(Some);
        }
        // A generic facade function (`Window.broadcast(data)`, `Window.onMessage<T>(f)`).
        if self.generic_fns.contains_key(&name) {
            let what = format!("{class}.{m}");
            let g = self.generic_fns[&name].clone();
            let targs = self.infer_targs(
                &what,
                &g,
                c.type_arguments.as_deref(),
                &c.arguments,
                None,
                None,
            )?;
            let (name, targs) = self.async_variant(&name, targs);
            self.check_message_types(&what, class, m, &targs)?;
            let inst = self.instantiate_fn(&name, targs)?;
            return self.call_declared(&inst, &c.arguments, None).map(Some);
        }
        let argc_variant = format!("{name}${}", c.arguments.len());
        if self.fids.contains_key(&argc_variant) {
            name = argc_variant;
        }
        let obj_variant = format!("{name}$o");
        if self.fids.contains_key(&obj_variant)
            && c.arguments
                .iter()
                .any(|a| matches!(a, Argument::ObjectExpression(_)))
        {
            name = obj_variant;
        }
        if !self.fids.contains_key(&name) {
            return Ok(None);
        }
        self.call_declared(&name, &c.arguments, None).map(Some)
    }

    /// `GameObject.createPrimitive("cube", { position, scale, .. })`: `Primitives.create(kind)`,
    /// then the options as for `new GameObject`.
    fn lower_create_primitive(&mut self, c: &'a CallExpression<'a>) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let Some(create) = self
            .manifest
            .namespaces
            .get("Primitives")
            .and_then(|n| n.get("create"))
            .cloned()
        else {
            return Ok(None);
        };
        let Some(kind) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: createPrimitive takes a kind")
        };
        let kind = self.lower_expr(kind, Some(&Ty::Str))?;
        let kind = self.coerce(kind, &Ty::Str)?;
        let ty = Ty::Handle("GameObject".to_string());
        let tmp = self.new_local(ty.clone());
        let none = mk(ExprKind::Null, Ty::Str.nullable());
        let made = self.emit_extern(&create, vec![kind, none]);
        let mut stmts = vec![Stmt::Assign(tmp, made)];
        match c.arguments.get(1).and_then(|a| a.as_expression()) {
            Some(Expression::ObjectExpression(o)) => {
                self.apply_options("GameObject", tmp, o, &mut stmts, &at)?
            }
            Some(_) => bail!("{at}: createPrimitive options are an object literal"),
            None => {}
        }
        let result = self.var_expr(tmp);
        Ok(Some(mk(ExprKind::Seq(stmts, Box::new(result)), ty)))
    }

    /// `Class.prop` through a facade static getter, or a static function of the engine API that
    /// takes nothing (`Camera.main`).
    pub(crate) fn ext_static_prop(&mut self, class: &str, prop: &str) -> R<Option<Expr>> {
        let name = Self::x_name("p", class, prop);
        if self.fids.contains_key(&name) {
            return self.call_helper(&name, vec![]).map(Some);
        }
        let native = self
            .manifest
            .classes
            .get(class)
            .and_then(|k| k.statics.get(prop))
            .filter(|f| f.params.is_empty() && f.ret != ExtTy::Void)
            .cloned();
        match native {
            Some(f) => {
                let ty = self.ext_ir(&f.ret);
                let mut e = self.emit_extern(&f, vec![]);
                // A handle that may be none (`Camera.main`).
                if matches!(ty, Ty::Handle(_)) {
                    e.ty = ty.nullable();
                }
                Ok(Some(e))
            }
            None => Ok(None),
        }
    }

    /// A `new GameObject(name, { .. })` option the plain setter lookup would not find: `parent`
    /// (`setParent(p, false)`), and `position` / `rotation` / `scale`, which are local to the
    /// parent (`localPosition`, ...). `Ok(false)` leaves the key to the default path.
    pub(crate) fn game_object_option(
        &mut self,
        tmp: VarId,
        key: &str,
        value: &'a Expression<'a>,
        stmts: &mut Vec<Stmt>,
        at: &str,
    ) -> R<bool> {
        let go = &self.manifest.classes["GameObject"];
        if key == "parent" {
            let Some(f) = go.methods.get("setParent").cloned() else {
                bail!("{at}: GameObject has no setParent")
            };
            let want = self.ext_ir(&f.params[1].1);
            let v = self.lower_expr(value, Some(&want))?;
            let v = self.coerce_ext(v, &want)?;
            let h = self.var_expr(tmp);
            let keep = mk(ExprKind::Bool(false), Ty::Bool);
            stmts.push(Stmt::Eval(self.emit_extern(&f, vec![h, v, keep])));
            return Ok(true);
        }
        let local = match key {
            "position" => "localPosition",
            "rotation" => "localRotation",
            "scale" => "localScale",
            "active" => {
                let Some(setter) = go.setters.get("enabled").cloned() else {
                    return Ok(false);
                };
                let v = self.lower_expr(value, Some(&Ty::Bool))?;
                let v = self.coerce(v, &Ty::Bool)?;
                let h = self.var_expr(tmp);
                stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![h, v])));
                return Ok(true);
            }
            _ => return Ok(false),
        };
        let get = go.getters.get("transform").cloned();
        let set = self
            .manifest
            .classes
            .get("Transform")
            .and_then(|t| t.setters.get(local))
            .cloned();
        let (Some(get), Some(set)) = (get, set) else {
            return Ok(false);
        };
        let want = self.ext_ir(&set.params[1].1);
        let v = self.lower_expr(value, None)?;
        // An optional value (`rotation?: Quaternion` passed on) is left alone when it is not there.
        // (An optional parameter of an object type is the object type with a null pointer.)
        if v.ty.is_nullable()
            || matches!(v.ty.strip(), Ty::Obj(_) if matches!(v.kind, ExprKind::Var(_)))
        {
            let tv = v;
            let is_null = mk(ExprKind::IsNull(Box::new(tv.clone())), Ty::Bool);
            let present = mk(ExprKind::Un(UnOp::Not, Box::new(is_null)), Ty::Bool);
            let inner = self.coerce(tv, &want)?;
            let h = self.var_expr(tmp);
            let tr = self.emit_extern(&get, vec![h]);
            let write = Stmt::Eval(self.emit_extern(&set, vec![tr, inner]));
            stmts.push(Stmt::If(present, vec![write], vec![]));
            return Ok(true);
        }
        let v = self.coerce(v, &want)?;
        let h = self.var_expr(tmp);
        let tr = self.emit_extern(&get, vec![h]);
        stmts.push(Stmt::Eval(self.emit_extern(&set, vec![tr, v])));
        Ok(true)
    }

    /// `RenderSettings.skybox = { .. }` / `= null`: off, then on with the defaults, then each key's
    /// setter (a `null` key turns its `has<Key>` switch off, as `sunDirection: null` does).
    pub(crate) fn lower_split_assign(
        &mut self,
        a: &'a AssignmentExpression<'a>,
    ) -> R<Option<Vec<Stmt>>> {
        let Some(SimpleAssignmentTarget::StaticMemberExpression(m)) =
            a.left.as_simple_assignment_target()
        else {
            return Ok(None);
        };
        let Expression::Identifier(id) = &m.object else {
            return Ok(None);
        };
        let (ns, prop) = (id.name.as_str(), m.property.name.as_str());
        if a.operator != AssignmentOperator::Assign || self.lookup(ns).is_some() {
            return Ok(None);
        }
        // `RenderSettings.heightFog = h`: a facade write (`__xw_RenderSettings_heightFog(h)`).
        let write = Self::x_name("w", ns, prop);
        if let Some(&fid) = self.fids.get(&write) {
            self.ensure_lowered(&write)?;
            let want = self.fsig[fid].0[0].clone();
            let v = self.lower_expr(&a.right, Some(&want))?;
            let v = self.coerce_ext(v, &want)?;
            let call = self.call_helper(&write, vec![v])?;
            return Ok(Some(vec![Stmt::Eval(call)]));
        }
        let Some((enable, prefix)) = split_rule(ns, prop) else {
            return Ok(None);
        };
        let at = self.loc(a.span.start);
        let props = self.manifest.ns_props.get(ns).cloned().unwrap_or_default();
        let setter_of = |name: &str| props.get(name).and_then(|p| p.setter.clone());
        let Some(enable) = setter_of(enable) else {
            bail!("{at}: {ns}.{prop} has no switch in the engine API")
        };
        let mut stmts = Vec::new();
        let switch = |this: &mut Self, on: bool| {
            let v = mk(ExprKind::Bool(on), Ty::Bool);
            Stmt::Eval(this.emit_extern(&enable, vec![v]))
        };
        stmts.push(switch(self, false));
        match &a.right {
            Expression::NullLiteral(_) => Ok(Some(stmts)),
            Expression::ObjectExpression(o) => {
                stmts.push(switch(self, true));
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        bail!("{at}: spread in {ns}.{prop}")
                    };
                    let PropertyKey::StaticIdentifier(key) = &p.key else {
                        bail!("{at}: {ns}.{prop} keys are plain names")
                    };
                    let mut cap = key.name.to_string();
                    cap[..1].make_ascii_uppercase();
                    if matches!(&p.value, Expression::NullLiteral(_)) {
                        // `sunDirection: null` is `skyHasSun = false`.
                        let base = cap.strip_suffix("Direction").unwrap_or(&cap);
                        let Some(has) = setter_of(&format!("{prefix}Has{base}")) else {
                            bail!("{at}: {ns}.{prop}.{} cannot be null", key.name)
                        };
                        let off = mk(ExprKind::Bool(false), Ty::Bool);
                        stmts.push(Stmt::Eval(self.emit_extern(&has, vec![off])));
                        continue;
                    }
                    let Some(setter) = setter_of(&format!("{prefix}{cap}")) else {
                        bail!("{at}: {ns}.{prop} has no `{}`", key.name)
                    };
                    let want = self.ext_ir(&setter.params[0].1);
                    let v = self.lower_expr(&p.value, Some(&want))?;
                    let v = self.coerce_ext(v, &want)?;
                    stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![v])));
                }
                Ok(Some(stmts))
            }
            _ => bail!("{at}: {ns}.{prop} takes an object literal or null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-TS-3197
    #[test]
    fn window_helpers_come_with_programs_that_name_a_window() {
        let manifest = crate::manifest::Manifest::default();
        let own = "class Bus { on(f: () => void) {} emit(x: number) {} }\nconst b = new Bus(); b.on(() => {}); b.emit(1);";
        assert!(extend_source(own, &manifest).is_none_or(|s| !s.contains("__xm_Window_on")));
        let win = "const w = Window.main; w.on(\"x\", () => {}); w.emit(\"y\", 1);";
        let out = extend_source(win, &manifest).expect("the window chunks are added");
        assert!(out.contains("__xm_Window_on") && out.contains("__xm_Window_emit"));
        let broadcast = "function broadcast() {}\nbroadcast();";
        assert!(
            extend_source(broadcast, &manifest)
                .is_none_or(|s| !s.contains("__xt_Window_broadcast"))
        );
    }
}
