// SPDX-License-Identifier: Apache-2.0

// A React render error after the first mount (M2-LAY-06): React unmounts the root,
// and the UI tree must take the resulting commit without the engine falling over.
import { createRoot, Text } from "@platypus/react";
import { useState } from "react";

let bump = () => {};

function Fragile(props: { n: number }) {
  if (props.n > 0) throw new Error("render failed on purpose");
  return (
    <box style={{ padding: 8, flexDirection: "column", gap: 4 }}>
      <Text>{`count ${props.n}`}</Text>
      <box style={{ flexDirection: "row", gap: 4 }}>
        <Text>a</Text>
        <Text>b</Text>
      </box>
      {/* Absolute boxes are laid out by their containing block (hoisted there). */}
      <box style={{ position: "absolute", left: 4, top: 4, width: 20, height: 10 }}>
        <Text>pin</Text>
      </box>
    </box>
  );
}

function App() {
  const [n, setN] = useState(0);
  bump = () => setN((x) => x + 1);
  return (
    <box style={{ width: "100%", height: "100%", flexDirection: "column" }}>
      <Text>header</Text>
      <Fragile n={n} />
      <Text>footer</Text>
      <box style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%" }}>
        <box style={{ position: "absolute", right: 10, bottom: 10, width: 50, height: 20 }}>
          <Text>overlay</Text>
        </box>
      </box>
    </box>
  );
}

createRoot().render(<App />);
(globalThis as Record<string, unknown>).__bump = () => bump();
