# Research: networking and file access for game scripts

Status: proposal (2026-10-17). Nothing here is built yet. It covers how game scripts should get
`fetch`, WebSockets, raw sockets (or not), and safe file access, with web/Bun-shaped APIs that are
implemented and policed in Rust.

Grounded in: [DESIGN.md](../../DESIGN.md) §3, §4.3 ("no filesystem or network access by default"),
§8.3 (multiplayer transport), [scripting.md](../scripting.md), `crates/iron_script/src`
(`quickjs.rs`, `prelude.js`), `js/engine/src`, `iron_core::resolve_asset`, and `App::tick` in
`crates/iron_app/src/main.rs`.

## 1. Recommendation in short

| Area | Recommendation |
|---|---|
| Async bridge | Promises live in JS. JS allocates a request id, sends an op, keeps `{resolve, reject}` in a `Map`. Rust finishes the work on worker threads and, once per frame, calls one global `__iron_io(events)` with every completion. Same pattern as timers. Do not use rquickjs `futures` / `Ctx::spawn`. |
| Binary data | Add `Value::Bytes(Vec<u8>)` to `iron_core::Value`, mapped to `ArrayBuffer` / `Uint8Array`. |
| Runtime | One `tokio` multi-thread runtime with 1 to 2 workers, created lazily on the first IO op, in a new `iron_io` crate. QUIC (`quinn`) is tokio-based, so this is not a throwaway choice. |
| HTTP | `reqwest` 0.13 (rustls + platform verifier by default), custom DNS resolver for SSRF pinning, our own redirect loop. |
| WebSocket | `tokio-tungstenite` 0.30 over the same runtime and TLS config. |
| TLS | rustls 0.23 everywhere, with `rustls-platform-verifier` for OS trust stores. No native-tls. |
| Network policy | Off unless the game's manifest lists hosts. HTTPS/WSS only for shipped games. Always block IP literals and private, loopback, link-local and CGNAT addresses, judged on resolved addresses. Loopback allowed only in dev runs. Size, time, count and rate limits. Forbidden-header list. |
| Raw TCP/UDP | Do not expose to scripts. Multiplayer stays engine-side (Steam / QUIC, `@sync` / `@rpc`). |
| Files | Two roots: `game` (read-only, the game folder or pack) and `data` (read-write, per game, in the OS app-data dir). Paths go through `resolve_asset` plus stricter name rules; the data root is opened with `cap-std` so symlinks and `..` cannot escape. Quotas and atomic writes. Async API shaped like `Bun.file` / `Bun.write`. |
| Order | Bridge, then files, then fetch, then WebSocket. |

## 2. What exists today

- `__iron_op(name, ...args)` pushes an `Op` onto `Shared.ops`. It returns nothing. `App::tick`
  drains ops once per frame (after component updates and timers) and matches on `op.name`.
- Rust calls JS with `call_global`, which ends with `run_jobs()`, so promise reactions and
  microtasks run before control returns to Rust.
- Timers live entirely in JS (`prelude.js`). Rust only calls `__iron_fire_next_timer(now)`. Rust
  never holds a JS function.
- `Value` is `Null | Bool | Number | String | Array`. There is no byte type and no object type.
- JS allocates ids for things it creates (stylesheets in `addStyleSheet`).
- `resolve_asset(root, url)` refuses empty paths, `:` (schemes, drive letters, NTFS streams),
  leading `/` or `\`, and any component other than a normal name or `.`. It does not check
  symlinks, Windows reserved names (`CON`, `NUL`), trailing dots or spaces, or case clashes.
- The TypeScript config uses `lib: ["ES2022"]` with no DOM types, so `fetch`, `Headers`,
  `WebSocket`, `URL`, `TextEncoder` and `AbortController` have no types and no implementation in
  QuickJS today.

The async design below adds to this; it does not change any of it.

## 3. The async bridge

### 3.1 Shape

```
JS (script thread)                      Rust main thread                 IO threads (tokio)
──────────────────                      ────────────────                 ──────────────────
p = asyncOp("io.fetch", ...)
  id = nextId++
  pending.set(id, {resolve, reject})
  __iron_op("io.fetch", id, ...)  ──▶  drain_ops() ─▶ IoHost::submit ─▶  policy check, request
                                                                          ...
                                        IoHost::poll() ◀── mpsc ◀──────── Completion{id, result}
                                        call_global("__iron_io", [batch])
  __iron_io(batch): for each event,
    pending.get(id).resolve(value)
  run_jobs(): .then() callbacks run
```

- **JS owns the promise.** `asyncOp` lives in `@iron/engine` (or the prelude). It allocates an id,
  stores the resolvers in a `Map`, and sends a normal op. Nothing new is needed in the
  `ScriptRuntime` trait.
- **Rust owns the work.** `IoHost` (new crate `iron_io`) takes ops whose name starts with `io.`,
  runs them on the tokio runtime, and sends results back through a `std::sync::mpsc` (or
  `crossbeam-channel`) channel. Worker threads never touch QuickJS.
- **One crossing per frame.** `IoHost::poll()` drains the channel. The app calls
  `__iron_io(events)` once with a flat array of events. `call_global` then runs microtasks, so
  `await fetch(...)` continuations run in the same frame.
- **Cancellation** is an op too: `io.abort(id)`. Rust keeps a `HashMap<id, AbortHandle>` and drops
  the task. JS rejects the promise with `AbortError` straight away; a late completion for an
  unknown id is ignored.
- **Hot reload.** `reset()` throws away the JS `Map`. `IoHost` keeps a generation number. On reset
  it aborts every task and bumps the generation, and completions from an older generation are
  dropped.

### 3.2 Where it fits in the frame

Proposed order inside `App::tick`:

1. `io.poll()` then `__iron_io(events)`: promises resolve and microtasks run.
2. Component updates (fixed 60 Hz loop, as today).
3. `scripts.tick(now)`: timers.
4. Drain commits, then drain ops. `io.*` ops go to `IoHost::submit`.

Latency is at most one frame on each side. Step 1 has a budget: at most N events (say 512) or
about 2 ms of work per frame. Anything left waits for the next frame, so a burst of WebSocket
messages cannot cause a hitch.

### 3.3 Why not rquickjs promises driven from Rust

rquickjs 0.14 has `Promise::new(ctx)` returning `(promise, resolve, reject)`, `Persistent<T>` to
keep JS values across `with` calls, and, behind the `futures` feature, `Ctx::spawn`,
`Promise::wrap_future`, `AsyncRuntime` and `AsyncContext` (checked in the vendored
`rquickjs-core-0.14.0` source). They are not a good fit here:

- Rust would hold JS functions (`Persistent<Function>`) across frames. That breaks the core rule
  that Rust never holds engine references into JS, and makes `reset()` harder (persistents must not
  outlive their runtime).
- The `futures` feature moves the runtime to `AsyncRuntime` / `AsyncContext`, with async locks and
  a different driving model (`idle()`), which does not match the frame-driven, single-thread
  design.
- It ties the bridge to rquickjs. The planned `deno_core` backend has its own op and promise system.
  A JS-side promise table works the same on every backend and can be pinned by the conformance
  suite.

The only native additions should be pure helpers with no engine access (see §6.1).

### 3.4 Binary data

`Value` has no bytes, and base64 strings would cost a third more memory and two copies. Add
`Value::Bytes(Vec<u8>)`:

- `from_js`: `ArrayBuffer`, `Uint8Array` (and other typed array views as their bytes) become `Bytes`.
- `to_js`: `Bytes` becomes an `ArrayBuffer`. `rquickjs::ArrayBuffer::new(ctx, Vec<u8>)` takes
  ownership of the vector, so this does not copy.
- Add conformance tests: round trip, empty buffer, typed array view with an offset.

This also helps later work (images from scripts, binary messages, save files).

### 3.5 Op and event shapes

All ids are JS-allocated numbers. Headers travel as flat `[name, value, name, value, ...]` lists,
like styles (TODO.md D2).

| Op (JS → Rust) | Arguments |
|---|---|
| `io.fetch` | `id, url, method, headers[], body (string \| bytes \| null), opts[]` where opts is `[redirect, timeoutMs]` |
| `io.abort` | `id` |
| `io.ws.open` | `id, url, protocols[], headers[]` |
| `io.ws.send` | `id, data (string \| bytes)` |
| `io.ws.close` | `id, code, reason` |
| `io.file.read` | `id, root ("game" \| "data"), path, as ("text" \| "bytes")` |
| `io.file.write` | `id, root, path, data (string \| bytes)` |
| `io.file.stat` / `io.file.list` / `io.file.delete` | `id, root, path` |

Events (Rust → JS), one batch per frame: `__iron_io([[...], [...]])`.

| Event | Fields |
|---|---|
| Success | `["ok", id, value...]`. For fetch: `status, statusText, finalUrl, redirected, headers[], body bytes`. For files: the data, stat fields or list. |
| Failure | `["err", id, kind, message]`. `kind` is `"blocked"`, `"network"`, `"timeout"`, `"too_large"`, `"not_found"`, `"quota"`, `"invalid"`. |
| WebSocket | `["ws", id, "open", protocol]`, `["ws", id, "message", data]`, `["ws", id, "close", code, reason, wasClean]`, `["ws", id, "error", message]`, `["ws", id, "buffered", bytes]` |

JS turns `blocked` into a `TypeError` whose message names the rule (web `fetch` rejects with
`TypeError` on network errors; Deno uses `NotCapable` for refused permissions). Programming errors
(bad header name, bad close code) throw synchronously in JS before any op is sent, as browsers do.

Later, when the op set settles, these can move to the binary op encoding planned in
`M2-BEN-04`.

## 4. Rust crate choices

Versions are the latest stable on crates.io on 2026-10-17 (from the crates.io API). `tokio`
1.53.3 is already in `Cargo.lock`.

| Crate | Version | Role |
|---|---|---|
| [tokio](https://crates.io/crates/tokio) | 1.53.3 | Async runtime for IO threads |
| [reqwest](https://crates.io/crates/reqwest) | 0.13.5 | HTTP client |
| [hyper](https://crates.io/crates/hyper) / hyper-util | 1.12.0 / 0.1.21 | Under reqwest; also for test servers |
| [ureq](https://crates.io/crates/ureq) | 3.4.2 | Alternative blocking client |
| [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite) / [tungstenite](https://crates.io/crates/tungstenite) | 0.30.0 | WebSocket |
| [rustls](https://crates.io/crates/rustls) | 0.23.46 | TLS |
| [rustls-platform-verifier](https://crates.io/crates/rustls-platform-verifier) | 0.7.1 | OS certificate verification |
| [webpki-roots](https://crates.io/crates/webpki-roots) | 1.0.9 | Bundled Mozilla roots (fallback) |
| [aws-lc-rs](https://crates.io/crates/aws-lc-rs) / [ring](https://crates.io/crates/ring) | 1.18.2 / 0.17.14 | rustls crypto providers |
| [native-tls](https://crates.io/crates/native-tls) | 0.2.18 | Not recommended (see below) |
| [cap-std](https://crates.io/crates/cap-std) | 4.0.3 | Capability-based directory access for the data root |
| [url](https://crates.io/crates/url) | 2.5.8 | URL parsing in Rust |
| [ipnet](https://crates.io/crates/ipnet) | 2.12.2 | IP range tables for the SSRF guard |
| [wiremock](https://crates.io/crates/wiremock) | 0.6.5 | Optional HTTP mocking in tests |

### 4.1 tokio or a small thread pool

| | tokio runtime (1 to 2 workers) | Thread pool + blocking IO |
|---|---|---|
| Many WebSockets | One task each, cheap | One thread each (blocking reads) |
| Timeouts, abort | `tokio::time::timeout`, `AbortHandle` | Socket timeouts, awkward abort |
| HTTP/2, pooling | Yes (reqwest) | ureq is HTTP/1.1 only |
| Size and compile time | Larger | Smaller |
| Future multiplayer | `quinn` (DESIGN §8.3) is tokio-based anyway | A second runtime arrives later |

Recommendation: **tokio**, multi-thread flavour with 1 to 2 workers, built lazily on the first
`io.*` op so games without IO start no threads. Put networking behind a cargo feature (`net`) on
`iron_io` / `iron_app` so size can be measured and the feature dropped for builds that do not need
it. File IO can use `tokio::task::spawn_blocking` on the same runtime.

### 4.2 reqwest, ureq or hyper

- **reqwest 0.13**: default features are `default-tls` (now `rustls`, with the aws-lc-rs provider and
  `rustls-platform-verifier`), `charset`, `http2`, `system-proxy` (checked in the 0.13.5
  `Cargo.toml`). Its `ClientBuilder` has what the policy needs: `dns_resolver` (custom resolver),
  `redirect(Policy::none())`, `https_only`, `no_proxy`, `connect_timeout`, `read_timeout`, and
  `tls_backend_rustls` / `tls_backend_preconfigured`. **Recommended.**
- **ureq 3**: small, blocking, rustls by default, and has a `Resolver` trait. Good if we chose the
  thread-pool route; worse for many concurrent requests and no HTTP/2.
- **hyper 1** directly: most control and smallest on top of tokio, but we would rewrite pooling,
  redirects, decompression and proxies. Not worth it for a first version.

### 4.3 WebSocket

`tokio-tungstenite` 0.30 shares the runtime. Connect the TCP stream ourselves (to an address the
policy approved, see §5.3), wrap it in `tokio-rustls` with the same `ClientConfig` as reqwest, then
call `client_async_with_config` with `max_message_size` / `max_frame_size` set from the policy.
Plain `tungstenite` would need a thread per socket.

### 4.4 TLS

- **rustls everywhere.** Same behaviour on every platform, no OpenSSL on Linux or Android, and
  reqwest's default already.
- **Certificate verification:** `rustls-platform-verifier` uses the OS verifier on Windows, macOS
  and iOS (Security.framework) and Android (Trust Manager, API 24+), and webpki with the system CA
  bundle on Linux ([README](https://github.com/rustls/rustls-platform-verifier)). On Android it
  needs a small Kotlin component added through Gradle, a ProGuard keep rule, and an
  `init_with_env` call before any networking. Plan for that in the Android wrapper.
- **Crypto provider:** aws-lc-rs is the reqwest default. It builds C code, which can need extra
  tools (CMake, a C compiler, sometimes NASM) when cross-compiling, notably for Android and iOS.
  `ring` is usually easier to cross-compile. reqwest's `rustls-no-provider` feature lets us install
  `ring` ourselves if aws-lc-rs gets in the way. Decide when the Android/iOS work starts; the API
  does not change.
- **native-tls:** different behaviour per OS (SChannel, Secure Transport, OpenSSL), and OpenSSL is
  a pain on Linux and Android. Not recommended.

### 4.5 Platform notes

| Platform | Notes |
|---|---|
| Windows / Linux / macOS | No special setup with rustls + platform verifier. |
| Android | Verifier Kotlin component and init (above). `INTERNET` permission in the manifest. |
| iOS | No JIT is unrelated here. Blocking LAN addresses (§5) also avoids the iOS local network permission prompt. Downloadable games are already an App Store policy risk (DESIGN §10). |
| All | Measure binary size with `cargo bloat` when phase 3 lands; keep `net` an optional feature. |

## 5. Security model

### 5.1 What others do

**s&box** (source: [Facepunch/sbox-public](https://github.com/Facepunch/sbox-public), files
`engine/Sandbox.Engine/Utility/Web/Http.cs`, `Utility/WebSocket.cs`,
`Tests/Sandbox.Test.Engine/Web/Http.cs`):

- `Http.IsAllowed(uri)`: scheme must be `http`, `https`, `ws` or `wss`; IP literals are refused;
  a host that resolves to any private, loopback or link-local address (including `0.0.0.0`,
  `169.254.169.254`, `100.64.0.0/10`, `fd00::/8`, IPv4-mapped forms) is refused. The comment cites
  DNS rebinding attacks on home routers. The decision is made on the **resolved addresses**, not
  the host text.
- Loopback is allowed only on ports 80, 443, 8080 and 8443, and only when every resolved address is
  loopback. All local addresses are allowed in standalone exports, and in the editor or dedicated
  servers with `-allowlocalhttp`.
- Redirects are followed manually (max 50) so each hop is checked again; `Authorization` is cleared
  on redirect; HTTPS to HTTP downgrades stop the chain. Auto-redirect is off because the .NET
  handler "bypasses DelegatingHandler on redirects, allowing SSRF".
- A forbidden-header list based on the browser one (Host, Content-Length, Origin, Referer, ...,
  `Proxy-*`, `Sec-*`), but Cookie is allowed. User-Agent and Referer are fixed by the engine.
- One shared `HttpClient`, no cookie persistence, async only (sync `Send` throws).
- `WebSocket` runs the same `IsAllowed` and header checks, has a 64 KiB default max message size,
  and compression is off by default (the doc comment warns it can be dangerous with secrets).
- There is **no per-game host allow-list**: any public host is reachable.
- Files ([docs](https://sbox.game/dev/doc/assets/file-system.md)): `System.IO.File` is blocked.
  `FileSystem.Mounted` is mounted game content, `FileSystem.Data` is a per-game read-write folder
  under `sbox/data/<org>/<game>/`, `FileSystem.OrganizationData` is shared across one
  organisation's games, and `FileSystem.Cache` is a key store that "may be deleted at any time".
  Helpers: `ReadAllText`, `WriteAllText`, `ReadJson<T>`, `WriteJson`, `FileExists`. The docs do not
  mention quotas.
- [AuthTokens](https://wiki.facepunch.com/sbox/AuthTokens) let a game's backend check that an HTTP
  or WebSocket request came from a real Steam user.

**Garry's Mod** ([HTTP](https://wiki.facepunch.com/gmod/Global.HTTP)) blocks private addresses
including 127.0.0.1 unless the server runs with `-allowlocalhttp`.

**Browsers:** no raw sockets; a list of
[forbidden request headers](https://developer.mozilla.org/en-US/docs/Glossary/Forbidden_header_name);
mixed content blocking (HTTPS pages cannot load HTTP); CORS (does not apply to Iron: there is no
web origin to protect, so header and address rules replace it); redirect limit of 20 in the
[Fetch standard](https://fetch.spec.whatwg.org/#http-redirect-fetch); WebSocket
[`close()`](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/close) only accepts code 1000
or 3000 to 4999 from scripts.

**Deno** ([security](https://docs.deno.com/runtime/fundamentals/security/)): sandboxed by default;
`--allow-net=host[:port]`, `--allow-read=path`, `--allow-write=path`, `--deny-*` overrides, an
interactive `y/n/A` prompt on a terminal, `NotCapable` errors, and `Deno.permissions.query /
request / revoke`.

**Bun:** no sandbox; full Node-style access. Its API shapes are worth copying, its trust model is
not.

### 5.2 Iron's network policy

Threat model: a downloaded game, written by someone the player does not know. Risks are attacks on
the player's LAN (router admin pages, cloud metadata at `169.254.169.254`), using players as a
botnet, leaking data to unknown hosts, and resource abuse.

1. **Off by default** (DESIGN §4.3). A game opts in through its manifest (Iron has none yet; a
   `game.json` or an `iron` block in `package.json` would also carry the org/game ident used for the
   data folder):
   ```json
   { "ident": "acme.spacegame",
     "network": { "hosts": ["api.acme.com", "*.cdn.acme.com"] } }
   ```
   Wildcards match one or more labels under a domain, never a bare TLD. The launcher or Workshop
   page shows the list to the player, like Android permissions or browser extension host access.
   This is stricter than s&box and suits "downloadable games". A `"hosts": ["*"]` option can exist
   but should be visible as "any website".
2. **No runtime prompts** in the first version. Prompts work poorly in a fullscreen game with a
   gamepad. The manifest list is the prompt. Add an editor/launcher permission screen later if
   needed.
3. **Schemes:** `https` and `wss` only for shipped or downloaded games. `http` and `ws` only to
   loopback in dev runs (an unpacked game folder started with `iron <folder>`, or the editor).
4. **Address guard, always on**, even for allowed hosts, judged on resolved addresses:
   - refuse IP literals in URLs (except loopback in dev runs);
   - refuse if **any** resolved address is loopback, unspecified (`0.0.0.0`, `::`), private
     (10/8, 172.16/12, 192.168/16, fc00::/7), link-local (169.254/16, fe80::/10), CGNAT (100.64/10),
     multicast, broadcast, documentation or reserved ranges, or the IPv4-mapped forms of these;
   - dev runs may reach loopback on any port, with a log line.
5. **DNS pinning.** s&box resolves once to check and the HTTP stack resolves again to connect,
   which leaves a small DNS rebinding window. Iron should resolve once inside a custom
   `reqwest::dns::Resolve`, filter the addresses, and return only approved ones, so the
   connection goes to exactly what was checked. The WebSocket path connects its own `TcpStream`
   to the approved address.
6. **Proxies:** use `no_proxy()` by default. A system proxy resolves names itself, which skips the
   address guard. Revisit if players behind proxies report problems.
7. **Redirects:** `Policy::none()` in reqwest plus our own loop: max 10 hops, re-run host and
   address checks each hop, stop on HTTPS to HTTP, drop `Authorization` and `Cookie` when the
   origin changes, apply the 301/302/303 method rules. `redirect: "manual" | "error"` from JS are
   honoured.
8. **Headers:** reject the browser forbidden list (as s&box: `Host`, `Content-Length`,
   `Connection`, `Transfer-Encoding`, `Upgrade`, `TE`, `Trailer`, `Keep-Alive`, `Expect`, `Via`,
   `Origin`, `Referer`, `Date`, `DNT`, `Accept-Charset`, `Accept-Encoding`, `Access-Control-Request-*`,
   `Proxy-*`, `Sec-*`). The engine sets `User-Agent: Iron/<version> (<game ident>)`. No cookie jar;
   a game can set `Cookie` itself.
9. **Limits** (defaults; the manifest may raise some within hard caps):

   | Limit | Default |
   |---|---|
   | Response body (after decompression) | 16 MiB; cap 64 MiB |
   | Request body | 4 MiB |
   | Total request time / connect time | 30 s / 10 s |
   | Concurrent requests | 6 per host, 32 total |
   | Request rate | token bucket, 20/s burst 40 |
   | Open WebSockets | 8 |
   | WebSocket incoming message | 1 MiB (s&box: 64 KiB) |
   | WebSocket outgoing buffer (`bufferedAmount`) | 4 MiB, then `send` throws |
   | Response headers | 64 KiB |

   Count decompressed bytes so gzip or brotli bombs stop at the limit. Compression for WebSocket
   (permessage-deflate) off, as in s&box.
10. **Logging:** every request and socket is logged at `debug` under target `net` with the game
    ident, and denials at `warn`, so a future editor network panel can show them.

### 5.3 File policy

Two roots, named like s&box's `Mounted` and `Data`:

| Root | Where | Access |
|---|---|---|
| `game` | The game folder today; the pack file later (DESIGN §6) | Read-only |
| `data` | Per game: `<OS app data>/Iron/<org>/<game>/` (`%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `$XDG_DATA_HOME` on Linux, the app's files dir on Android/iOS) | Read-write |

Later, maybe: `cache` (may be deleted any time, s&box `Cache`) and an org-shared root. Not in the
first version.

Rules:

- **Paths** are relative, `/`-separated, and go through `resolve_asset` first (no schemes, drive
  letters, absolute paths or `..`). Then stricter checks for the `data` root: each name 1 to 128
  bytes from `[A-Za-z0-9 ._-]`, no leading or trailing space or dot, no Windows reserved names
  (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`, with any extension), depth at most 16.
  Lower-case names on write (s&box normalises case too) so saves move between Windows, macOS and
  Linux without clashes.
- **No symlink escapes.** Open the data root as a `cap_std::fs::Dir` and do every operation
  relative to it; cap-std refuses paths that leave the directory, including through symlinks. For
  the read-only `game` root, also refuse symlinks (`symlink_metadata`), or canonicalise and check
  the prefix. Packs remove the problem later.
- **Quotas:** 100 MiB and 10,000 files per game by default, 16 MiB per file. Keep a running total
  in memory (scan once on first use). A write over quota fails with `quota`.
- **Atomic writes:** write `name.tmp`, flush, rename over `name`. A crash never leaves half a save.
- **Async only.** Reads and writes run on `spawn_blocking`. Save files are small, so a frame of
  latency is fine.
- **No file watching, no file handles, no append** at first.

Reuse: `resolve_asset` stays the first gate for both roots, and a new
`iron_core::resolve_data_path` adds the stricter name rules, so CSS `url()`, `loadFont` and the
file API share one definition of "inside the game".

## 6. JS API proposal (`@iron/engine`)

Goal: code written for the browser or Bun ports with few changes. Install `fetch`, `Headers`,
`Response`, `Request`, `WebSocket`, `AbortController`, `AbortSignal`, `URL`, `URLSearchParams`,
`TextEncoder`, `TextDecoder`, `EventTarget` and `Event` as globals when `@iron/engine` loads (like
the `__iron_*` hooks), so npm libraries that call `fetch` work. Declare their types in
`globals.ts` (the project has no DOM lib), and declare only what is implemented.

### 6.1 Support classes

- `TextEncoder` / `TextDecoder` (UTF-8 only): two small native functions in `install_globals`
  (`__iron_utf8_encode`, `__iron_utf8_decode`) wrapped in JS classes. Pure helpers, no engine
  access.
- `URL` / `URLSearchParams`: QuickJS has neither. Options: a small JS implementation in
  `@iron/engine` (enough for parsing, `searchParams`, `href`), or a polyfill package bundled by
  Bun. Rust re-parses with the `url` crate anyway, so the JS one only needs to be convenient, not
  authoritative.
- `EventTarget`, `Event`, `MessageEvent`, `CloseEvent`, `AbortController`, `AbortSignal`
  (`abort()`, `aborted`, `reason`, `AbortSignal.timeout(ms)` on engine time).

### 6.2 fetch

```ts
type BodyInit = string | ArrayBuffer | ArrayBufferView | URLSearchParams;
type HeadersInit = Headers | Record<string, string> | [string, string][];

interface RequestInit {
  method?: string;              // default "GET"
  headers?: HeadersInit;
  body?: BodyInit | null;
  signal?: AbortSignal;
  redirect?: "follow" | "error" | "manual";
  // Accepted for portability, ignored: mode, credentials, cache, referrer, keepalive.
}

declare function fetch(input: string | URL | Request, init?: RequestInit): Promise<Response>;

declare class Headers implements Iterable<[string, string]> {
  constructor(init?: HeadersInit);
  append(name: string, value: string): void;
  set(name: string, value: string): void;
  get(name: string): string | null;
  has(name: string): boolean;
  delete(name: string): void;
  forEach(cb: (value: string, name: string) => void): void;
  entries(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
}

declare class Response {
  constructor(body?: BodyInit | null, init?: { status?: number; statusText?: string; headers?: HeadersInit });
  readonly status: number;
  readonly statusText: string;
  readonly ok: boolean;
  readonly headers: Headers;
  readonly url: string;
  readonly redirected: boolean;
  readonly bodyUsed: boolean;
  text(): Promise<string>;
  json<T = unknown>(): Promise<T>;
  arrayBuffer(): Promise<ArrayBuffer>;
  bytes(): Promise<Uint8Array>;
  clone(): Response;
  static json(data: unknown, init?: ResponseInit): Response;
  static error(): Response;
}

declare class Request {
  constructor(input: string | URL | Request, init?: RequestInit);
  readonly url: string;
  readonly method: string;
  readonly headers: Headers;
  readonly signal: AbortSignal;
  text(): Promise<string>;
  json<T = unknown>(): Promise<T>;
  arrayBuffer(): Promise<ArrayBuffer>;
}
```

The body arrives whole (within the size limit) and `Response` wraps it, so `text()` and friends
resolve at once. Non-2xx responses resolve normally, as on the web; only network, policy, timeout
and size failures reject.

### 6.3 WebSocket

```ts
declare class WebSocket extends EventTarget {
  static readonly CONNECTING: 0; static readonly OPEN: 1;
  static readonly CLOSING: 2;    static readonly CLOSED: 3;
  // `options.headers` follows Bun's client extension; checked against the forbidden list.
  constructor(url: string | URL, protocols?: string | string[] | { protocols?: string | string[]; headers?: HeadersInit });
  readonly url: string;
  readonly readyState: 0 | 1 | 2 | 3;
  readonly protocol: string;
  readonly bufferedAmount: number;   // updated from "buffered" events
  binaryType: "arraybuffer";         // "blob" is not supported
  send(data: string | ArrayBuffer | ArrayBufferView): void;
  close(code?: number, reason?: string): void;  // code 1000 or 3000-4999, reason <= 123 UTF-8 bytes
  onopen: ((ev: Event) => void) | null;
  onmessage: ((ev: MessageEvent<string | ArrayBuffer>) => void) | null;
  onerror: ((ev: Event) => void) | null;
  onclose: ((ev: CloseEvent) => void) | null;  // code, reason, wasClean; 1006 when the link drops
}
```

Browser behaviour to keep: the constructor never throws for network reasons (it fails later with
`error` then `close` 1006); a bad URL or a host not in the manifest throws `SyntaxError` /
`TypeError` synchronously; `send` while `CONNECTING` throws `InvalidStateError`.

### 6.4 Files

Shaped like `Bun.file` / `Bun.write` ([Bun file I/O](https://bun.sh/docs/api/file-io)), but scoped
to a root, and all async (Bun's `size` is a sync property; ours is `stat()`):

```ts
interface IronFile {
  readonly name: string;                 // path within its root
  exists(): Promise<boolean>;
  stat(): Promise<{ size: number; modified: number; isDirectory: boolean }>;
  text(): Promise<string>;
  json<T = unknown>(): Promise<T>;
  bytes(): Promise<Uint8Array>;
  arrayBuffer(): Promise<ArrayBuffer>;
}

interface FileEntry { name: string; isDirectory: boolean; size: number; modified: number }

interface ReadonlyRoot {
  file(path: string): IronFile;
  list(dir?: string): Promise<FileEntry[]>;
}

interface WritableRoot extends ReadonlyRoot {
  /** Like Bun.write: replaces the file atomically, creates parent folders, returns bytes written. */
  write(path: string, data: string | ArrayBuffer | ArrayBufferView): Promise<number>;
  delete(path: string): Promise<void>;   // files and empty folders
  readJson<T>(path: string, fallback?: T): Promise<T>;
  writeJson(path: string, value: unknown, options?: { pretty?: boolean }): Promise<number>;
  /** Bytes and files used against this game's quota. */
  usage(): Promise<{ bytes: number; files: number; quotaBytes: number; quotaFiles: number }>;
}

export declare const files: { readonly game: ReadonlyRoot; readonly data: WritableRoot };
```

```ts
import { files } from "@iron/engine";
const save = await files.data.readJson("save.json", { level: 1 });
await files.data.writeJson("save.json", { ...save, level: save.level + 1 });
const intro = await files.game.file("text/intro.txt").text();
```

### 6.5 Left out on purpose

| Left out | Why |
|---|---|
| `ReadableStream` bodies, `Blob`, `File`, `FormData`, `EventSource` | Size and complexity; add streaming in a later phase if a game needs it |
| `XMLHttpRequest` | Old API; `fetch` covers it |
| Cookie jar, `credentials` | No origin to tie cookies to; games can send headers |
| `node:fs`, `node:net`, `Bun.*` globals, absolute paths, process spawning | Sandbox (DESIGN §4.3) |
| Sync file APIs | Keep the frame free of disk waits; revisit for tiny settings files if needed |
| File handles, append, watch | Not needed for saves; hot reload watching is engine-side |
| Raw `TCPSocket` / `UDPSocket` | See §7 |

## 7. Raw TCP and UDP

**Recommendation: do not expose raw sockets to scripts.**

- They defeat the address guard's purpose: port scanning the LAN, talking to routers and printers,
  and sending UDP floods from players' machines. Browsers never exposed them (the Direct Sockets
  API is limited to Isolated Web Apps), and s&box does not either (its scripts only get the `Http`
  and `WebSocket` classes).
- No TLS by default, and on iOS LAN traffic triggers the local network permission prompt.
- Multiplayer has its own plan (DESIGN §8.3, TODO "Later"): Steam networking and QUIC (`quinn`)
  behind a transport trait, with `@sync` / `@rpc` in the component model. Scripts get messages and
  state, not sockets. That transport can also offer a "custom message" channel for game data
  without opening arbitrary sockets.
- If a game needs a custom server protocol, WebSocket to an allow-listed host covers most cases.
  If that proves too slow later, the next step should be a QUIC/WebTransport-style channel to
  allow-listed hosts, built on the multiplayer transport, not raw sockets.
- LAN discovery for local multiplayer, if wanted, belongs in the engine's transport, not scripts.

## 8. Testing

**Rust, policy as pure functions** (no network, like s&box's `IsResolvedAllowed` tests):
`is_address_allowed(addr)`, `check_url(url, addrs, policy)`, header checks, redirect rules
(downgrade, auth stripping, hop limit), host wildcard matching, file name rules, reserved names.
Table-driven, one row per case.

**Rust, local servers** (integration tests in `iron_io/tests`):
- An HTTP server on `127.0.0.1:0` built with hyper (or wiremock) in the test, with a policy that
  allows loopback (`Policy::for_tests()`), since the real policy blocks it. Cover status codes,
  headers, large bodies over the limit, gzip bombs, slow servers (timeouts), redirects (including
  to a blocked address and HTTPS to HTTP), aborts.
- TLS with a self-signed cert from `rcgen`, trusted by a test-only root store, so the TLS path is
  tested without the internet.
- A `tokio-tungstenite` echo server: text and binary, close codes, oversized messages, server
  drop (1006), backpressure.
- A resolver stub that returns chosen addresses, to test DNS pinning and rebinding (first answer
  public, second private).

**Fake transport.** `IoHost` takes a `Transport` trait object. A `FakeTransport` returns scripted
results deterministically on the next `poll()`. The script conformance suite and JS-level tests
use it, so they never open sockets.

**Script conformance** (`runtime_conformance_tests!`): promise resolves on the next poll and its
`.then` runs before `call_global` returns; reject paths; abort; `reset()` drops in-flight work and
late completions are ignored; `Value::Bytes` round trips; the per-frame event budget.

**JS** (`bun test`): `Headers`, `Response`, `URL`, `TextEncoder`, `WebSocket` state machine and
close-code validation, with a fake `__iron_op` and hand-fed `__iron_io` batches. Note that Bun's
own globals (`fetch`, `WebSocket`) exist under `bun test`; tests must import Iron's
implementations explicitly, not use the globals.

**Files:** temp-dir tests for traversal attempts, reserved names, quotas, atomic writes (kill
between write and rename leaves the old file), and symlink escapes. Creating symlinks on Windows
needs Developer Mode or admin, so use a junction on Windows or mark that case unix-only.

**No live internet in CI.** At most one ignored test (`#[ignore]`) that fetches a public URL, for
manual runs.

Suggested test IDs: `IO-BRG-*` (bridge), `IO-FS-*` (files), `IO-NET-*` (fetch and policy),
`IO-WS-*` (WebSocket), under whichever milestone this lands in.

## 9. Phased plan

| Phase | Work | Size | Depends on |
|---|---|---|---|
| 1. Bridge | `Value::Bytes`; `asyncOp` and the `__iron_io` batch in `@iron/engine`; `iron_io` crate with `IoHost`, generations, event budget, `FakeTransport`; TextEncoder/TextDecoder natives; conformance tests; wire into `App::tick` | S to M (2 to 3 days) | Nothing |
| 2. Files | Game manifest with `ident`; `resolve_data_path`; `game` and `data` roots with cap-std; quotas; atomic writes; `files` API and types; an example that saves settings | M (3 to 4 days) | 1 |
| 3. fetch | tokio runtime (lazy), reqwest client, policy (manifest hosts, address guard, pinned resolver, redirects, headers, limits); `fetch`/`Headers`/`Request`/`Response`/`AbortController`/`URL`; local-server and TLS tests; `cargo bloat` check | M to L (1 to 1.5 weeks) | 1 |
| 4. WebSocket | tokio-tungstenite on the pinned connector; events, binary, close codes, backpressure, limits; echo-server tests; a chat example against a local server | M (4 to 5 days) | 3 |
| 5. Later | Streaming bodies, `Blob`/`FormData`, player-facing permission screen, editor network panel, Steam auth tickets for game backends, `cache` root, org-shared data | As needed | 3, 4 |

Files come before networking: saves are needed by almost every game, and the risk is lower.

## 10. Risks

| Risk | Mitigation |
|---|---|
| Binary size and compile time (tokio, hyper, rustls, aws-lc-rs) | `net` cargo feature; measure with `cargo bloat`; tokio arrives with `quinn` anyway |
| aws-lc-rs cross-compiling for Android/iOS | Switch to `ring` via `rustls-no-provider` without API changes |
| Android certificate verification setup | Do it in the Android wrapper work; fall back to `webpki-roots` if needed |
| DNS rebinding / SSRF | Address guard on resolved addresses, pinned resolver, re-check every redirect, no proxy |
| Players behind proxies or captive portals | `no_proxy` default; revisit with an opt-in setting |
| Frame hitches from big payloads or message floods | Per-frame event budget, zero-copy `ArrayBuffer`, size limits |
| Hot reload with requests in flight | Generation counter; abort on `reset()` |
| `Value` change ripples (conformance, op decoding) | Small, additive variant; conformance tests first |
| Games phoning home | Manifest host list shown to players; off by default |
| Web API look-alikes that behave differently | Document every difference in `docs/differences.md`; declare only implemented members in `globals.ts` |
| QuickJS memory from large bodies | Size limits; consider `Runtime::set_memory_limit` per game |

## Sources

- s&box source: [Facepunch/sbox-public](https://github.com/Facepunch/sbox-public)
  (`engine/Sandbox.Engine/Utility/Web/Http.cs`, `engine/Sandbox.Engine/Utility/WebSocket.cs`,
  `engine/Sandbox.Engine/Systems/Filesystem/FileSystem.cs`,
  `engine/Tests/Sandbox.Test.Engine/Web/Http.cs`), read 2026-10-17
- s&box docs: [File System](https://sbox.game/dev/doc/assets/file-system.md),
  [AuthTokens](https://wiki.facepunch.com/sbox/AuthTokens)
- Garry's Mod: [HTTP](https://wiki.facepunch.com/gmod/Global.HTTP)
- Deno: [Security and permissions](https://docs.deno.com/runtime/fundamentals/security/)
- Bun: [File I/O](https://bun.sh/docs/api/file-io)
- MDN: [Forbidden header name](https://developer.mozilla.org/en-US/docs/Glossary/Forbidden_header_name),
  [WebSocket.close()](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/close)
- WHATWG: [Fetch standard, HTTP-redirect fetch](https://fetch.spec.whatwg.org/#http-redirect-fetch)
- [rustls-platform-verifier README](https://github.com/rustls/rustls-platform-verifier)
- Crate versions: crates.io API, 2026-10-17. reqwest 0.13.5 features, ureq 3.4.2 features,
  tungstenite 0.30.0 features: from their published `Cargo.toml` files. rquickjs 0.14.0 APIs: the
  vendored source in the local cargo registry.
