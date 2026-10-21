// M2 showcase: React UI rendered by Iron. Flexbox and grid layout, gradients,
// images, borders, per-corner radii, box shadows, blur filters, frosted glass and
// group opacity, plus a timer-driven counter that adds and removes boxes.

const card: IronStyle = {
  backgroundColor: "#1e2633",
  borderRadius: 14,
  border: "1px solid rgba(255,255,255,0.08)",
  boxShadow: "0 10px 24px rgba(0,0,0,0.45)",
  padding: 14,
  flexDirection: "column",
  gap: 10,
};

function Card({ style, children }: { style?: IronStyle; children?: ReactNode }) {
  return <box style={{ ...card, ...style }}>{children}</box>;
}

/** A demo tile filling its card. */
function Swatch({ style }: { style: IronStyle }) {
  return <box style={{ flexGrow: 1, borderRadius: 10, ...style }} />;
}

/** Counts 1 → 8 → 1 forever, one step every 400ms (engine time). */
function useBouncingCounter() {
  const [n, setN] = useState(1);
  const [dir, setDir] = useState(1);
  useEffect(() => {
    const id = setTimeout(() => {
      const next = n + dir;
      if (next > 8 || next < 1) setDir(-dir);
      setN(Math.min(8, Math.max(1, next)));
    }, 400);
    return () => clearTimeout(id);
  }, [n, dir]);
  return n;
}

function Counter() {
  const n = useBouncingCounter();
  const hue = (i: number) => `hsl(${200 + i * 18} 80% 60%)`;
  return (
    <Card style={{ gridColumn: "1 / -1" }}>
      <box style={{ flexDirection: "row", gap: 8, height: 36 }}>
        {Array.from({ length: n }, (_, i) => (
          <box
            key={i}
            style={{
              width: 36,
              borderRadius: "50%",
              backgroundColor: hue(i),
              boxShadow: `0 0 12px ${hue(i)}`,
            }}
          />
        ))}
      </box>
    </Card>
  );
}

function Blobs() {
  // Colourful shapes behind the glass header, so the backdrop blur is visible.
  const blob = (left: number, top: number, size: number, color: string): IronStyle => ({
    position: "absolute",
    left,
    top,
    width: size,
    height: size,
    borderRadius: "50%",
    backgroundImage: `radial-gradient(circle, ${color}, rgba(0,0,0,0) 70%)`,
  });
  return (
    <>
      <box style={blob(40, -40, 220, "#ff5e7e")} />
      <box style={blob(420, -60, 260, "#5ec8ff")} />
      <box style={blob(860, -30, 200, "#ffd25e")} />
      <box style={{ position: "absolute", left: 640, top: 10, width: 120, height: 60, backgroundImage: "url(assets/stripes.png)", borderRadius: 30 }} />
    </>
  );
}

function App() {
  return (
    <box
      style={{
        width: "100%",
        height: "100%",
        flexDirection: "column",
        padding: 24,
        gap: 20,
        backgroundImage: "linear-gradient(160deg, #141a24, #0b0e14)",
      }}
    >
      <Blobs />

      {/* Frosted-glass header */}
      <box
        style={{
          height: 72,
          borderRadius: 18,
          backgroundColor: "rgba(255,255,255,0.08)",
          backdropFilter: "blur(18px)",
          border: "1px solid rgba(255,255,255,0.18)",
          boxShadow: "inset 0 1px 0 rgba(255,255,255,0.25), 0 8px 30px rgba(0,0,0,0.35)",
          flexDirection: "row",
          alignItems: "center",
          padding: "0 20px",
          gap: 12,
        }}
      >
        <box style={{ width: 36, height: 36, borderRadius: 10, backgroundImage: "linear-gradient(135deg, #ff5e7e, #ffd25e)" }} />
        <box style={{ flexGrow: 1 }} />
        {[0, 1, 2].map((i) => (
          <box key={i} style={{ width: 70, height: 28, borderRadius: 14, backgroundColor: "rgba(255,255,255,0.12)" }} />
        ))}
      </box>

      <box
        style={{
          flexGrow: 1,
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gridTemplateRows: "1fr 1fr auto",
          gap: 20,
        }}
      >
        <Card>
          <Swatch style={{ backgroundImage: "linear-gradient(to bottom right, #6a5cff, #00d4ff)" }} />
        </Card>
        <Card>
          <Swatch style={{ backgroundImage: "url(assets/stripes.png)", backgroundSize: "24px" }} />
        </Card>
        <Card>
          <Swatch style={{ backgroundColor: "#f5f7fb", boxShadow: "inset 0 0 24px 4px rgba(30,40,80,0.45)" }} />
        </Card>
        <Card>
          <Swatch style={{ backgroundColor: "#2ecc71", borderRadius: "40px 4px", border: "4px solid #a8f0c6" }} />
        </Card>
        <Card>
          <Swatch style={{ backgroundImage: "radial-gradient(circle, #fff6, #ff5e7e 40%, #3b1d5e)" }} />
        </Card>
        <Card>
          {/* filter: blur applies to the box and its children */}
          <box style={{ flexGrow: 1, borderRadius: 10, backgroundColor: "#ffd25e", filter: "blur(6px)", alignItems: "center", justifyContent: "center" }}>
            <box style={{ width: 50, height: 50, borderRadius: 25, backgroundColor: "#ff5e7e" }} />
          </box>
        </Card>
        <Card>
          {/* group opacity: the child covers the parent before fading together */}
          <box style={{ flexGrow: 1, borderRadius: 10, backgroundColor: "#5ec8ff", opacity: 0.5, padding: 20 }}>
            <box style={{ flexGrow: 1, borderRadius: 8, backgroundColor: "#ffffff" }} />
          </box>
        </Card>
        <Card style={{ backgroundImage: "url(assets/stripes.png)", backgroundSize: "contain", backgroundRepeat: "no-repeat", backgroundPosition: "center" }} />
        <Counter />
      </box>
    </box>
  );
}

createRoot().render(<App />);
