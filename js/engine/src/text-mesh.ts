// SPDX-License-Identifier: Apache-2.0

// Text in the world without a UI panel of your own (Unity's `TextMesh`, GDI's `DrawString`):
// a label on its object's XY plane, centred on the object. Iron's UI lays the text out and
// draws it (any font loaded with `loadFont`, line breaks, wrapping), sized to fit, then tints
// it with `color` like a sprite. `pixelsPerUnit` UI pixels make one world unit; `resolution`
// texture pixels per UI pixel keep it sharp up close.

import { Color } from "./math.ts";
import { property } from "./property.ts";
import { Component } from "./scene.ts";

export type TextAlignment = "left" | "center" | "right";

export class TextMesh extends Component {
  private _text = "";
  private _fontSize = 32;
  private _color = Color.white;
  private _font: string | null = null;
  private _fontWeight = 400;
  private _alignment: TextAlignment = "center";
  private _maxWidth = 0;
  private _pixelsPerUnit = 100;
  private _resolution = 2;
  private started = false;
  private dirty = true;

  /** The text; `\n` starts a new line. */
  @property
  get text(): string {
    return this._text;
  }
  set text(t: string) {
    this._text = String(t);
    this.dirty = true;
  }
  /** In UI pixels. */
  @property
  get fontSize(): number {
    return this._fontSize;
  }
  set fontSize(s: number) {
    this._fontSize = Math.max(1, s);
    this.dirty = true;
  }
  @property
  get color(): Color {
    return this._color;
  }
  set color(c: Color) {
    this._color = c;
    this.dirty = true;
  }
  /** A family loaded with `loadFont` (null: Iron's default, Inter). */
  @property
  get font(): string | null {
    return this._font;
  }
  set font(f: string | null) {
    this._font = f;
    this.dirty = true;
  }
  /** 100 to 900; 400 is regular, 700 bold. */
  @property
  get fontWeight(): number {
    return this._fontWeight;
  }
  set fontWeight(w: number) {
    this._fontWeight = w;
    this.dirty = true;
  }
  /** How lines line up with each other. */
  @property
  get alignment(): TextAlignment {
    return this._alignment;
  }
  set alignment(a: TextAlignment) {
    this._alignment = a;
    this.dirty = true;
  }
  /** Wraps lines longer than this many UI pixels (0: only at line breaks). */
  @property
  get maxWidth(): number {
    return this._maxWidth;
  }
  set maxWidth(w: number) {
    this._maxWidth = Math.max(0, w);
    this.dirty = true;
  }
  /** UI pixels per world unit (100 by default, like a sprite's pixels per unit). */
  @property
  get pixelsPerUnit(): number {
    return this._pixelsPerUnit;
  }
  set pixelsPerUnit(p: number) {
    this._pixelsPerUnit = p;
    this.dirty = true;
  }
  /** Texture pixels per UI pixel: more is sharper and costs more memory. */
  @property
  get resolution(): number {
    return this._resolution;
  }
  set resolution(r: number) {
    this._resolution = r;
    this.dirty = true;
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  // Changes made during a frame go to the engine once, just before it's drawn.
  override onPreRender(): void {
    if (this.dirty) this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
  }

  private send(): void {
    if (!this.started) return;
    this.dirty = false;
    const c = this._color;
    __iron_op(
      "render.text",
      this.id,
      this.gameObject.id,
      this._text,
      [c.r, c.g, c.b, c.a],
      this._fontSize,
      this._font,
      this._fontWeight,
      this._alignment,
      this._maxWidth,
      this._pixelsPerUnit,
      this._resolution,
    );
  }
}
