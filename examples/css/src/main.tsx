// M3b showcase: stylesheets. `ui.css` is bundled by Bun into dist/game.css and
// loaded by the engine; classNames select into it. Shows combinators, structural
// pseudo-classes, :has/:not/:empty, attributes (:checked/:disabled), ::before and
// ::after, custom properties, @font-face, and a runtime stylesheet (CSS-in-JS)
// whose accent colour changes every second. Interaction (M3c): click an item to pin
// the highlight, click the toggles (or Tab to them and press Space), double-click
// the title. The Clipping card shows overflow and clip-path on images, and a list
// that scrolls with the wheel, keys and its scrollbar. The Variables card has text
// fields (a controlled input and a textarea).
import "./ui.css";

import { addStyleSheet, removeStyleSheet } from "@iron/engine";
import { createRoot, Input, Span, Text, TextArea } from "@iron/react";
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

function Toggle({ label, initial = false, disabled = false }: { label: string; initial?: boolean; disabled?: boolean }) {
  const [checked, setChecked] = useState(initial);
  const toggle = () => !disabled && setChecked(!checked);
  return (
    <box
      className="toggle"
      checked={checked}
      disabled={disabled}
      tabIndex={0}
      onClick={toggle}
      onKeyDown={(e) => {
        if (e.key === " " || e.key === "Enter") {
          e.preventDefault();
          toggle();
        }
      }}
    >
      <box className="box" />
      <Text className="label">{label}</Text>
    </box>
  );
}

function Fields() {
  const [name, setName] = useState("");
  return (
    <>
      <Input placeholder="Your name" value={name} onChange={(e) => setName(e.target.value ?? "")} />
      <Text>{name ? `Hello, ${name}!` : "Type your name above."}</Text>
      <TextArea rows={3} placeholder="A textarea: Enter adds lines, Ctrl+A selects all." />
    </>
  );
}

function ScrollList() {
  const [top, setTop] = useState(0);
  return (
    <>
      <Text className="heading">{`SCROLL ${Math.round(top)}PX`}</Text>
      <box className="scroller" onScroll={(e) => setTop(e.scrollTop)}>
        {Array.from({ length: 20 }, (_, i) => (
          <Text key={i} className="row" tabIndex={0}>
            {`Row ${i + 1}`}
          </Text>
        ))}
      </box>
    </>
  );
}

function App() {
  const tick = useTicker();
  const [pinned, setPinned] = useState<number | null>(null);
  const [titleClicks, setTitleClicks] = useState(0);
  const highlighted = pinned ?? tick % 5;
  return (
    <box className="app">
      <Accent tick={tick} />
      <Text className="title" onDoubleClick={() => setTitleClicks(titleClicks + 1)}>
        CSS in <Span className="brand">Iron</Span>
        {titleClicks > 0 ? ` (double-clicked ${titleClicks}x)` : ""}
      </Text>

      <box className="grid">
        <Card title="Structural selectors">
          <box className="list">
            {["First (bold)", "Second", "Third", "Fourth", "Last (muted)"].map((label, i) => (
              <Text
                key={label}
                className={i === highlighted ? "item on" : "item"}
                onClick={() => setPinned(pinned === i ? null : i)}
              >
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
          <Toggle label="Checked" initial />
          <Toggle label="Unchecked" />
          <Toggle label="Disabled" disabled />
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
          <Fields />
        </Card>

        <Card title="Clipping">
          <box className="shapes">
            <box className="avatar">
              <box className="photo" />
            </box>
            <box className="shape circle" />
            <box className="shape ticket" />
            <box className="shape lens" />
          </box>
          <ScrollList />
        </Card>
      </box>
    </box>
  );
}

createRoot().render(<App />);
