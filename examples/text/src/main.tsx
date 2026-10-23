// SPDX-License-Identifier: Apache-2.0

// M3a showcase: text and fonts. Sizes and weights, italics, mixed spans,
// wrapping and alignment, decorations, text shadows, emoji and CJK through
// system font fallback, a font loaded at runtime, and live-updating text.

// Loaded from the game folder. Charm is a Thai/Latin display font (SIL OFL);
// if the files aren't there, text using it falls back to the default font.
loadFont({ family: "Charm", src: "assets/fonts/Charm-Regular.ttf", weight: 400 });
loadFont({ family: "Charm", src: "assets/fonts/Charm-Bold.ttf", weight: 700 });

const card: IronStyle = {
  backgroundColor: "#1c222c",
  borderRadius: 12,
  border: "1px solid rgba(255,255,255,0.08)",
  padding: 16,
  flexDirection: "column",
  gap: 8,
};

function Card({ title, children, style }: { title: string; children?: ReactNode; style?: IronStyle }) {
  return (
    <box style={{ ...card, ...style }}>
      <Text style={{ fontSize: 12, fontWeight: 700, letterSpacing: "0.12em", color: "#7f8a9c" }}>
        {title.toUpperCase()}
      </Text>
      {children}
    </box>
  );
}

function Clock() {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const id = setTimeout(() => setSeconds(seconds + 1), 1000);
    return () => clearTimeout(id);
  }, [seconds]);
  const mm = String(Math.floor(seconds / 60)).padStart(2, "0");
  const ss = String(seconds % 60).padStart(2, "0");
  return (
    <Text style={{ fontSize: 40, fontWeight: 800, color: "#5ec8ff", textShadow: "0 0 12px rgba(94,200,255,0.6)" }}>
      {mm}:{ss}
    </Text>
  );
}

const lorem =
  "Iron lays out text with Parley and Taffy: this paragraph wraps to the width of its card, and the card grows to fit it. Resize the window to watch it reflow.";

function App() {
  return (
    <box
      style={{
        width: "100%",
        height: "100%",
        padding: 24,
        gap: 20,
        flexDirection: "column",
        backgroundImage: "linear-gradient(160deg, #141a24, #0b0e14)",
        color: "#e8ecf2",
      }}
    >
      <Text style={{ fontSize: 34, fontWeight: 800 }}>
        Text in <Span style={{ color: "#ff8a5e" }}>Iron</Span>
      </Text>

      <box style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 20, flexGrow: 1 }}>
        <Card title="Weights and styles">
          {[300, 400, 600, 800].map((w) => (
            <Text key={w} style={{ fontWeight: w, fontSize: 18 }}>
              Weight {w}
            </Text>
          ))}
          <Text style={{ fontStyle: "italic", fontSize: 18 }}>Italic, from the italic face</Text>
          <Text style={{ textDecoration: "underline", fontSize: 18 }}>Underlined</Text>
          <Text style={{ textDecoration: "line-through", fontSize: 18, color: "#7f8a9c" }}>Struck through</Text>
        </Card>

        <Card title="Wrapping and alignment">
          <Text style={{ lineHeight: 1.5 }}>{lorem}</Text>
          <Text style={{ textAlign: "center", color: "#9fe6a0" }}>Centred line</Text>
          <Text style={{ textAlign: "right", color: "#ffd25e" }}>Right-aligned line</Text>
        </Card>

        <Card title="Loaded font (Charm)">
          <Text style={{ fontFamily: "Charm, serif", fontSize: 30 }}>Hello from Charm</Text>
          <Text style={{ fontFamily: "Charm, serif", fontSize: 30, fontWeight: 700 }}>Bold face</Text>
          <Text style={{ fontFamily: "Charm, serif", fontSize: 26 }}>สวัสดีครับ</Text>
        </Card>

        <Card title="Shadows">
          <Text style={{ fontSize: 28, fontWeight: 800, textShadow: "2px 2px 0 #ff5e7e" }}>Hard shadow</Text>
          <Text style={{ fontSize: 28, fontWeight: 800, textShadow: "0 4px 10px rgba(0,0,0,0.9)" }}>Soft shadow</Text>
          <Text style={{ fontSize: 28, fontWeight: 800, color: "#fff", textShadow: "0 0 8px #ff5e7e, 0 0 20px #ffd25e" }}>
            Glow
          </Text>
        </Card>

        <Card title="Scripts and emoji (system fonts)">
          <Text style={{ fontSize: 22 }}>日本語のテキスト</Text>
          <Text style={{ fontSize: 22 }}>中文文本 · 한국어</Text>
          <Text style={{ fontSize: 22 }}>Ελληνικά · Кириллица</Text>
          <Text style={{ fontSize: 26 }}>🎮 🦀 ⚙️ 🚀</Text>
        </Card>

        <Card title="Live text">
          <Clock />
          <Text style={{ color: "#7f8a9c" }}>Updates through React state every second.</Text>
        </Card>
      </box>
    </box>
  );
}

createRoot().render(<App />);
