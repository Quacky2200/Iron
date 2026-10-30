// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Alpha blending probe: translucent fills and borders over an opaque #3060c0 field.
// Each probe box is 100x100 at a known place, so pixels can be read back.
function Probe({ x, style }: { x: number; style: PlatypusStyle }) {
  return <box style={{ position: "absolute", left: x, top: 50, width: 100, height: 100, ...style }} />;
}

function App() {
  return (
    <box style={{ position: "absolute", left: 0, top: 0, width: 800, height: 200, backgroundColor: "#3060c0" }}>
      {/* 1: white fill at 25% */}
      <Probe x={20} style={{ backgroundColor: "rgba(255, 255, 255, 0.25)" }} />
      {/* 2: white border at 25%, 10px, no fill */}
      <Probe x={140} style={{ border: "10px solid rgba(255, 255, 255, 0.25)" }} />
      {/* 3: red fill at 50% */}
      <Probe x={260} style={{ backgroundColor: "rgba(255, 0, 0, 0.5)" }} />
      {/* 4: black border at 25% over a 50% red fill */}
      <Probe x={380} style={{ backgroundColor: "rgba(255, 0, 0, 0.5)", border: "10px solid rgba(0, 0, 0, 0.25)" }} />
      {/* 5: white border at 25% with rounded corners (a dock's case) */}
      <Probe x={500} style={{ border: "10px solid rgba(255, 255, 255, 0.25)", borderRadius: 18 }} />
      {/* 6: white fill at 25% with a backdrop blur (glass) */}
      <Probe x={620} style={{ backgroundColor: "rgba(255, 255, 255, 0.25)", backdropFilter: "blur(4px)" }} />
    </box>
  );
}

createRoot().render(<App />);
