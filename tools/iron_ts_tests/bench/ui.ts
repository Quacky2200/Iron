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
