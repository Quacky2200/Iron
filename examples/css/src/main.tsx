// M3b showcase: stylesheets. `ui.css` is bundled by Bun into dist/game.css and
// loaded by the engine; classNames select into it. Shows combinators, structural
// pseudo-classes, :has/:not/:empty, attributes (:checked/:disabled), ::before and
// ::after, custom properties, @font-face, and a runtime stylesheet (CSS-in-JS)
// whose accent colour changes every second.
import "./ui.css";

import { addStyleSheet, removeStyleSheet } from "@iron/engine";
import { createRoot, Span, Text } from "@iron/react";
import { type ReactNode, useEffect, useState } from "react";

const accents = ["#5ec8ff", "#9fe6a0", "#ffd25e", "#ff5e7e"];

function Card({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <box className="card">
      <Text className="heading">{title.toUpperCase()}</Text>
      {children}
    </box>
  );
}

function useTicker(): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const id = setTimeout(() => setTick(tick + 1), 1000);
    return () => clearTimeout(id);
  }, [tick]);
  return tick;
}

function Accent({ tick }: { tick: number }) {
  // Overrides the --accent variable; every rule using var(--accent) restyles.
  useEffect(() => {
    const id = addStyleSheet(`:root { --accent: ${accents[tick % accents.length]} }`);
    return () => removeStyleSheet(id);
  }, [tick]);
  return null;
}

function App() {
  const tick = useTicker();
  const highlighted = tick % 5;
  return (
    <box className="app">
      <Accent tick={tick} />
      <Text className="title">
        CSS in <Span className="brand">Iron</Span>
      </Text>

      <box className="grid">
        <Card title="Structural selectors">
          <box className="list">
            {["First (bold)", "Second", "Third", "Fourth", "Last (muted)"].map((label, i) => (
              <Text key={label} className={i === highlighted ? "item on" : "item"}>
                {label}
              </Text>
            ))}
          </box>
        </Card>

        <Card title="::before and ::after">
          <Text className="tag">generated</Text>
          <Text className="quote">Quoted by CSS</Text>
          <box className="bullet">
            <Text>Box bullet</Text>
          </box>
          <box className="bullet">
            <Text>Another one</Text>
          </box>
        </Card>

        <Card title="Attributes and :empty">
          <box className="toggle" checked>
            <box className="box" />
            <Text className="label">Checked</Text>
          </box>
          <box className="toggle">
            <box className="box" />
            <Text className="label">Unchecked</Text>
          </box>
          <box className="toggle" disabled>
            <box className="box" />
            <Text className="label">Disabled</Text>
          </box>
          <box className="slot" />
          <box className="slot">
            <Text>Filled slot</Text>
          </box>
        </Card>

        <Card title="@font-face (Charm)">
          <Text className="fancy">Hello from Charm</Text>
          <Text className="fancy bold">Bold face</Text>
          <Text className="fancy">สวัสดีครับ</Text>
        </Card>

        <Card title="Variables">
          <Text>
            The accent colour comes from <Span className="brand">--accent</Span>, overridden by a stylesheet added at
            runtime. It changes every second.
          </Text>
          <Text id="status">tick {tick}</Text>
        </Card>
      </box>
    </box>
  );
}

createRoot().render(<App />);
