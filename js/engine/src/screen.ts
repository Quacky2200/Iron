// The window as scripts see it (Unity's `Screen`). Sizes are in UI pixels, the units
// pointer events (`clientX`/`clientY`) and screen UI use: the window's pixels divided
// by its DPI scale and the UI scale (`setUiScale`). The engine reports changes with
// `__iron_screen(width, height, scale)`.

let width = 1280;
let height = 720;
let scale = 1;

export const Screen = {
  /** Width in UI pixels. */
  get width(): number {
    return width;
  },
  /** Height in UI pixels. */
  get height(): number {
    return height;
  },
  /** Physical pixels per UI pixel (DPI scale × UI scale). */
  get scale(): number {
    return scale;
  },
  get aspect(): number {
    return width / Math.max(1, height);
  },
};

globalThis.__iron_screen = (w: number, h: number, s: number) => {
  width = w;
  height = h;
  scale = s;
};

/** Sets the screen size. Only for tests. */
export function _setScreenForTests(w: number, h: number, s = 1) {
  globalThis.__iron_screen(w, h, s);
}
