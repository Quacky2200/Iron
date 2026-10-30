// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Repro for four paint/layout issues the Fluid desktop hit (its docs/platypus.md, items 1-4), and
// containing blocks and clips of absolute boxes (5, 6).
// Every cell is a dark square; what should appear in it is red (in row 1: glass whose colour
// stays even). Each case sits next to a control that does the same thing a way that works. Cases
// are drawn at Fluid's size (1x, 6-12px) and at 8x.

const RED = "#ff3b30";

const cell: PlatypusStyle = {
  width: 150,
  height: 110,
  backgroundColor: "#2b2b30",
  borderRadius: 6,
  padding: 8,
  flexDirection: "column",
  gap: 6,
};

function Cell({ label, children }: { label: string; children?: ReactNode }) {
  return (
    <box style={cell}>
      <text style={{ fontSize: 10, color: "#ddd" }}>{label}</text>
      <box style={{ flexGrow: 1, flexDirection: "row", alignItems: "flex-start" }}>{children}</box>
    </box>
  );
}

function Row({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <box style={{ flexDirection: "column", gap: 6 }}>
      <text style={{ fontSize: 13, color: "white", fontWeight: 700 }}>{title}</text>
      <box style={{ flexDirection: "row", gap: 8, flexWrap: "wrap" }}>{children}</box>
    </box>
  );
}

// Item 1: two glass boxes stacked over a backdrop that is blue behind the top one and orange
// behind the bottom one. Browsers blur only what is inside each box, so the bottom glass is
// evenly orange; blue reaching into its top rows is the bug.
const glass = "blur(10px) saturate(1.6)";

function Glass({ top }: { top: boolean }) {
  return (
    <box
      style={{
        position: "relative",
        width: 134,
        height: 76,
        backgroundImage: top
          ? "linear-gradient(#2060ff 28px, #ff8020 28px)"
          : "linear-gradient(#ff8020, #ff8020)",
      }}
    >
      {top ? (
        <box
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            right: 0,
            height: 28,
            backgroundColor: "rgba(246,246,250,0.35)",
            backdropFilter: glass,
          }}
        />
      ) : null}
      <box
        style={{
          position: "absolute",
          left: 0,
          top: 28,
          right: 0,
          bottom: 0,
          backgroundColor: "rgba(246,246,250,0.62)",
          backdropFilter: glass,
        }}
      />
    </box>
  );
}

/**
 * Fluid's chain: a zero-size relative anchor in flow, an absolute `overflow: hidden` clip of
 * `clip` px hanging off it at top 3px, then `fill` inside the clip.
 */
function Clipped({ clip, fill }: { clip: number; fill: PlatypusStyle }) {
  return (
    <box style={{ position: "relative", width: 0, height: 0 }}>
      <box style={{ position: "absolute", top: 3, left: 0, width: clip, height: clip, overflow: "hidden" }}>
        <box style={fill} />
      </box>
    </box>
  );
}

// Item 2: a centred radial gradient, clear inside a radius, red outside: in a clip of half its size
// only a quarter shows, so the red should fill the clip's corner outside a quarter circle. The
// clip is a flex row (every box's default), so as in CSS an empty item wider than it shrinks to
// its width (2a, 2b, 3b, 3e): `flexShrink: 0` (2e, 3g) keeps the size Fluid meant.
function gradient(size: number, inner: number, outer: number): PlatypusStyle {
  return {
    width: size,
    height: size,
    backgroundImage: "radial-gradient(circle, transparent " + inner + "px, " + RED + " " + outer + "px)",
  };
}

// Item 3: a box rounded at one corner whose shadow should paint outside the curve (CSS never
// paints an outer shadow under its own box).
function spread(s: number): PlatypusStyle {
  return { width: s, height: s, borderTopLeftRadius: s, boxShadow: "0 0 0 " + s + "px " + RED };
}

function negative(s: number): PlatypusStyle {
  return { width: 2 * s, height: s, borderTopLeftRadius: s, boxShadow: -s + "px 0 0 0 " + RED };
}

function mirrored(s: number): PlatypusStyle {
  return { width: 2 * s, height: s, marginLeft: -s, borderTopRightRadius: s, boxShadow: s + "px 0 0 0 " + RED };
}

function ear(s: number): PlatypusStyle {
  return { width: s, height: s, borderBottomLeftRadius: s, boxShadow: -s / 2 + "px " + s / 2 + "px 0 0 " + RED };
}

// Item 4: Fluid's menu bar at the time, inline. The bar is absolute in a fixed 134x40 box; the
// title (`item`) is a row with no `position` (or `relative` in the control) and no fixed width;
// the red 8x8 box is absolute inside the title with the given insets.
function MenuBar({ itemPosition, ear }: { itemPosition: PlatypusStyle["position"]; ear: PlatypusStyle }) {
  return (
    <box style={{ position: "relative", width: 134, height: 40, backgroundColor: "#444" }}>
      <box
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          right: 0,
          height: 28,
          flexDirection: "row",
          justifyContent: "space-between",
          alignItems: "center",
          padding: "0 10px",
          backgroundColor: "#666",
        }}
      >
        <box style={{ flexDirection: "row", alignItems: "center" }}>
          <box
            style={{
              position: itemPosition,
              height: 22,
              padding: "0 9px",
              flexDirection: "row",
              alignItems: "center",
              backgroundColor: "#999",
            }}
          >
            <text style={{ fontSize: 12, color: "black" }}>File</text>
            <box style={{ position: "absolute", width: 8, height: 8, backgroundColor: RED, ...ear }} />
          </box>
        </box>
      </box>
    </box>
  );
}

// Item 5: a static grey box that is a containing block anyway (as CSS says: a transform, a filter,
// a backdrop filter, `willChange` naming one, `contain: paint`) holds its absolute red square in
// its corner. Before, the square went to the window's bottom-right corner.
function Holder({ style }: { style: PlatypusStyle }) {
  return (
    <box style={{ width: 64, height: 64, marginLeft: 30, marginTop: 4, backgroundColor: "#555", ...style }}>
      <box style={{ position: "absolute", right: 0, bottom: 0, width: 16, height: 16, backgroundColor: RED }} />
    </box>
  );
}

// Item 6: a 24px menu bar with `overflow: hidden` and an absolute red dropdown (`top: 18px`) inside
// an item. The frame around it is `position: relative`, so it is the dropdown's containing block:
// a static bar doesn't clip it (CSS), and the whole menu shows below the bar. A positioned or
// transformed bar is the containing block itself and clips it to a 6px strip; a `clipPath` on
// the item clips it wherever its containing block is.
function Dropdown({ bar, item }: { bar: PlatypusStyle; item?: PlatypusStyle }) {
  return (
    <box style={{ position: "relative", width: 134, height: 80 }}>
      <box
        style={{
          width: 120,
          height: 24,
          overflow: "hidden",
          borderRadius: 4,
          backgroundColor: "#666",
          flexDirection: "row",
          ...bar,
        }}
      >
        <box style={{ width: 40, height: 24, backgroundColor: "#999" }} />
        <box style={item ?? {}}>
          <box style={{ position: "absolute", left: 8, top: 18, width: 80, height: 56, backgroundColor: RED }} />
        </box>
      </box>
    </box>
  );
}

function App() {
  return (
    <box style={{ width: "100%", height: "100%", backgroundColor: "#111", padding: 16, flexDirection: "column", gap: 14 }}>
      <Row title="1. stacked backdrop-filter glass (bottom glass evenly orange)">
        <Cell label="1a glass over glass">
          <Glass top={true} />
        </Cell>
        <Cell label="1b control: bottom glass alone">
          <Glass top={false} />
        </Cell>
      </Row>
      <Row title="2. radial-gradient in a clip (red outside a quarter circle)">
        <Cell label="2a 1x: 12px box, 6px clip">
          <Clipped clip={6} fill={gradient(12, 5.5, 6.5)} />
        </Cell>
        <Cell label="2b 8x: 96px box, 48px clip">
          <Clipped clip={48} fill={gradient(96, 44, 52)} />
        </Cell>
        <Cell label="2c control 8x: no clip">
          <box style={gradient(96, 44, 52)} />
        </Cell>
        <Cell label="2d control: % stops, no clip">
          <box style={{ width: 96, height: 96, backgroundImage: "radial-gradient(circle, transparent 60%, " + RED + " 62%)" }} />
        </Cell>
        <Cell label="2e 2b with flexShrink 0">
          <Clipped clip={48} fill={{ ...gradient(96, 44, 52), flexShrink: 0 }} />
        </Cell>
      </Row>
      <Row title="3. box-shadow outside a rounded corner, in a clip (red outside the curve)">
        <Cell label="3a 8x spread 0 0 0 48px">
          <Clipped clip={48} fill={spread(48)} />
        </Cell>
        <Cell label="3b 8x offset -48px 0">
          <Clipped clip={48} fill={negative(48)} />
        </Cell>
        <Cell label="3c control 8x: mirrored +48px">
          <Clipped clip={48} fill={mirrored(48)} />
        </Cell>
        <Cell label="3d control 8x: ear -32px 32px">
          <Clipped clip={64} fill={ear(64)} />
        </Cell>
        <Cell label="3e 1x: spread, -6px, +6px">
          <Clipped clip={6} fill={spread(6)} />
          <box style={{ width: 20 }} />
          <Clipped clip={6} fill={negative(6)} />
          <box style={{ width: 20 }} />
          <Clipped clip={6} fill={mirrored(6)} />
        </Cell>
        <Cell label="3f no clip: spread, -48px">
          <box style={{ ...spread(32), marginLeft: 32, marginTop: 32 }} />
          <box style={{ ...negative(24), marginLeft: 40, marginTop: 8 }} />
        </Cell>
        <Cell label="3g 3b with flexShrink 0">
          <Clipped clip={48} fill={{ ...negative(48), flexShrink: 0 }} />
        </Cell>
      </Row>
      <Row title="4. absolute insets in a title of a centred row (red square)">
        <Cell label="4a static, left 100% bottom 0">
          <MenuBar itemPosition="static" ear={{ left: "100%", bottom: 0 }} />
        </Cell>
        <Cell label="4b relative, left 100% bottom 0">
          <MenuBar itemPosition="relative" ear={{ left: "100%", bottom: 0 }} />
        </Cell>
        <Cell label="4c relative, right -8px top 0">
          <MenuBar itemPosition="relative" ear={{ right: -8, top: 0 }} />
        </Cell>
        <Cell label="4d relative, right 0 bottom 0">
          <MenuBar itemPosition="relative" ear={{ right: 0, bottom: 0 }} />
        </Cell>
        <Cell label="4e control: relative, left 0 top 0">
          <MenuBar itemPosition="relative" ear={{ left: 0, top: 0 }} />
        </Cell>
      </Row>
      <Row title="5. a static box that is a containing block anyway (red square in the grey corner)">
        <Cell label="5a transform">
          <Holder style={{ transform: "rotate(0deg)" }} />
        </Cell>
        <Cell label="5b filter">
          <Holder style={{ filter: "blur(0.5px)" }} />
        </Cell>
        <Cell label="5c backdrop-filter">
          <Holder style={{ backdropFilter: "blur(2px)" }} />
        </Cell>
        <Cell label="5d will-change: transform">
          <Holder style={{ willChange: "transform" }} />
        </Cell>
        <Cell label="5e contain: paint">
          <Holder style={{ contain: "paint" }} />
        </Cell>
        <Cell label="5f control: relative">
          <Holder style={{ position: "relative" }} />
        </Cell>
      </Row>
      <Row title="6. a dropdown in an overflow: hidden bar (red menu below a static bar, a strip otherwise)">
        <Cell label="6a static bar: escapes">
          <Dropdown bar={{}} />
        </Cell>
        <Cell label="6b control: relative bar clips">
          <Dropdown bar={{ position: "relative" }} />
        </Cell>
        <Cell label="6c transformed bar clips">
          <Dropdown bar={{ transform: "rotate(0deg)" }} />
        </Cell>
        <Cell label="6d clipPath on the item clips">
          <Dropdown bar={{}} item={{ flexGrow: 1, clipPath: "inset(0 0 -30px 0)" }} />
        </Cell>
      </Row>
    </box>
  );
}

createRoot().render(<App />);
