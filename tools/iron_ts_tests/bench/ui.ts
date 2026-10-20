// Compiled UI components for the native reconciler (`iron_ui_native` through `iron_ts_ui`): JSX,
// useState, useRef, useMemo, useEffect and keyed lists, written as ordinary TSX.

interface CounterProps {
  start: number;
  label: string;
}

function Counter(props: CounterProps): UiElement {
  const [n, setN] = useState(props.start);
  return (
    <box style={{ width: 100 }} onClick={() => setN(n + 1)}>
      <text>{props.label}</text>
      {n}
    </box>
  );
}

export function counterApp(): UiElement {
  return <Counter start={3} label="n=" />;
}

function List(props: { items: string[] }): UiElement {
  return <box>{props.items.map((s) => <text key={s}>{s}</text>)}</box>;
}

export function listApp(n: number, reverse: boolean): UiElement {
  const items: string[] = [];
  for (let i = 0; i < n; i++) items.push("k" + i);
  if (reverse) items.reverse();
  return <List items={items} />;
}

let log = 0;

function Fx(props: { n: number }): UiElement {
  useEffect(() => {
    log = log * 100 + 10 + props.n;
    return () => {
      log = log * 100 + 20 + props.n;
    };
  }, [props.n]);
  return <box />;
}

export function fxApp(n: number): UiElement {
  return <Fx n={n} />;
}

export function getLog(): number {
  return log;
}

let memoRuns = 0;

function Memo(props: { n: number; other: number }): UiElement {
  const sq = useMemo(() => {
    memoRuns += 1;
    return props.n * props.n;
  }, [props.n]);
  return (
    <box>
      {sq}
      {props.other}
    </box>
  );
}

export function memoApp(n: number, other: number): UiElement {
  return <Memo n={n} other={other} />;
}

export function memoCount(): number {
  return memoRuns;
}

function Toggle(): UiElement {
  const [on, setOn] = useState(false);
  const [text, setText] = useState("off");
  const ref = useRef(0);
  ref.current += 1;
  return (
    <box
      onClick={() => {
        setOn(!on);
        setText(on ? "off" : "on");
      }}
    >
      {text}
      {ref.current}
    </box>
  );
}

export function toggleApp(): UiElement {
  return <Toggle />;
}

const Theme = createContext("light");

function Reader(): UiElement {
  const t = useContext(Theme);
  return <text>{t}</text>;
}

export function ctxApp(dark: boolean): UiElement {
  const value = dark ? "dark" : "light";
  return (
    <Theme.Provider value={value}>
      <Reader />
    </Theme.Provider>
  );
}

export function ctxNone(): UiElement {
  return <Reader />;
}

let seenNode = -2;

function WithRef(): UiElement {
  const r = useRef<UiNode | null>(null);
  useEffect(() => {
    seenNode = Ui.nodeId(r.current);
  }, []);
  return <box ref={r} />;
}

export function refApp(): UiElement {
  return <WithRef />;
}

export function getSeenNode(): number {
  return seenNode;
}

interface Api {
  answer: number;
}

let handleSeen = -1;

function Child(props: { handle: Ref<Api | null> }): UiElement {
  useImperativeHandle(props.handle, (): Api => ({ answer: 42 }), []);
  return <box />;
}

function Parent(): UiElement {
  const h = useRef<Api | null>(null);
  useEffect(() => {
    const api = h.current;
    handleSeen = api === null ? -1 : api.answer;
  }, []);
  return <Child handle={h} />;
}

export function handleApp(): UiElement {
  return <Parent />;
}

export function getHandleSeen(): number {
  return handleSeen;
}

function Plus(): UiElement {
  const [n, setN] = useState(0);
  return (
    <box
      onClick={() => {
        setN((c) => c + 1);
        setN((c) => c + 10);
      }}
    >
      {n}
    </box>
  );
}

export function plusApp(): UiElement {
  return <Plus />;
}

interface BoxStyle {
  width: number;
  color?: string;
  height?: number;
}

function Styled(props: { st: BoxStyle }): UiElement {
  return <box style={props.st} />;
}

export function styledApp(w: number, withColor: boolean): UiElement {
  const st: BoxStyle = withColor ? { width: w, color: "red" } : { width: w };
  return <Styled st={st} />;
}

interface Common {
  className: string;
  id: string;
  tabIndex?: number;
}

function Spread(props: { c: Common }): UiElement {
  return <box {...props.c} />;
}

export function spreadApp(): UiElement {
  const c: Common = { className: "a b", id: "main", tabIndex: 3 };
  return <Spread c={c} />;
}

function Inner(props: { a: number; b: number }): UiElement {
  return (
    <box>
      {props.a}
      {props.b}
    </box>
  );
}

function Outer(props: { a: number; b: number }): UiElement {
  return <Inner {...props} b={9} />;
}

export function outerApp(): UiElement {
  return <Outer a={1} b={2} />;
}

let leafCount = 0;

function Leaf(props: { n: number }): UiElement {
  leafCount += 1;
  return <box>{props.n}</box>;
}

const FastLeaf = memo(Leaf);

function MemoParent(props: { n: number; tick: number }): UiElement {
  return (
    <box>
      {props.tick}
      <FastLeaf n={props.n} />
    </box>
  );
}

export function memoParentApp(n: number, tick: number): UiElement {
  return <MemoParent n={n} tick={tick} />;
}

export function leafRenders(): number {
  return leafCount;
}

interface BoxApi {
  size: number;
}

const Fwd = forwardRef((props: { label: string }, ref: Ref<BoxApi | null>): UiElement => {
  useImperativeHandle(ref, (): BoxApi => ({ size: props.label.length }), []);
  return <text>{props.label}</text>;
});

let fwdSeen = -1;

function FwdParent(): UiElement {
  const r = useRef<BoxApi | null>(null);
  useEffect(() => {
    const a = r.current;
    fwdSeen = a === null ? -1 : a.size;
  }, []);
  return <Fwd label="hello" ref={r} />;
}

export function fwdApp(): UiElement {
  return <FwdParent />;
}

export function getFwdSeen(): number {
  return fwdSeen;
}

function counterReducer(s: number, a: string): number {
  return a === "inc" ? s + 1 : s - 1;
}

function RedCounter(): UiElement {
  const [count, dispatch] = useReducer(counterReducer, 10);
  return (
    <box
      onClick={() => dispatch("inc")}
      onPointerDown={() => {
        dispatch("dec");
        dispatch("dec");
      }}
    >
      {count}
    </box>
  );
}

export function reducerApp(): UiElement {
  return <RedCounter />;
}

let store = 5;
const listeners: (() => void)[] = [];

function subscribeStore(cb: () => void): () => void {
  listeners.push(cb);
  return () => {
    const i = listeners.indexOf(cb);
    if (i >= 0) listeners.splice(i, 1);
  };
}

function StoreView(): UiElement {
  const v = useSyncExternalStore(subscribeStore, () => store);
  return <box>{v}</box>;
}

export function storeApp(): UiElement {
  return <StoreView />;
}

export function bumpStore(): void {
  store += 1;
  for (const l of listeners.slice()) l();
}

export function listenerCount(): number {
  return listeners.length;
}

export function radiusApp(ball: boolean): UiElement {
  return <box style={{ width: 50, borderRadius: ball ? "50%" : 4 }} />;
}
