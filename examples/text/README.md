# text

A single-screen showcase of Iron's text rendering: font weights and styles, wrapping and alignment, a
font loaded at run time, text shadows, CJK/Greek/Cyrillic/emoji through system font fallback, and text that
updates through React state. The UI is React drawn by Iron's own renderer; there is no 3D scene.

APIs used: `createRoot().render(...)`, the `box`, `Text` and `Span` elements, `IronStyle` objects,
`loadFont`, and the hooks `useState` / `useEffect`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/text
```

Then from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/text
```

For a still image: `cargo run -p iron_app -- examples/text --screenshot out.png --frames 30`. The
interactive window may crash some GPUs; the screenshot run is the safest. A packaged exe comes from
`iron-build package ../examples/text`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only (window title "Iron - Text") |
| `tsconfig.json` | Extends the base config with `"jsx": "react-jsx"` |
| `src/main.tsx` | Everything: `loadFont` calls, `Card`, `Clock`, `App`, `createRoot().render(<App />)` |

Assets: the "Charm" font is loaded from `assets/fonts/Charm-Regular.ttf` and `Charm-Bold.ttf`. The font
is not committed to the repo (it is kept as local copies per example); in this checkout there is no
`examples/text/assets` folder. As the source comment says, if the files are missing, text using
`fontFamily: "Charm, serif"` falls back to the default font.

## How it works

1. **Fonts.** At the top level two faces of one family are registered, one per weight:

```ts
loadFont({ family: "Charm", src: "assets/fonts/Charm-Regular.ttf", weight: 400 });
loadFont({ family: "Charm", src: "assets/fonts/Charm-Bold.ttf", weight: 700 });
```

2. **Layout.** `App` is a column box filling the window (`width: "100%"`, `height: "100%"`) with a
   gradient background (`backgroundImage: "linear-gradient(160deg, #141a24, #0b0e14)"`), a title and a
   `display: "grid"` box with `gridTemplateColumns: "repeat(3, 1fr)"`. Layout is Taffy (flexbox and grid).
3. **Cards.** `Card` spreads a shared `IronStyle` object into a `box` and puts an upper-cased,
   letter-spaced `Text` heading over its children.
4. **Weights and styles.** Four `fontWeight` values (300, 400, 600, 800) are mapped over an array, then
   `fontStyle: "italic"` and `textDecoration` `underline` / `line-through`.
5. **Wrapping and alignment.** A long string in a `Text` wraps to the card width (`lineHeight: 1.5`);
   `textAlign` centre and right follow. Resize the window to see it reflow.
6. **Mixed spans.** `Span` colours part of a line:

```tsx
<Text style={{ fontSize: 34, fontWeight: 800 }}>
  Text in <Span style={{ color: "#ff8a5e" }}>Iron</Span>
</Text>
```

7. **Shadows.** `textShadow` takes one or several CSS shadows: a hard offset one, a blurred one and a
   two-layer glow (`"0 0 8px #ff5e7e, 0 0 20px #ffd25e"`).
8. **Fallback fonts.** Japanese, Chinese, Korean, Greek, Cyrillic and emoji strings use no custom font; the
   system fonts supply the glyphs.
9. **Live text.** `Clock` bumps a counter with a timeout inside `useEffect`, so the text changes each
   second through React state:

```tsx
useEffect(() => {
  const id = setTimeout(() => setSeconds(seconds + 1), 1000);
  return () => clearTimeout(id);
}, [seconds]);
```

## Controls

None; the window can be resized and the text reflows.

## Things to try

- Change `textShadow` values, or change the heading's `letterSpacing`.
- Register your own font with another `loadFont` call and use it by `fontFamily`.
- Replace the clock with a game timer, formatted with `padStart` as here.
- Move the styles into a stylesheet (see `css`) and use `className` instead of `style`.
- Put the text on a plane in the 3D world with `WorldPanel` (see `docs/ui-tree.md` and `docs/react.md`).

## Gotchas

- Style values are plain JS objects (camelCase); there is no CSS file here. Strings such as `"2px 2px 0 #ff5e7e"` are parsed as CSS.
- Missing font files do not fail the build or the run; the text silently uses the default font.
- This example uses `setTimeout` / `clearTimeout` as in the browser; the engine provides them.
- This example uses the `Text` / `Span` components; other examples (`arcade`, `animation`) use the lowercase `text` element. I did not check where the two differ.

Related examples: css, boxes, form
