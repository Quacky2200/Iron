// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// A handler may be missing (`onClick={cond ? undefined : f}`) and may take no parameter.
function __handler(f: ((e: UiEvent) => void) | undefined): () => void {
  return (): void => {
    if (f) {
      f(new UiEvent());
    }
  };
}

class State<T> {
  slot: number = 0;
  value: T;
  set: (v: T) => void;
  update: (f: (prev: T) => T) => void;
}

function useState<T>(init: T): State<T> {
  const s = new State<T>();
  s.slot = Ui.stateNew(init);
  s.value = Ui.load<T>(s.slot);
  const slot = s.slot;
  s.set = (v: T): void => {
    Ui.store(slot, v);
  };
  s.update = (f: (prev: T) => T): void => {
    Ui.store(slot, f(Ui.load<T>(slot)));
  };
  return s;
}

// `useState(() => init)`: the initial value is computed on the first render only.
function useStateLazy<T>(init: () => T): State<T> {
  const s = new State<T>();
  s.slot = Ui.stateSlot();
  if (Ui.stateEmpty(s.slot)) {
    Ui.store(s.slot, init());
  }
  s.value = Ui.load<T>(s.slot);
  const slot = s.slot;
  s.set = (v: T): void => {
    Ui.store(slot, v);
  };
  s.update = (f: (prev: T) => T): void => {
    Ui.store(slot, f(Ui.load<T>(slot)));
  };
  return s;
}

class Reducer<S, A> {
  state: S;
  dispatch: (a: A) => void;
}

function useReducer<S, A>(reducer: (s: S, a: A) => S, init: S): Reducer<S, A> {
  const slot = Ui.stateNew(init);
  const r = new Reducer<S, A>();
  r.state = Ui.load<S>(slot);
  r.dispatch = (a: A): void => {
    Ui.store(slot, reducer(Ui.load<S>(slot), a));
  };
  return r;
}

// A plain object type, not a class: a `{ current: T }` the program writes (`ref: { current:
// PlatypusElement | null }` in its own interface) is then the same type, so the ref object itself goes
// there, not a copy that the element would never be written into.
type Ref<T> = { current: T };

function useRef<T>(init: T): Ref<T> {
  const r: Ref<T> = { current: init };
  const slot = Ui.stateNew(r);
  return Ui.load<Ref<T>>(slot);
}

function useMemo<T>(f: () => T): T {
  const slot = Ui.memoSlot();
  if (Ui.stale(slot)) {
    Ui.store(slot, f());
  }
  return Ui.load<T>(slot);
}

function useCallback<F>(f: F): F {
  return useMemo(() => f);
}

class Context<T> {
  id: number = 0;
  def: T;
}

function createContext<T>(def: T): Context<T> {
  const c = new Context<T>();
  c.id = Ui.contextId();
  c.def = def;
  return c;
}

function useContext<T>(c: Context<T>): T {
  const slot = Ui.contextSlot(c.id);
  if (slot === 0) {
    return c.def;
  }
  return Ui.load<T>(slot);
}

function useImperativeHandle<T>(ref: Ref<T | null>, create: () => T): void {
  Ui.effect((): void => {
    Ui.runEffect((): (() => void) => {
      ref.current = create();
      return (): void => {
        ref.current = null;
      };
    });
  });
}

function useSyncExternalStore<T>(subscribe: (cb: () => void) => () => void, getSnapshot: () => T): T {
  const st = useState(0);
  const counter = useRef(0);
  const set = st.set;
  const value = getSnapshot();
  Ui.depsBegin();
  Ui.dep(subscribe);
  Ui.effect((): void => {
    Ui.runEffect((): (() => void) =>
      subscribe((): void => {
        counter.current += 1;
        set(counter.current);
      }),
    );
  });
  return value;
}
