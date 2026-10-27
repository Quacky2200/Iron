// SPDX-License-Identifier: Apache-2.0

// The OS mouse cursor (Unity's `Cursor`): show or hide it, hold it in the window, and
// give it an image. UI elements' CSS `cursor` still wins where they set one; the image
// here replaces the default arrow everywhere else.

import type { Vector2Like } from "./math.ts";
import type { Texture } from "./textures.ts";

/** How the cursor is held (Unity's `CursorLockMode`). */
export const CursorLockMode = {
  /** Free to leave the window. */
  None: "none",
  /**
   * Hidden and held in place, for mouse look: read `input.mouseDelta` or
   * `getAxis("Mouse X")`. (Windows can't pin it, so there it's confined and hidden.)
   */
  Locked: "locked",
  /** Kept inside the window. */
  Confined: "confined",
} as const;

export type CursorLockMode = (typeof CursorLockMode)[keyof typeof CursorLockMode];

let visible = true;
let lockState: CursorLockMode = "none";

export const Cursor = {
  /** Whether the cursor shows over the window (locking hides it as well). */
  get visible(): boolean {
    return visible;
  },
  set visible(v: boolean) {
    visible = v;
    __platypus_op("cursor.visible", v);
  },

  get lockState(): CursorLockMode {
    return lockState;
  },
  set lockState(mode: CursorLockMode) {
    lockState = mode;
    __platypus_op("cursor.lock", mode);
  },

  /**
   * Shows an image as the cursor (PNG, JPEG or `.cur`, up to 128 px), with `hotspot`
   * (in image pixels) being the point that clicks. `null` goes back to the arrow.
   * Takes a path in the game folder or a texture loaded from one.
   */
  setCursor(image: Texture | string | null, hotspot?: Vector2Like): void {
    if (image === null) {
      __platypus_op("cursor.image", null);
      return;
    }
    const src = typeof image === "string" ? image : image.src;
    if (!src) {
      console.warn("Cursor.setCursor: the texture must come from a file (Texture.load)");
      return;
    }
    __platypus_op("cursor.image", src, hotspot ? [hotspot.x, hotspot.y] : null);
  },
};

/** Resets the settings. Only for tests. */
export function _resetCursorForTests() {
  visible = true;
  lockState = "none";
}
