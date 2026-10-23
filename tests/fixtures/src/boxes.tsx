// SPDX-License-Identifier: Apache-2.0

// M2-INT-01/02: a React UI rendered through @iron/react inside the engine's runtime.
// After 100ms (engine time) a timer adds two more grid cells.
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

function App() {
  const [extra, setExtra] = useState(0);
  useEffect(() => {
    const id = setTimeout(() => setExtra(2), 100);
    return () => clearTimeout(id);
  }, []);

  return (
    <box
      style={{
        width: "100%",
        height: "100%",
        flexDirection: "column",
        padding: 10,
        gap: 10,
        backgroundColor: "#202830",
      }}
    >
      <box style={{ height: 50, backgroundColor: "#3a7", borderRadius: 8 }} />
      <box style={{ flexGrow: 1, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
        {Array.from({ length: 2 + extra }, (_, i) => (
          <box
            key={i}
            style={{ backgroundColor: "#567", boxShadow: "0 2px 6px rgba(0,0,0,.5)" }}
          />
        ))}
      </box>
    </box>
  );
}

createRoot().render(<App />);
