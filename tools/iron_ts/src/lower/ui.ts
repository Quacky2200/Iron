class UiEvent {
  type: string = Ui.evKind();
  target: number = Ui.evTarget();
  clientX: number = Ui.evNum(0);
  clientY: number = Ui.evNum(1);
  key: string = Ui.evStr(0);
  value: string = Ui.evStr(0);
  stopPropagation(): void {
    Ui.evStop();
  }
  preventDefault(): void {
    Ui.evPrevent();
  }
}

function __handler(f: (e: UiEvent) => void): () => void {
  return (): void => {
    f(new UiEvent());
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

class Ref<T> {
  current: T;
}

function useRef<T>(init: T): Ref<T> {
  const r = new Ref<T>();
  r.current = init;
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
