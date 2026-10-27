// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The UI benchmark's app for React (the native one is `tools/platypus_ts_tests/bench/ui_perf.ts`; keep
// them in step). Bundled by bun into an IIFE that QuickJS runs: `__uiStep(n, rev, tick, all)`
// renders one step through the real `@platypus/react` reconciler, whose commits go to `__platypus_commit`.
import { createRoot } from "../src/index.ts";

function Row(props: { label: number }) {
  return <box style={{ width: 10 }}>{props.label}</box>;
}

function App(p: { n: number; rev: boolean; tick: number; all: boolean }) {
  const items = [];
  for (let k = 0; k < p.n; k++) {
    const i = p.rev ? p.n - 1 - k : k;
    const label = i === 0 ? p.tick : p.all ? p.tick + i : i;
    items.push(<Row key={"k" + i} label={label} />);
  }
  return <box>{items}</box>;
}

let root: ReturnType<typeof createRoot> | null = null;

(globalThis as { __uiStep?: (n: number, rev: boolean, tick: number, all: boolean) => void }).__uiStep = (
  n,
  rev,
  tick,
  all,
) => {
  root ??= createRoot();
  root.render(<App n={n} rev={rev} tick={tick} all={all} />);
};
