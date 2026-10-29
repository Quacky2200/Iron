// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The React names a game uses without importing them, and the module they are re-exported from
//! (`react`, `@platypus/react`). The signatures are TypeScript text (the hooks are generic over the
//! program's own types); `tools/platypus_ts` prints them and lowers the calls (`lower/jsx.rs`).

/// The module a name is also exported from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Module {
    React,
    PlatypusReact,
}

/// A global function; the same name twice is an overload.
#[derive(Clone, Copy, Debug)]
pub struct Func {
    pub name: &'static str,
    pub module: Module,
    /// Everything after the name: `<T>(init: T): Ref<T>`.
    pub sig: &'static str,
}

const fn r(name: &'static str, sig: &'static str) -> Func {
    Func {
        name,
        module: Module::React,
        sig,
    }
}

/// The hooks and higher-order functions (`react`).
pub const HOOKS: &[Func] = &[
    r(
        "useState",
        "<T>(init: T | (() => T)): [T, (v: T | ((prev: T) => T)) => void]",
    ),
    r(
        "useEffect",
        "(f: () => void | (() => void), deps?: readonly unknown[]): void",
    ),
    r(
        "useLayoutEffect",
        "(f: () => void | (() => void), deps?: readonly unknown[]): void",
    ),
    r("useRef", "<T>(init: T): RefObject<T>"),
    r("useRef", "<T>(init: null): RefObject<T | null>"),
    r(
        "useImperativeHandle",
        "<T>(ref: Ref<T> | undefined, create: () => T, deps?: readonly unknown[]): void",
    ),
    r("useMemo", "<T>(f: () => T, deps: readonly unknown[]): T"),
    r(
        "useCallback",
        "<F extends (...args: any[]) => any>(f: F, deps: readonly unknown[]): F",
    ),
    r(
        "useSyncExternalStore",
        "<T>(subscribe: (onStoreChange: () => void) => () => void, getSnapshot: () => T): T",
    ),
    r(
        "useReducer",
        "<S, A>(reducer: (s: S, a: A) => S, init: S): [S, (a: A) => void]",
    ),
    r(
        "createContext",
        "<T>(def: T): { Provider: (props: { value: T; children?: ReactNode }) => ReactElementLike }",
    ),
    r(
        "useContext",
        "<T>(c: { Provider: unknown } & { __t?: T }): T",
    ),
    r(
        "memo",
        "<P>(c: (props: P) => ReactElementLike | null): (props: P) => ReactElementLike | null",
    ),
    r(
        "forwardRef",
        "<P, T>(f: (props: P, ref: Ref<T>) => ReactElementLike | null): (props: P & { ref?: Ref<T> }) => ReactElementLike | null",
    ),
    // `<Fragment key={k}>`: a fragment that takes a key (`lower/jsx.rs`).
    r(
        "Fragment",
        "(props: { children?: ReactNode }): ReactElementLike",
    ),
];

/// `createRoot` (`@platypus/react`): the target is a world panel, a window or its handle; `render` takes
/// the tree.
pub const CREATE_ROOT: Func = Func {
    name: "createRoot",
    module: Module::PlatypusReact,
    sig: "(target?: WorldPanel | Window | number): { render(node: ReactNode): void; unmount(): void }",
};

/// The type names of `react` (everything else of `react` is a function of [`HOOKS`]).
pub const REACT_TYPES: &[&str] = &["ReactNode", "Ref", "RefObject"];

/// The type names of `@platypus/react` besides the event interfaces, the element components and
/// [`CREATE_ROOT`].
pub const PLATYPUS_REACT_TYPES: &[&str] = &["PlatypusStyle", "PlatypusElement"];
