// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

// Floats: an article whose picture floats left and whose pull-quote floats right, with the text
// wrapping around both, a second picture inside a paragraph of bare text, and `clear` to end the
// wrap. Containers that hold floats are `display: block` (boxes are flex by default).
import "./ui.css";

const PHOTO = "assets/images/landscape.png";

const LOREM =
  "The quick brown fox jumps over the lazy dog while the five boxing wizards jump quickly. " +
  "Pack my box with five dozen liquor jugs, and sphinx of black quartz, judge my vow. " +
  "How vexingly quick daft zebras jump over a field of bright flowers in the summer sun.";

function App() {
  const [side, setSide] = useState<"left" | "right">("left");
  return (
    <box className="page">
      <Text className="title">Floats</Text>

      <box className="article">
        <Image className="figure" src={PHOTO} style={{ float: side }} />
        <box className="quote">
          <Text>"Jump quickly."</Text>
        </box>
        <Text className="body">{LOREM}</Text>
        <Text className="body">{LOREM}</Text>
        <box className="rule" />
      </box>

      <Text className="caption">A picture inside a paragraph of bare text</Text>
      <box className="article" onClick={() => setSide(side === "left" ? "right" : "left")}>
        <Image className="small" src={PHOTO} style={{ float: side === "left" ? "right" : "left" }} />
        {LOREM + " " + LOREM + " (Click to swap sides.)"}
      </box>
    </box>
  );
}

createRoot().render(<App />);
