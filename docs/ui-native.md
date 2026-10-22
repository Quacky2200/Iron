# iron_ui_native: the native reconciler

Crate: [iron_ui_native](../crates/iron_ui_native/src/lib.rs). Design and migration plan:
[research/iron-ui-native.md](research/iron-ui-native.md). Tests: `M2-NAT-01`..`12`
([reconcile.rs](../crates/iron_ui_native/tests/reconcile.rs), [native_oracle.test.tsx](../js/react/test/native_oracle.test.tsx)).

A component is a function from hooks to an `Element` description (plain data). A `Runtime` keeps an instance tree
parallel to the descriptions, diffs each new description against it and applies typed ops
(`iron_ui::opcodes::Op`, [ui-tree.md](ui-tree.md#typed-ops)) to a `UiTree` as one batch per update. React stays as it is
(`@iron/react`); both draw into the same `UiTree`, so give the runtime a node-id range the script side does not use
(`Runtime::new(first_node)`).

| Piece | What it does |
|---|---|
| `Element` | `None`, `Text`, `Host { ty, key, props, children }`, `Fragment`, `Component { id, key, render }`, `Portal`, `Provider`. `HostProps` carries raw style and attribute pairs (the values `iron_ui` parses), event handlers (`"click"`, `"click:capture"`) and a `NodeRef`. |
| Hooks (`Ctx`) | `use_state` (with `Setter::set`/`update`, `Object.is` bail-out), `use_ref`, `use_memo`, `use_callback`, `use_effect` (deps `None` = every render, `Some(&[])` = once), `use_context`. State lives in slots in call order; a changed hook order panics. |
| Diff | Same kind/type/component id and key: update in place. Children: match by key, else by position; what is not matched unmounts first, then the rest update, new ones mount. Host props: per-property style and attribute diff (`null` resets). Placement is computed per container from the desired node order (remove, then insert/move right to left), so fragments and components that change what they return are placed correctly. |
| Effects | Same order as React 19 (checked against real React): mount effects run children before parents; on update the changed effects' cleanups run children before parents, then the new effects; deleted subtrees clean up parent before child, before the new effects. Effects run after the ops are applied. State set by an effect loops until nothing is dirty (100 passes at most, then an error). |
| Events | `Runtime::dispatch(&mut Event, &mut UiTree)`: capture outside-in, bubble inside-out along the **component tree** (a portal's content bubbles to its logical parent), `stop_propagation` and `prevent_default`, then a synchronous flush. |
| Text fields | A host `input` / `textarea` with a `value` attribute is controlled: the typed text (`Event.strings[0]` of an `"input"` event) is what the field holds until the handlers ran; if the new `value` prop is not that text (the edit was rejected or changed) the prop is sent back with one `SetAttrs`. |
| Hot reload | `Runtime::refresh(root, element, remount_ids, tree)` renders the new code's element over the old tree: every component renders again with its new render function (`memo` ignored) and keeps its state, except the ids in `remount_ids`, which start over. A compiled program exports `__uiSignatures()` (`Name:ID:hook,hook;...`); the host remounts the components whose hook list differs between the old and the new program. |
| Portals / roots | `create_root(container)` mounts into any node; `Element::portal(target, children)` mounts under another node. One `Runtime` per `UiTree`. |

The compiler side (JSX, hooks, closures to Rust, stable component ids) is described in [iron-ts.md](iron-ts.md#ui-jsx-and-hooks-native-reconciler).

Not built yet (in the design note's order): hook signatures for hot reload, `ref=`, context from TSX, `memo`, `forwardRef`, `useReducer`, `useImperativeHandle`, `useSyncExternalStore`, controlled text
fields, hot-reload state, and wiring into `iron_app` (another session's area).

## Tables of what a script can say

`crates/iron_ui_tables` lists the style properties (with their TypeScript types and docs), attributes, elements, event fields and `on*` props. Compiled games read them in three places: `tools/iron_ts/src/ui_dts.rs` prints `IronStyle`, `IronProps`, the event interfaces and `JSX` into the generated `types/iron.d.ts`; the same file writes the compiled prelude (`StyleEntry`, and the `UiEvent` class whose fields read `Ui.evNum(slot)` / `Ui.evStr(slot)`); `lower/jsx.rs` turns an `on*` prop into the event type the table names (`onChange` listens to `input`, `onFocus` to `focusin`). `crates/iron_ui/tests/ui_tables.rs` keeps the tables honest against `apply_style`, `Attrs::set`, `UiEvent::script_args` and `EventKind`.
