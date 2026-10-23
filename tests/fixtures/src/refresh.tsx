// SPDX-License-Identifier: Apache-2.0

// M4-HOT-06: Fast Refresh in QuickJS. Built twice as a dev game bundle (`LABEL` "v1" and
// "v2") against refresh-vendor.js; the Rust test loads v1, changes state, then loads v2
// in the same runtime the way a reload in place does.
import { addStyleSheet, Component, GameObject, register } from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

declare const LABEL: string;

let bump = () => {};

function Counter() {
  const [n, setN] = useState(0);
  bump = () => setN((x) => x + 1);
  (globalThis as { __refresh_seen?: string }).__refresh_seen = `count ${n} (${LABEL})`;
  return (
    <box>
      <text>{`count ${n}`}</text>
    </box>
  );
}

class Ticker extends Component {
  ticks = 0;
  describe() {
    return `${LABEL} ticks=${this.ticks}`;
  }
}

register({ Ticker }, { spawn: false });
addStyleSheet(`.label { color: red }`);
createRoot().render(<Counter />);

const g = globalThis as Record<string, unknown>;
g.__refresh_bump = () => bump();
g.__refresh_spawn = () => {
  new GameObject("ticker").addComponent(Ticker).ticks = 5;
};
g.__refresh_ticker = () => GameObject.findObjectOfType(Ticker)?.describe() ?? "none";
