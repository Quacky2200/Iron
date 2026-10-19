// The UI benchmark's app, as compiled TSX for the native reconciler. The same app is written for
// React in `js/react/test/perf_entry.tsx`; keep the two in step. A list of `n` keyed rows, each a
// box holding a number: mount it, change one row, change every row, reverse it.

function Row(props: { label: number }): UiElement {
  return <box style={{ width: 10 }}>{props.label}</box>;
}

function App(p: { n: number; rev: boolean; tick: number; all: boolean }): UiElement {
  const items: UiElement[] = [];
  for (let k = 0; k < p.n; k++) {
    const i = p.rev ? p.n - 1 - k : k;
    const label = i === 0 ? p.tick : p.all ? p.tick + i : i;
    items.push(<Row key={"k" + i} label={label} />);
  }
  return <box>{items}</box>;
}

export function perfApp(n: number, rev: boolean, tick: number, all: boolean): UiElement {
  return <App n={n} rev={rev} tick={tick} all={all} />;
}
