# UI without React: a Preact-style component and effect layer in Rust

Status: **design for review, no code** (TODO.md M9.b2 "UI without React", a STOP item). Written 2026-10-19.

## 1. How the UI works today

```
 React 19 + react-reconciler (JS, in QuickJS)
   createInstance / appendChild / commitUpdate ...
        │  queueFor(id).push([OPCODE, ...])           host.ts, style.ts, attrs.ts
        ▼
   resetAfterCommit → __iron_commit(ops[, target])    ONE call per React commit
        │  Vec<Value> arrays, decoded again in Rust
        ▼
 iron_ui::UiTree::apply_commit(&Commit)               crates/iron_ui/src/tree.rs
   apply_op per op (CREATE, SET_STYLE, APPEND_CHILD, INSERT_BEFORE, REMOVE_CHILD,
   CREATE_TEXT, SET_TEXT, SET_ATTRS), then one layout sync
   ...  CSS cascade, Taffy layout, Parley text, draw list, hit testing  ...
        ▲
        │  Pointer / Interaction build the event + its path (target, then ancestors)
 __iron_ui_event(type, path, data, strings)           js/react/src/events.ts
   JS does capture/bubble over the `on*` props of each Instance, flushSyncWork,
   returns `defaultPrevented`; Rust uses it to skip default actions
```

What each side owns:

| | Owner | Where |
|---|---|---|
| The retained tree, ids, CSS, layout, text, hit test, focus, scroll, drag and drop | Rust | `iron_ui` |
| Component state, hooks, the diff (which ops to send), handler tables, event objects, capture/bubble | JS | `js/react` + React |
| Contract between them | 8 opcodes (`opcodes.rs` / `opcodes.ts`) and `__iron_ui_event` | `iron_core::Commit` |

Facts that shape the design:

- **Rust already owns the tree.** React is only a producer of ops and a consumer of events. Values are raw
  (numbers, CSS strings); Rust parses and validates (`apply_style`, `Attrs::set`).
- **The UI surface actually used is small.** Across the 27 files in `examples/`, `editor/` and `js/editor-ui` that
  import React: `useState` 83, `useEffect` 47, `useRef` 33, `useMemo` 10, `useSyncExternalStore` 5,
  `useCallback` 2. No context, reducers, transitions, suspense or portals.
- React features we pay for but do not use: the scheduler and lanes, concurrent rendering, synthetic events,
  DOM-specific props, suspense, the dev tools hook (Fast Refresh needs it).
- Per-frame cost: a React commit in QuickJS plus op serialisation (`Value` arrays) plus decode. The bigger cost for the
  AOT plan is that React and `react-reconciler` need a full JS engine, so a shipped game keeps QuickJS only for UI.

## 2. Goal and non-goals

Goal: UI written in the typed TypeScript subset `iron_ts` compiles, with components, state and effects, running with
**no JS engine**, drawing through the existing `UiTree`. Same element set, same CSS, same events.

Non-goals: React compatibility (no `react` import), a DOM, concurrent rendering, suspense, portals (dialogs already
have the top layer in Rust), server rendering.

## 3. Avoiding duplication: what is reused, what is replaced

| Piece | Decision |
|---|---|
| `UiTree`, CSS, layout, text, hit test, focus, scrolling, dnd | **Reused unchanged.** |
| The op set | **Kept as the single mutation interface.** Add a typed `Op` enum next to `Commit` and decode `Value` arrays into it once; `apply_commit` takes `&[Op]`. React keeps sending `Value`s (decoded to `Op`), the native layer builds `Op`s directly. One applier, no second tree API. |
| Style and attribute parsing | Reused: the native layer passes the same raw strings and numbers. |
| Event path building, `defaultPrevented` | Reused: the path comes from `Pointer`/`Interaction` already. |
| Capture/bubble over `on*` handlers | **Moves to Rust** for native roots (handler table per node id in the new layer). `events.ts` stays for React roots; both are small and neither is shared code. The app chooses by root kind. |
| Element props type (`IronStyle`, event classes) | One TS declaration set, generated or hand-kept in `@iron/ui` and imported by both React and native roots (`.d.ts` from the manifest where Rust owns the shape). |
| React path (`js/react`) | **Stays as is during migration** (editor, examples, Fast Refresh) and is removed only when nothing imports it. |
| Reconciler / diffing | **New**, replacing React's. No other diff exists in Rust. |

## 4. The model

### 4.1 Choice: signals core, with a hook-shaped surface

Options:

- **A. Preact-style virtual DOM** (components return vnodes; diff old against new): familiar, but every state change
  re-runs the component and allocates a vnode tree, which suits neither a GC we want to keep quiet nor an AOT compiler
  without generics.
- **B. Signals** (`signal`, `computed`, `effect`; components run once and bind signals to props; Solid and Preact
  Signals style): no vdom, updates touch only what changed, cheap in Rust, maps directly to ops
  (`SET_STYLE`/`SET_TEXT` from an effect). Lists need a keyed `For`.
- **C. B underneath with hook names on top** (`useState` = a signal owned by the component, `useEffect` = an effect with
  cleanup tied to the owner, `useMemo` = computed, `useRef` = a cell). Existing UI code ports with little change; the
  call-order rule is kept so a component instance keeps a slot vector.

**Recommendation: C**, with B's rules visible: a component function runs once per mount; reading a signal inside a
binding (`style={() => ...}`, `{() => text}`) subscribes only that binding. This is the cheapest in Rust and keeps the
port of 83 `useState` uses mechanical.

### 4.2 Pieces

- **Reactive core (Rust).** Signals, computed values and effects in a slab with generation ids; an owner tree
  (component scope → child scopes) so unmounting runs cleanups and drops effects; batching so several writes in one
  handler run each effect once, before the frame's layout; a flush point after every handler and once per frame.
- **Component instances (Rust).** A component is a compiled function value `(props) => Node`. The instance holds
  its owner scope, hook slots, and the node ids it created.
- **Node builders.** `box(props, children)`, `text(...)`, `Show`, `For(list, key, render)`, `Switch`. A static prop is
  applied once; a function prop becomes an effect that emits `SET_STYLE`/`SET_ATTRS` ops with the *diff* of that one
  binding. `For` keeps keyed child scopes and emits `INSERT_BEFORE`/`REMOVE_CHILD` (moves are ops already).
- **Events.** `onPointerDown={e => ...}` is stored in a per-node handler table in Rust; dispatch walks the path
  from Rust (capture then bubble, `stopPropagation`, `preventDefault`) and calls the closure with a typed event
  struct. Discrete events flush effects before returning, as `flushSyncWork` does today.
- **Refs and imperative calls.** `useRef` holds a node handle; `focus()`, `scrollTo`, `setPointerCapture` are
  `#[iron_api]` methods on a `UiNode` handle class (the same mechanism as `GameObject`), replacing the `op("focus", id)`
  calls the `Instance` class makes.
- **Text fields.** `value`/`onChange` controlled-field restore moves to Rust, where the editor state lives; today it
  is a JS round trip.

### 4.3 What compiles under `iron_ts`

The layer is Rust plus a small TypeScript surface (`@iron/ui`):

- Closures (effects, handlers, render callbacks) are `Closure{func, env}` values. Rust roots them with
  `ts_rt_root_add` and calls `func(env, args)`; this is the "closures to Rust" mechanism of the boundary note section 6
  and the same `NativeHost` rooting already built for components.
- Components are generic-free functions `(props: P) => Node`; props are interfaces or classes the compiler already
  handles. `useState<T>` needs generics: until they exist the surface is typed per use through the compiler's
  generics-as-intrinsics default (the same treatment as `getComponent`), or `Signal<number | string | boolean>` and
  object signals first.
- JSX: a source rewrite like `generators.rs` (JSX to `h(type, props, children)` calls), because oxc already parses
  JSX/TSX.

## 5. Migration path, smallest first

1. `Op` enum and one applier in `iron_ui` (React path unchanged, tests unchanged). Removes the decode duplication.
2. Reactive core in Rust (`signal`, `computed`, `effect`, owners, batching) with Rust tests only.
3. Builders + `UiTree` binding in Rust, tested with Rust closures (no compiler): a counter, a list with keys, a
   `Show`, handler dispatch, text field.
4. Closure bridge to compiled TS and the TS surface (`@iron/ui`), JSX rewrite; compile and run a counter and a todo
   list with `iron_ts_tests` (bun-compared where behaviour is JS-visible).
5. Port one real piece (the arcade HUD) next to its React version; compare screenshots with the existing goldens.
6. Port `@iron/editor-ui` (menubar, dock, virtual list, tree view, console), then drop `js/react` when nothing imports
   it. Fast Refresh/hot reload for native UI is a separate item under "Dev loop without Bun".

## 6. Risks

- Hook rules (call order) and effect timing differ subtly from React's; port by hand-checking each effect, not by search
  and replace. The 47 `useEffect` uses are the review list.
- Without generics, `useState<T>` and `For<T>` are intrinsics with per-use types; this is the same limit as
  `getComponent` and has the same workaround.
- Exceptions inside handlers: native faults take the process down (boundary note section 13, answer 5); handlers need
  the same fault policy as hooks (`NativeHost` disables a component; for UI, log and drop the handler).
- GC: handlers and effects hold closures; the owner tree must root exactly what is alive and unroot on unmount, or the
  UI leaks or crashes under `IRON_TS_GC=stress`.

## 7. Questions for the user

1. **Signals-first (C) or a literal Preact remake (A)?** Recommendation C. A is closer to "Preact" but costs allocation
   per update and a diff the tree does not need.
2. **Where does the reactive core live?** Recommendation: Rust (`iron_ui` or a new `iron_ui_kit` crate), as asked. The
   alternative, writing it in the TS subset, avoids the closure bridge but puts the engine's hot UI path in compiled
   user-level code.
3. **Keep React for the editor until the end?** Recommendation yes: the editor is the largest user, and both paths
   share the same `UiTree` and ops.
4. **JSX/TSX as the surface?** Recommendation yes (rewrite to builder calls), so ported components keep their shape.
5. **Hook names (`useState`, `useEffect`) or Solid names (`createSignal`, `createEffect`)?** Recommendation: hook names
   at first, for porting; Solid names can be added later because they are the same objects.
