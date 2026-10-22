# @iron/react

Package: [js/react](../js/react/src). A custom `react-reconciler` host config that turns React
tree mutations into UI commits for the Rust [UiTree](ui-tree.md), and runs UI events back through
React handlers. React 19 and `react-reconciler` run unmodified inside QuickJS.

| File | Role |
|---|---|
| [index.ts](../js/react/src/index.ts) | Public API: `createRoot`, `Text`, `Span`, prop and JSX types, re-exports |
| [host.ts](../js/react/src/host.ts) | The reconciler host config, `Instance`, `instances`, `Container`s with their op queues, `flush` |
| [style.ts](../js/react/src/style.ts) | `IronStyle` type, `styleEntries`, `styleDiff` |
| [attrs.ts](../js/react/src/attrs.ts) | Attribute props, `attrEntries`, `attrDiff` |
| [events.ts](../js/react/src/events.ts) | Event classes and `dispatchUiEvent` (installed as `__iron_ui_event`) |
| [opcodes.ts](../js/react/src/opcodes.ts) | Opcode constants; must match [opcodes.rs](../crates/iron_ui/src/opcodes.rs) |

## Elements

Host element types are `box`, `text`, `span`, `input`, `textarea`, `image`, `list` and `dialog` (`ELEMENT_TYPES` in host.ts;
anything else throws at `createInstance`). `Input` and `TextArea` are the components for the last two
([Text fields](#text-fields)). `List` creates a `list` (`<list>` is also a JSX intrinsic): its element children are list items ([ui-tree.md](ui-tree.md#lists-m3d-step-5g)). `Dialog` creates a `dialog` and takes `open`, `modal`, `closedBy`, `onCancel` and `onClose` ([ui-tree.md](ui-tree.md#dialogs-and-the-top-layer-m3d-step-5j)). `Image` (props: `src`, plus the usual `style`/`className`) creates an `image` element ([ui-tree.md](ui-tree.md#images)). `<box>` is a JSX intrinsic (declared in `index.ts` through module
augmentation). React's own JSX types already define `<text>` (SVG) and `<span>` (HTML), so Iron
exposes **`Text`** and **`Span`** as components that call `createElement("text" | "span", ...)`. CSS
type selectors still use `text` and `span` ([css.md](css.md)). Strings become `#text` nodes
(`createTextInstance`); `shouldSetTextContent` is always `false`, so text is always a real node.

## Reconciler host config ([host.ts](../js/react/src/host.ts))

Mutation mode only (`supportsMutation: true`). The design is: **collect ops during React's commit,
send one `__iron_commit` at the end** (decision D1 in TODO.md).

```
createInstance / appendChild / commitUpdate ...  -->  queueFor(id).push([OPCODE, ...])
resetAfterCommit(container)                      -->  flushContainer(container): __iron_commit(ops[, target])
```

Each root has a `Container { id, target, children, pending }`. `owners` maps every node id to the
container that created it, and `queueFor(id)` picks that container's `pending` queue, so ops for a
world panel's nodes never mix with the screen's. `flushContainer` sends the screen's ops as
`__iron_commit(ops)` and others as `__iron_commit(ops, target)`; `flush()` (outside a commit, for
controlled-field restores) flushes every container. `pending` still exports the screen's queue for tests.

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
  `className`, `id`, `disabled`, `checked`, `draggable`, `tabIndex`, `dir`, a text field prop (`value`, `defaultValue`,
  `placeholder`, `readOnly`, `rows`) or starts with `data-` (`isAttr`). Everything
  else (including `on*` handlers) stays in `props` and never crosses to Rust.

Note the ops carry **raw values** (numbers or CSS strings); Rust does all parsing and validation
(`apply_style`, `Attrs::set`) and reports problems as `UiError`s logged by the app.

## `createRoot`

`createRoot(target?)` ([index.ts](../js/react/src/index.ts)) builds a `ConcurrentRoot`. With no argument
it is the **screen's** root: the container is `screenContainer` (target 0, the engine's root node 0).
Pass a `WorldPanel` component, or its id, to get that panel's root (`containerFor(id)`), whose commits
go to the panel's own UI tree ([world-rendering.md](world-rendering.md#world-panels)). Each target
may have **one root** (`rootsCreated`): a second `createRoot()` throws `createRoot() can only be
called once per game`, and a second root for a panel throws `panel N already has a root`.
Unmounting frees the target for a new root. `root.render(el)` uses
`updateContainerSync` + `flushSyncWork` so the commit lands the same frame; `unmount` renders
`null`. Uncaught/caught/recoverable React errors go to `console.error` with the stack.

**Fast Refresh (dev builds).** In a dev build ([scripting.md](scripting.md#from-typescript-to-a-running-game))
React lives in `vendor.js` and the game's code runs again on each edit:

- [refresh.ts](../js/react/src/refresh.ts) loads first in the vendor bundle. It calls
  `injectIntoGlobalHook(globalThis)` before React and the reconciler load, and defines the
  `$RefreshReg$` / `$RefreshSig$` globals that Bun's `--react-fast-refresh` transform calls in the
  game's code. Typings for the runtime are in [react-refresh.d.ts](../js/react/src/react-refresh.d.ts).
- `host.ts` ends with `reconciler.injectIntoDevTools()`, so the hook sees the renderer and can find
  its roots. Without the hook (production builds) this does nothing.
- While the game's code is loading again (`_loadingGame()` from `@iron/engine`), `createRoot(target)`
  returns the live root of that target (kept in a `roots` map) instead of throwing the
  "only once" error, and its `render` returns without touching the tree once it has rendered.
  The tree stays, so state stays.
- After the new code ran, `performReactRefresh` re-renders the components whose code changed. State
  is kept where the hook signatures still match; React remounts a component whose hooks changed.

## Events ([events.ts](../js/react/src/events.ts))

Importing `@iron/react` runs `globalThis.__iron_ui_event = dispatchUiEvent`; the app detects that
global to decide whether to dispatch events at all. Flow and data layout are in
[input-and-events.md](input-and-events.md); on the JS side:

- `HANDLERS` maps an engine event type to prop names. Pointer events also trigger the mouse alias
  (`pointermove` runs `onPointerMove` then `onMouseMove`); `focusin`/`focusout` map to
  `onFocus`/`onBlur` (which bubble, like React DOM); `input` -> `onChange` then `onInput` (`IronInputEvent`, bubbles); `scroll` -> `onScroll` (`IronScrollEvent`: `scrollLeft/Top/Width/Height`, `clientWidth/Height`, read from `data[10..16]`; does not bubble, continuous priority); `cancel` -> `onCancel`, `close` -> `onClose` (dialogs; `cancel` can be prevented, which stops `close`); `dragstart`/`drag`/`dragenter`/`dragover`/`dragleave`/`drop`/`dragend` -> `onDragStart` ... `onDragEnd` (`IronDragEvent` with `dataTransfer: IronDataTransfer` and `dropped`; see [input-and-events.md](input-and-events.md#drag-and-drop-m5e)); `transitionrun/start/end/cancel` -> `onTransitionRun/Start/End/Cancel` (`IronTransitionEvent`: `propertyName`, `elapsedTime` in seconds) and `animationstart/iteration/end/cancel` -> `onAnimationStart/Iteration/End/Cancel` (`IronAnimationEvent`: `animationName`, `elapsedTime`); all bubble. See [animation.md](animation.md).
- `dispatchUiEvent(type, path, data, strings)`: `path[0]` is the target and the rest are its ancestors.
  It builds the event object (`IronPointerEvent`, `IronWheelEvent`, `IronKeyboardEvent`,
  `IronFocusEvent`, `IronInputEvent`, `IronScrollEvent`, `IronTransitionEvent` or `IronAnimationEvent`), then runs the **capture phase** from the
  outermost ancestor to the target (`on*Capture`), then the **bubble phase** from the target up.
  `pointerenter`/`pointerleave`, `scroll`, `cancel` and `close` do not bubble (`NON_BUBBLING`). `stopPropagation()` lets the current
  element's remaining handlers run, as in React.
- Handler exceptions are logged and dispatch continues (like DOM listeners).
- The return value is `defaultPrevented`; Rust uses it to skip default actions (focus on press,
  Tab, wheel scrolling).
- **Priorities**: events in `CONTINUOUS` (move, enter, leave, wheel, scroll) run with
  `ContinuousEventPriority`, so their updates batch into the next tick; others use
  `DiscreteEventPriority` and call `reconciler.flushSyncWork()` before returning, so the commit
  (and its `__iron_commit`) happens inside the same Rust call.
- Event classes mimic the DOM: `clientX/Y`, `button`, `buttons`, `detail`, modifier flags,
  `key`/`code`/`repeat` (from `strings[0..2]`), `relatedTarget` (resolved through `instances`). Modifier bits: shift 1,
  ctrl 2, alt 4, meta 8.

## Text fields

`Input` (one line) and `TextArea` (wrapping, `rows` sets its default height) in
[index.ts](../js/react/src/index.ts) take `FieldProps`: `value`, `defaultValue`, `placeholder`,
`readOnly`, `onChange`/`onInput`, plus the usual attributes, `style` and events. Fields are focusable
without `tabIndex`. Engine side: [ui-tree.md](ui-tree.md#text-fields) and
[input-and-events.md](input-and-events.md#text-fields-m3c-step-5).

- `Instance.value` mirrors the field's text (`e.target.value`, like the DOM): set in the constructor
  from `value ?? defaultValue ?? ""`, updated in `commitUpdate` when the `value` prop is set, and
  updated by `dispatchUiEvent` **before** handlers run.
- `input` events carry `strings = [value]`; `IronInputEvent.value` is that text. Handlers run the
  normal capture/bubble path with discrete priority, so a `setState` commits before the call returns.
- **Controlled fields**: after `flushSyncWork`, `restoreControlledValue(target)` compares the
  instance's value with the `value` prop. If the handler did not accept the edit (no `setState`, or
  it stored something else) the prop wins: `Instance.value` is reset and `[SET_ATTRS, id, "value",
  v]` is pushed and flushed immediately, so Rust's text goes back. Accepting it (even transformed,
  e.g. upper-cased) commits through the normal `commitUpdate` diff. `value={undefined}` or `null`
  means uncontrolled and nothing is restored.
- Uncontrolled fields keep what was typed and send no ops on input.

## Refs, `IronElement`, `focus()` and `blur()`

`ref` on `<box>`, `<Text>` and `<Span>` yields the host `Instance` (`getPublicInstance`), exported as
`IronElement`. Its methods queue ops rather than touching Rust directly:

| Method | Op | Handled by |
|---|---|---|
| `focus()` | `op("focus", id)` | `App::tick` -> `Input::Focus(id)`; ignored unless the node is focusable |
| `blur()` | `op("blur", id)` | `Input::Blur(id)`: only if that node has focus |
| `setPointerCapture()` / `releasePointerCapture()` | `op("setPointerCapture", id)` etc. | `Input::SetPointerCapture/ReleasePointerCapture`; only effective while a button is held, so call it from `onPointerDown` |
| `hasPointerCapture()` | none (local) | True when `pointerCapture.element` (host.ts) is this element; `dispatchUiEvent` updates it on `gotpointercapture` / `lostpointercapture` |
| `scrollTo(left, top)` / `scrollBy(dx, dy)` / `scrollIntoView(arg)` | `op("scrollTo", ...)` etc.; `scrollIntoView` sends `block` and `inline` (`"start"`, `"center"`, `"end"`, `"nearest"`; the DOM's defaults `start` / `nearest`, `false` is `end`) | `Input::ScrollTo/ScrollBy/ScrollIntoView { block, inline }` (`ScrollAlign`) |

Ops are drained once per frame, so focus changes apply on the next frame, not synchronously.
`tabIndex` is just an attribute prop (see below).

## Props summary

`BoxProps = ElementAttrs & EventProps & { style, children, key, ref }`:

- `ElementAttrs`: `className`, `id`, `disabled`, `checked`, `draggable`, `tabIndex`, `dir` (`'ltr' | 'rtl'`), `data-*`. Text fields add
  `FieldProps` (above).
- `EventProps`: pointer/mouse (`onClick`, `onDoubleClick`, `onContextMenu`, `onPointerDown/Up/Move/Enter/Leave`,
  `onMouse*`, `onWheel`), keyboard (`onKeyDown/Up`), focus (`onFocus`, `onBlur`), pointer capture (`onGotPointerCapture`, `onLostPointerCapture`; no `*Capture` variants), drag and drop (`onDragStart` ... `onDragEnd`), `onChange`/`onInput` on fields, each with a
  `*Capture` variant except enter/leave.
- `IronStyle` ([style.ts](../js/react/src/style.ts)) is a type only. The authoritative list of
  properties is `apply_style` in Rust; add a property in both places.
