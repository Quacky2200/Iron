# form

An interaction demo: a profile form with controlled text fields, validation and a character counter,
keyboard- and gamepad-operable toggles and buttons with `:hover` / `:active` / `:focus-visible` /
`:disabled` styles, and a scrolling list of saved profiles that you can click to load. The source comment says
it is the example behind manual check `M3-MAN-01`.

APIs used: `Input`, `TextArea`, `useRef<IronElement>` with `.focus()` and `.scrollTo()`, `IronKeyboardEvent`,
`input.gamepads()` and `input.on("gamepadconnected", ...)`, `import "./form.css"`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/form
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/form
```

Screenshot run: `cargo run -p iron_app -- examples/form --screenshot out.png --frames 30` (the
interactive window may crash some GPUs). Package with `iron-build package ../examples/form`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | `Button`, `Toggle`, `Field`, `usePads`, `App`, the sample data |
| `src/form.css` | Variables, layout, field/toggle/button/list styles with state selectors |

No assets.

## How it works

1. **State.** `App` keeps the form (`Profile`), the saved list (22 generated `SAMPLES`: famous computer
   scientists), a status line and a `touched` flag. A generic setter updates one field:

```tsx
const set = <K extends keyof Profile>(key: K, value: Profile[K]) => setForm((f) => ({ ...f, [key]: value }));
```

2. **Initial focus.** `useEffect(() => nameRef.current?.focus(), [])` focuses the name input at start, so the
   gamepad d-pad has a place to begin.
3. **Validation.** Errors show only after the first Save attempt (`touched`). The email check is a regular
   expression, `validEmail`. After a failed attempt the Save button is `disabled` until the form is valid
   (`disabled={touched && !valid}`).
4. **Controlled fields.** `Input` and `TextArea` take `value` and `onChange`; the bio rejects input past
   `BIO_LIMIT` (140) and the label shows `BIO  n/140`.
5. **Pressable boxes.** `Button` and `Toggle` are `box` elements with `tabIndex={0}`, an `onClick` and an
   `onKeyDown` that handles Enter and Space, so they work with mouse, keyboard and (through the engine's
   gamepad navigation, A to activate) a gamepad:

```tsx
onKeyDown={(e: IronKeyboardEvent) => {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    press();
  }
}}
```

6. **Enter submits.** The form card catches `onKeyDown` bubbling from children and saves when the target is a
   one-line `input` (textareas keep Enter for new lines): `e.target.type === "input"`.
7. **Save and scroll.** Save replaces any profile with the same name, appends, and scrolls the list to the end
   with `listRef.current?.scrollTo(0, 1e9)`. Clicking a row loads it into the form.
8. **Gamepads.** `usePads` reads `input.gamepads()` and re-reads on the connect and disconnect events; the
   header shows the pad names or "No gamepad connected". The effect returns the unsubscribe functions that
   `input.on` gives back.
9. **CSS states.** `form.css` has `.button:hover`, `:active`, `:focus-visible`, `.button.primary`,
   `.button:disabled`, `.toggle:checked .check::after`, `input.invalid`, `.row:nth-child(even)`, `.row.current`.

## Controls

| Input | Effect |
|---|---|
| Tab / Shift+Tab | Move focus |
| Enter / Space | Press the focused button or toggle |
| Enter in the name or email field | Save |
| Wheel, keys, scrollbar | Scroll the saved list |
| Click a row | Load that profile into the form |
| Gamepad d-pad or left stick / A | Move focus / click |

## Things to try

- Add a "Delete" button that removes the selected profile from `saved`.
- Persist the list with `PlayerPrefs` (see how `arcade` stores settings).
- Replace the email regex with your own rules and show the message under the field.
- Reuse `Button` and `Toggle` for a game menu; they need nothing but CSS.
- Add a dropdown built from a `box` list that opens on focus.

## Gotchas

- There is no built-in checkbox or button element: toggles and buttons are boxes you make focusable, and
  you handle the keys yourself.
- Optional chaining on a ref (`nameRef.current?.focus()`) is how the example guards the first render.
- `e.target.value ?? ""`: field values can be undefined in the event, so the example defaults them.

Related examples: css, text, boxes
