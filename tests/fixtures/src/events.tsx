// M3-EVT-02/03: pointer events from Rust reach React handlers. The button counts
// its clicks and stops them from bubbling; clicks elsewhere count on the panel.
// Pressing the button prevents the default action.
import { createRoot, Text } from "@iron/react";
import { useState } from "react";

function App() {
  const [clicks, setClicks] = useState(0);
  const [panelClicks, setPanelClicks] = useState(0);
  return (
    <box
      className="panel"
      style={{ width: "100%", height: "100%", padding: 20, flexDirection: "column", gap: 10 }}
      onClick={() => setPanelClicks((n) => n + 1)}
    >
      <box
        className="button"
        style={{ width: 100, height: 40, cursor: "pointer" }}
        onPointerDown={(e) => e.preventDefault()}
        onClick={(e) => {
          e.stopPropagation();
          setClicks((n) => n + 1);
        }}
      >
        <Text>{`clicks ${clicks}`}</Text>
      </box>
      <Text>{`panel ${panelClicks}`}</Text>
    </box>
  );
}

createRoot().render(<App />);
