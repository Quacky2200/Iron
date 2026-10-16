// M3-EVT-02/03/04: UI events from Rust reach React handlers. The button counts its
// clicks, stops them from bubbling, prevents pointerdown (so it doesn't steal focus)
// and focuses the field through a ref. Clicks elsewhere count on the panel. The
// field (tabIndex 0) shows the keys typed into it; Escape blurs it. The name input
// is controlled: it upper-cases what's typed and refuses digits.
import { createRoot, Input, type IronElement, Text } from "@iron/react";
import { useRef, useState } from "react";

function App() {
  const [clicks, setClicks] = useState(0);
  const [panelClicks, setPanelClicks] = useState(0);
  const [typed, setTyped] = useState("");
  const [focused, setFocused] = useState(false);
  const [name, setName] = useState("");
  const field = useRef<IronElement>(null);
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
          field.current?.focus();
        }}
      >
        <Text>{`clicks ${clicks}`}</Text>
      </box>
      <Text>{`panel ${panelClicks}`}</Text>
      <box
        className="field"
        ref={field}
        tabIndex={0}
        style={{ width: 200, height: 30 }}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onKeyDown={(e) => {
          if (e.key === "Escape") field.current?.blur();
          else if (e.key === "Backspace") setTyped((t) => t.slice(0, -1));
          else if (e.key.length === 1) setTyped((t) => t + e.key);
        }}
      >
        <Text>{`${focused ? "focused" : "idle"}: ${typed}`}</Text>
      </box>
      <Input
        className="name"
        value={name}
        onChange={(e) => {
          const v = e.target.value ?? "";
          if (!/[0-9]/.test(v)) setName(v.toUpperCase());
        }}
      />
    </box>
  );
}

createRoot().render(<App />);
