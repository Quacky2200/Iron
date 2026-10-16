# @iron/react

Package: [js/react](../js/react/src). A custom `react-reconciler` host config that turns React
tree mutations into UI commits for the Rust [UiTree](ui-tree.md), and runs UI events back through
React handlers. React 19 and `react-reconciler` run unmodified inside QuickJS.

| File | Role |
|---|---|
| [index.ts](../js/react/src/index.ts) | Public API: `createRoot`, `Text`, `Span`, prop and JSX types, re-exports |
| [host.ts](../js/react/src/host.ts) | The reconciler host config, `Instance`, `instances`, `pending`, `flush` |
| [style.ts](../js/react/src/style.ts) | `IronStyle` type, `styleEntries`, `styleDiff` |
| [attrs.ts](../js/react/src/attrs.ts) | Attribute props, `attrEntries`, `attrDiff` |
| [events.ts](../js/react/src/events.ts) | Event classes and `dispatchUiEvent` (installed as `__iron_ui_event`) |
| [opcodes.ts](../js/react/src/opcodes.ts) | Opcode constants; must match [opcodes.rs](../crates/iron_ui/src/opcodes.rs) |

## Elements

Host element types are `box`, `text` and `span` (`ELEMENT_TYPES` in host.ts; anything else throws
at `createInstance`). `<box>` is a JSX intrinsic (declared in `index.ts` through module
augmentation). React's own JSX types already define `<text>` (SVG) and `<span>` (HTML), so Iron
exposes **`Text`** and **`Span`** as components that call `createElement("text" | "span", ...)`. CSS
type selectors still use `text` and `span` ([css.md](css.md)). Strings become `#text` nodes
(`createTextInstance`); `shouldSetTextContent` is always `false`, so text is always a real node.

## Reconciler host config ([host.ts](../js/react/src/host.ts))

Mutation mode only (`supportsMutation: true`). The design is: **collect ops during React's commit,
send one `__iron_commit` at the end** (decision D1 in TODO.md).

```
createInstance / appendChild / commitUpdate ...  -->  pending.push([OPCODE, ...])
resetAfterCommit()                               -->  flush(): __iron_commit(pending.splice(0))
```

| Host function | Ops produced |
|---|---|
| `createInstance(type, props)` | `CREATE id type`, then `SET_STYLE` (if style) and `SET_ATTRS` (if attrs) |
| `createTextInstance` / `commitTextUpdate` | `CREATE_TEXT` / `SET_TEXT` |
| `appendInitialChild`, `appendChild`, `appendChildToContainer` | `APPEND_CHILD` (container uses `ROOT_ID` 0) |
| `insertBefore`, `insertInContainerBefore` | `INSERT_BEFORE` (also how moves are expressed) |
| `removeChild`, `removeChildFromContainer`, `clearContainer` | `REMOVE_CHILD` (Rust destroys the subtree) |
| `commitUpdate(instance, _, prev, next)` | `SET_STYLE` of the diff, `SET_ATTRS` of the diff |
| `hideInstance` / `unhideInstance` (Suspense) | `SET_STYLE display none` / restore the prop's display |

Details that are easy to miss:

- **Ids** come from a module counter `nextId` starting at 1 and are never reused in a session
  (test M2-REC-05); 0 is the root container. The `Instance` class keeps `id`, `type` and the latest
  `props`. `instances: Map<id, Instance>` is how engine events find handlers (`detachDeletedInstance`
  removes entries).
- Container children are tracked in a `Set` so `clearContainer` can emit removals.
- `resolveUpdatePriority` returns the priority set by `withUpdatePriority` (used by event
  dispatch), else `DefaultEventPriority`. Scheduling uses `queueMicrotask` / `setTimeout` from the
  script runtime, so React work is driven by `ScriptRuntime::tick` and microtask draining
  ([scripting.md](scripting.md)).
- Everything form/transition/suspending-commit related is stubbed (`maySuspendCommit` is false).

## Style and attribute diffing

- `styleEntries(style)` flattens to `[prop, value, ...]`, skipping `undefined`.
- `styleDiff(prev, next)` emits props whose values changed (shallow `!==`) and `null` for removed
  props; Rust treats `null` as "reset to default". Style objects are not frozen or hashed, so
  creating a new object with identical values costs a loop but sends nothing.
- `attrEntries` / `attrDiff` do the same for attribute props. A prop is an attribute if it is
  `className`, `id`, `disabled`, `checked`, `tabIndex` or starts with `data-` (`isAttr`). Everything
  else (including `on*` handlers) stays in `props` and never crosses to Rust.

Note the ops carry **raw values** (numbers or CSS strings); Rust does all parsing and validation
(`apply_style`, `Attrs::set`) and reports problems as `UiError`s logged by the app.

## `createRoot`

`createRoot()` ([index.ts](../js/react/src/index.ts)) builds a `ConcurrentRoot` container whose id is
the engine's root node 0. It may be called **once per game** (a second call throws): there is a
single engine root, so compose more UI as components. `root.render(el)` uses
`updateContainerSync` + `flushSyncWork` so the commit lands the same frame; `unmount` renders
`null`. Uncaught/caught/recoverable React errors go to `console.error` with the stack.

## Events ([events.ts](../js/react/src/events.ts))

Importing `@iron/react` runs `globalThis.__iron_ui_event = dispatchUiEvent`; the app detects that
global to decide whether to dispatch events at all. Flow and data layout are in
[input-and-events.md](input-and-events.md); on the JS side:

- `HANDLERS` maps an engine event type to prop names. Pointer events also trigger the mouse alias
  (`pointermove` runs `onPointerMove` then `onMouseMove`); `focusin`/`focusout` map to
  `onFocus`/`onBlur` (which bubble, like React DOM); `scroll` -> `onScroll` (`IronScrollEvent`: `scrollLeft/Top/Width/Height`, `clientWidth/Height`, read from `data[10..16]`; does not bubble, continuous priority).
- `dispatchUiEvent(type, path, data, keys)`: `path[0]` is the target and the rest are its ancestors.
  It builds the event object (`IronPointerEvent`, `IronWheelEvent`, `IronKeyboardEvent`,
  `IronFocusEvent`, or `IronScrollEvent`), then runs the **capture phase** from the
  outermost ancestor to the target (`on*Capture`), then the **bubble phase** from the target up.
  `pointerenter`/`pointerleave` (and `scroll`) do not bubble. `stopPropagation()` lets the current
  element's remaining handlers run, as in React.
- Handler exceptions are logged and dispatch continues (like DOM listeners).
- The return value is `defaultPrevented`; Rust uses it to skip default actions (focus on press,
  Tab, wheel scrolling).
- **Priorities**: events in `CONTINUOUS` (move, enter, leave, wheel, scroll) run with
  `ContinuousEventPriority`, so their updates batch into the next tick; others use
  `DiscreteEventPriority` and call `reconciler.flushSyncWork()` before returning, so the commit
  (and its `__iron_commit`) happens inside the same Rust call.
- Event classes mimic the DOM: `clientX/Y`, `button`, `buttons`, `detail`, modifier flags,
  `key`/`code`/`repeat`, `relatedTarget` (resolved through `instances`). Modifier bits: shift 1,
  ctrl 2, alt 4, meta 8.

## Refs, `IronElement`, `focus()` and `blur()`

`ref` on `<box>`, `<Text>` and `<Span>` yields the host `Instance` (`getPublicInstance`), exported as
`IronElement`. Its methods queue ops rather than touching Rust directly:

| Method | Op | Handled by |
|---|---|---|
| `focus()` | `op("focus", id)` | `App::tick` -> `Input::Focus(id)`; ignored unless the node is focusable |
| `blur()` | `op("blur", id)` | `Input::Blur(id)`: only if that node has focus |
| `scrollTo(left, top)` / `scrollBy(dx, dy)` / `scrollIntoView()` | `op("scrollTo", ...)` etc. | `Input::ScrollTo/ScrollBy/ScrollIntoView` |

Ops are drained once per frame, so focus changes apply on the next frame, not synchronously.
`tabIndex` is just an attribute prop (see below).

## Props summary

`BoxProps = ElementAttrs & EventProps & { style, children, key, ref }`:

- `ElementAttrs`: `className`, `id`, `disabled`, `checked`, `tabIndex`, `data-*`.
- `EventProps`: pointer/mouse (`onClick`, `onDoubleClick`, `onContextMenu`, `onPointerDown/Up/Move/Enter/Leave`,
  `onMouse*`, `onWheel`), keyboard (`onKeyDown/Up`), focus (`onFocus`, `onBlur`), each with a
  `*Capture` variant except enter/leave.
- `IronStyle` ([style.ts](../js/react/src/style.ts)) is a type only. The authoritative list of
  properties is `apply_style` in Rust; add a property in both places.
