//! The typed window messages (`postMessage<T>`, `onMessage<T>`, `request<R>`, the named events
//! `emit` / `on` / `ask<R>` / `answer`, `Window.broadcast` / `emitAll`; docs/windows.md) over the
//! JSON natives of `iron_engine_api::window` (`postMessageJson`, `emitJson`, `requestJson`,
//! `askJson`, ..). The engine writes the event envelope, keeps the pending requests and settles
//! their promises, checks names and sizes; what is left for the compiler is the static type: it
//! writes, per program, functions that stringify the payload and parse what arrives as the type
//! of the call (`JSON.stringify` / `JSON.parse` of `T`, one promise per reply type `R`). The
//! lowerer picks them by name (`facade.rs`: `__xm_Window_<member>`, `..0` for a handler without
//! parameters, `..P` for one that returns a promise).

/// The raw messages: `postMessage`, `onMessage`, `onRequest`, `Window.broadcast`.
fn raw() -> String {
    let mut s = String::new();
    s += "\nfunction __xm_Window_postMessage<T>(self: Window, data: T): boolean {\n  return self.postMessageJson(JSON.stringify(data));\n}\n";
    s += "function __xt_Window_broadcast<T>(data: T): number {\n  return Window.broadcastJson(JSON.stringify(data));\n}\n";
    // A listener per receiver: an instance method (`__xm_`, on `self`) or the domain's (`__xt_`).
    for (prefix, owner, recv) in [
        ("__xm_Window_", "self: Window, ", "self"),
        ("__xt_Window_", "", "Window"),
    ] {
        s += &format!(
            "function {prefix}onMessage<T>({owner}f: (from: Window, data: T) => void): number {{\n  return {recv}.onMessageJson((from: Window, text: string) => {{\n    const data: T = JSON.parse(text);\n    f(from, data);\n  }});\n}}\n"
        );
        // `reply` answers once; an async handler's rejection fails the asker's promise.
        for (suffix, generics, ret, settle) in [
            ("", "<T, R, U = number>", "U", ";"),
            (
                "P",
                "<T, R>",
                "Promise<void>",
                ".then(() => {}, (e: Error) => { Window.replyError(id, e.message); });",
            ),
        ] {
            s += &format!(
                "function {prefix}onRequest{suffix}{generics}({owner}f: (from: Window, data: T, reply: (answer: R) => void) => {ret}): number {{\n  return {recv}.onRequestJson((from: Window, text: string, id: number) => {{\n    const data: T = JSON.parse(text);\n    f(from, data, (answer: R) => {{\n      Window.replyJson(id, JSON.stringify(answer));\n    }}){settle}\n  }});\n}}\n"
            );
        }
    }
    s
}

/// The payload text of an event body ("" when it has none, which decodes as `null`).
const BODY: &str = "JSON.parse(body === \"\" ? \"null\" : body)";

/// The named events: `emit`, `Window.emitAll`, `on`, `answer`.
fn named() -> String {
    let mut s = String::new();
    // The engine says what is wrong with a name; the typed calls throw it.
    s += "\nfunction __win_name(name: string): void {\n  const bad = Window.eventNameError(name);\n  if (bad !== \"\") throw new Error(bad);\n}\n";
    // With a payload (`T`), and without (`..0`, a dummy type parameter the lowerer fills in).
    for (suffix, generics, param, text) in [
        ("", "<T>", ", payload: T", "JSON.stringify(payload)"),
        ("0", "<U>", "", "\"\""),
    ] {
        s += &format!(
            "function __xm_Window_emit{suffix}{generics}(self: Window, name: string{param}): boolean {{\n  __win_name(name);\n  return self.emitJson(name, {text});\n}}\n"
        );
        s += &format!(
            "function __xt_Window_emitAll{suffix}{generics}(name: string{param}): number {{\n  __win_name(name);\n  return Window.emitAllJson(name, {text});\n}}\n"
        );
    }
    s += &format!(
        "function __xm_Window_on<T>(self: Window, name: string, f: (payload: T, from: Window) => void): number {{\n  __win_name(name);\n  return self.onEventJson(name, (from: Window, body: string) => {{\n    const data: T = {BODY};\n    f(data, from);\n  }});\n}}\n"
    );
    s += "function __xm_Window_on0<U>(self: Window, name: string, f: () => void): number {\n  __win_name(name);\n  return self.onEventJson(name, (from: Window, body: string) => {\n    f();\n  });\n}\n";
    // `answer`: the handler's value is the reply; one that returns a promise (`..P`) replies when it
    // resolves, and its rejection fails the `ask`.
    for (suffix, generics, params, decode, call) in [
        (
            "",
            "<T, R>",
            "payload: T, from: Window",
            true,
            "f(data, from)",
        ),
        ("0", "<R>", "", false, "f()"),
    ] {
        let data = if decode {
            format!("    const data: T = {BODY};\n")
        } else {
            String::new()
        };
        s += &format!(
            "function __xm_Window_answer{suffix}{generics}(self: Window, name: string, f: ({params}) => R): number {{\n  __win_name(name);\n  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {{\n{data}    const answer: R = {call};\n    Window.replyJson(id, JSON.stringify(answer));\n  }});\n}}\n"
        );
        s += &format!(
            "function __xm_Window_answer{suffix}P{generics}(self: Window, name: string, f: ({params}) => Promise<R>): number {{\n  __win_name(name);\n  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {{\n{data}    {call}.then((answer: R) => {{\n      Window.replyJson(id, JSON.stringify(answer));\n    }}, (e: Error) => {{ Window.replyError(id, e.message); }});\n  }});\n}}\n"
        );
    }
    s
}

/// The promise of a reply of type `ty`: settled by the engine's `resolve` (the reply's JSON text)
/// and `reject` (the reason) callbacks of `call`, the JSON native with its arguments written up to
/// the callbacks.
fn reply_promise(ty: &str, call: &str) -> String {
    format!(
        "  return new Promise<{ty}>((resolve, reject) => {{\n    {call}(text: string) => {{\n      const value: {ty} = JSON.parse(text);\n      resolve(value);\n    }}, (e: string) => {{\n      reject(new Error(e));\n    }});\n  }});\n"
    )
}

/// `request<R>(data, timeoutMs?)` for the reply type written `ty` (key `key`).
fn request(ty: &str, key: &str) -> String {
    format!(
        "\nfunction __xm_Window_request_{key}<T>(self: Window, data: T, timeoutMs?: number): Promise<{ty}> {{\n{}}}\n",
        reply_promise(
            ty,
            "self.requestJson$n(JSON.stringify(data), timeoutMs ?? NaN, "
        )
    )
}

/// `ask<R>(name, payload?, timeoutMs?)` for the reply type written `ty` (key `key`): the event text
/// function holds the promise; `ask0` has no payload (its type parameter is a dummy).
fn ask(ty: &str, key: &str) -> String {
    format!(
        "\nfunction __xm_Window_asktext_{key}(self: Window, name: string, payload: string, timeoutMs?: number): Promise<{ty}> {{\n  __win_name(name);\n{}}}\nfunction __xm_Window_ask_{key}<T>(self: Window, name: string, payload: T, timeoutMs?: number): Promise<{ty}> {{\n  return __xm_Window_asktext_{key}(self, name, JSON.stringify(payload), timeoutMs);\n}}\nfunction __xm_Window_ask0_{key}<U>(self: Window, name: string, timeoutMs?: number): Promise<{ty}> {{\n  return __xm_Window_asktext_{key}(self, name, \"\", timeoutMs);\n}}\n",
        reply_promise(ty, "self.askJson$n(name, payload, timeoutMs ?? NaN, ")
    )
}

/// Hash of a type's written text without its spaces: names the `request<R>` function of that type.
pub(crate) fn request_key(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes().filter(|b| !b.is_ascii_whitespace()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// The reply types written in `.request<R>(` / `.ask<R>(` calls of `src` (`call` is the text up to
/// `<`): the text between the angle brackets.
fn reply_types(src: &str, call: &str) -> Vec<String> {
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

/// Whether the program names a window member (`a&b`: every part; `|`: any form).
fn names(src: &str, triggers: &str) -> bool {
    src.contains("Window")
        && triggers
            .split('|')
            .any(|t| t.split('&').all(|p| src.contains(p)))
}

/// The functions a program that sends or listens for window messages needs (none for one that
/// does not name them).
pub fn glue(src: &str) -> String {
    // Only code counts: a string that mentions `.request<..>(` is no call.
    let code = super::literals::code_only(src);
    let src = code.as_str();
    let mut s = String::new();
    if names(
        src,
        "postMessage|onMessage|onRequest|.request<|.request(|broadcast",
    ) {
        s += &raw();
    }
    if names(
        src,
        ".emit(|.emit<|.on(|.on<|.ask(|.ask<|.answer(|.answer<|emitAll",
    ) {
        s += &named();
    }
    for r in reply_types(src, ".request<") {
        s += &request(&r, &request_key(&r));
    }
    for r in reply_types(src, ".ask<") {
        s += &ask(&r, &request_key(&r));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_type_gets_its_own_promise_function() {
        let src =
            "Window.main.request<{ n: number }>(1); Window.main.ask<Map<string, number[]>>(\"x\");";
        let out = glue(src);
        let k = request_key("{n:number}");
        assert!(
            out.contains(&format!("__xm_Window_request_{k}<T>")),
            "{out}"
        );
        assert!(out.contains("Promise<{ n: number }>"), "{out}");
        assert!(out.contains("Promise<Map<string, number[]>>"), "{out}");
    }
}
