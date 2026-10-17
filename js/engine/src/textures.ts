// Textures and sprites (Unity's `Texture2D`, `Sprite` and `SpriteRenderer`). Textures
// load in the engine (`render.texture`), which reports their sizes back through
// `__iron_textures` once loaded; sprites are drawn as quads facing the object's +z,
// unlit, sorted with the other see-through things by distance.

import { property } from "./property.ts";
import { Color, Mathf, Rect, Vector2, type Vector2Like } from "./math.ts";
import { Component } from "./scene.ts";

/** How texels blend when sampled (Unity's `FilterMode`). */
export type FilterMode = "point" | "bilinear" | "trilinear";
/** What happens outside the texture (Unity's `TextureWrapMode`). */
export type TextureWrapMode = "repeat" | "clamp" | "mirror";

export interface TextureOptions {
  /** Default `bilinear`; `point` keeps pixel art crisp. */
  filterMode?: FilterMode;
  /** Default `repeat`. */
  wrapMode?: TextureWrapMode;
  /** Smaller copies for distant sampling. Default true. */
  mipmaps?: boolean;
}

let nextTexture = 1;
const textures = new Map<number, Texture>();

/**
 * An image the engine draws with: loaded from a file in the game folder
 * (`Texture.load`) or made from pixels (`Texture.create`). Its size is known once it
 * has loaded (`isLoaded`, `whenLoaded()`). If it fails, meshes and sprites using it draw
 * the magenta missing checker.
 */
export class Texture {
  readonly id: number;
  /** The file it came from, if any. */
  readonly src: string | null;
  width = 0;
  height = 0;
  isLoaded = false;
  /** True when loading failed (the engine logs why). */
  failed = false;
  private _filterMode: FilterMode;
  private _wrapMode: TextureWrapMode;
  private _mipmaps: boolean;
  private pixels: number[] | null = null;
  private waiting: ((t: Texture) => void)[] = [];

  private constructor(src: string | null, options: TextureOptions) {
    this.id = nextTexture++;
    this.src = src;
    this._filterMode = options.filterMode ?? "bilinear";
    this._wrapMode = options.wrapMode ?? "repeat";
    this._mipmaps = options.mipmaps ?? true;
    textures.set(this.id, this);
  }

  /** @internal A texture the engine made (a model's), already loaded. */
  static _engine(id: number, width: number, height: number): Texture {
    const t = new Texture(null, {});
    textures.delete(t.id);
    nextTexture--;
    (t as { id: number }).id = id;
    t.width = width;
    t.height = height;
    t.isLoaded = true;
    return t;
  }

  /** Loads a PNG or JPEG relative to the game folder. */
  static load(src: string, options: TextureOptions = {}): Texture {
    const t = new Texture(src, options);
    t.send();
    return t;
  }

  /**
   * A texture from linear colours, bottom row first like Unity's `SetPixels`
   * (`width × height` of them).
   */
  static create(width: number, height: number, colors: readonly Color[], options: TextureOptions = {}): Texture {
    if (colors.length !== width * height) {
      throw new Error(`Texture.create: ${colors.length} colours for ${width}x${height}`);
    }
    const t = new Texture(null, options);
    const px: number[] = new Array(width * height * 4);
    const byte = (v: number) => Math.round(Mathf.clamp01(Mathf.linearToGammaSpace(Mathf.clamp01(v))) * 255);
    for (let y = 0; y < height; y++) {
      // The engine takes the top row first.
      const row = height - 1 - y;
      for (let x = 0; x < width; x++) {
        const c = colors[y * width + x]!;
        const i = (row * width + x) * 4;
        px[i] = byte(c.r);
        px[i + 1] = byte(c.g);
        px[i + 2] = byte(c.b);
        px[i + 3] = Math.round(Mathf.clamp01(c.a) * 255);
      }
    }
    t.pixels = px;
    t.width = width;
    t.height = height;
    t.send();
    return t;
  }

  get filterMode(): FilterMode {
    return this._filterMode;
  }
  /** Changing it uploads the texture again. */
  set filterMode(f: FilterMode) {
    this._filterMode = f;
    this.send();
  }
  get wrapMode(): TextureWrapMode {
    return this._wrapMode;
  }
  set wrapMode(w: TextureWrapMode) {
    this._wrapMode = w;
    this.send();
  }

  /** Resolves once the size is known (at once if it already is). Rejects if it failed. */
  whenLoaded(): Promise<Texture> {
    if (this.isLoaded) return Promise.resolve(this);
    if (this.failed) return Promise.reject(new Error(`texture ${this.src ?? this.id} failed to load`));
    return new Promise((resolve, reject) => {
      this.waiting.push((t) => (t.failed ? reject(new Error(`texture ${t.src ?? t.id} failed to load`)) : resolve(t)));
    });
  }

  /** Frees it; things still using it draw the magenta missing checker. */
  destroy(): void {
    textures.delete(this.id);
    __iron_op("render.textureRemove", this.id);
  }

  private send(): void {
    __iron_op(
      "render.texture",
      this.id,
      this.src,
      this._filterMode,
      this._wrapMode,
      this._mipmaps,
      ...(this.pixels ? [this.width, this.height, this.pixels] : []),
    );
  }

  /** @internal The engine loaded it (`width` 0: it failed). */
  _loaded(width: number, height: number): void {
    this.failed = width === 0;
    this.isLoaded = !this.failed;
    if (!this.failed) {
      this.width = width;
      this.height = height;
    }
    const waiting = this.waiting;
    this.waiting = [];
    for (const f of waiting) f(this);
  }
}

globalThis.__iron_textures = (loaded: [number, number, number][]) => {
  for (const [id, w, h] of loaded) textures.get(id)?._loaded(w, h);
};

/**
 * Part of a texture to draw as a 2D image (Unity's `Sprite`): `rect` in pixels from
 * the texture's bottom-left (all of it when null), `pivot` (0..1 across the rect from its
 * bottom-left) is where the object's origin sits, and `pixelsPerUnit` texels make a metre.
 */
export class Sprite {
  private constructor(
    readonly texture: Texture,
    readonly rect: Rect | null,
    readonly pivot: Vector2,
    readonly pixelsPerUnit: number,
  ) {}

  static create(texture: Texture, rect: Rect | null = null, pivot: Vector2Like = new Vector2(0.5, 0.5), pixelsPerUnit = 100): Sprite {
    return new Sprite(texture, rect, new Vector2(pivot.x, pivot.y), pixelsPerUnit);
  }

  /** Its size in world units (zero until the texture has loaded, for a whole-texture sprite). */
  get size(): Vector2 {
    const w = this.rect?.width ?? this.texture.width;
    const h = this.rect?.height ?? this.texture.height;
    return new Vector2(w / this.pixelsPerUnit, h / this.pixelsPerUnit);
  }

  /**
   * Cuts a sprite sheet into `columns × rows` equal cells, read left to right, top to
   * bottom (needs the texture loaded).
   */
  static grid(texture: Texture, columns: number, rows: number, pivot?: Vector2Like, pixelsPerUnit?: number): Sprite[] {
    const w = texture.width / columns;
    const h = texture.height / rows;
    const out: Sprite[] = [];
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < columns; c++) {
        out.push(Sprite.create(texture, new Rect(c * w, texture.height - (r + 1) * h, w, h), pivot, pixelsPerUnit));
      }
    }
    return out;
  }
}

/** Draws a `Sprite` at the object, facing its +z, tinted by `color` (Unity's `SpriteRenderer`). */
export class SpriteRenderer extends Component {
  private _sprite: Sprite | null = null;
  private _color = Color.white;
  private _flipX = false;
  private _flipY = false;
  private _sortingOrder = 0;
  private started = false;
  private shown = false;

  @property
  get sprite(): Sprite | null {
    return this._sprite;
  }
  set sprite(s: Sprite | null) {
    this._sprite = s;
    this.send();
  }
  @property
  get color(): Color {
    return this._color;
  }
  set color(c: Color) {
    this._color = c;
    this.send();
  }
  @property
  get flipX(): boolean {
    return this._flipX;
  }
  set flipX(f: boolean) {
    this._flipX = f;
    this.send();
  }
  @property
  get flipY(): boolean {
    return this._flipY;
  }
  set flipY(f: boolean) {
    this._flipY = f;
    this.send();
  }

  /** Draw order: higher draws over lower, whatever the distance (Unity's `sortingOrder`). */
  @property
  get sortingOrder(): number {
    return this._sortingOrder;
  }
  set sortingOrder(o: number) {
    this._sortingOrder = Math.trunc(o);
    this.send();
  }

  private send(): void {
    if (!this.started) return;
    const s = this._sprite;
    if (!s) {
      if (this.shown) __iron_op("render.remove", this.id);
      this.shown = false;
      return;
    }
    const c = this._color;
    const r = s.rect;
    __iron_op(
      "render.sprite",
      this.id,
      this.gameObject.id,
      s.texture.id,
      r ? [r.x, r.y, r.width, r.height] : null,
      [s.pivot.x, s.pivot.y],
      s.pixelsPerUnit,
      [c.r, c.g, c.b, c.a],
      this._flipX,
      this._flipY,
      this._sortingOrder,
    );
    this.shown = true;
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
  }
}

/** Forgets every texture. Only for tests. */
export function _resetTexturesForTests() {
  textures.clear();
  nextTexture = 1;
}
