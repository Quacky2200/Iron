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
}

function useState<T>(init: T): State<T> {
  const s = new State<T>();
  s.slot = Ui.stateNew(init);
  s.value = Ui.load<T>(s.slot);
  const slot = s.slot;
  s.set = (v: T): void => {
    Ui.store(slot, v);
  };
  return s;
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
