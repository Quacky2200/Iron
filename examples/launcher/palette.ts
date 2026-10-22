// Build-time colours for the launcher's backgrounds: a screenshot's top and bottom tones and
// its most vivid colour, read straight from the PNG (8-bit RGB / RGBA, not interlaced, as
// Iron's screenshots are), so each game gets a gradient that matches its picture. White text
// sits on top, so every colour keeps a hue and stays well below white: a grey or white
// picture borrows its accent's hue, and bright pictures show fainter behind the UI.
import { inflateSync } from "node:zlib";

export interface Palette {
  /** The gradient's top and bottom, dark enough for white text. */
  top: string;
  bottom: string;
  /** A glow in the picture's most vivid colour, as `r, g, b`. */
  glow: string;
  /** How opaque the picture itself shows behind the UI: lower for bright pictures. */
  picture: number;
}

type Rgb = [number, number, number];

interface Png {
  width: number;
  height: number;
  channels: number;
  data: Uint8Array;
}

/** Decodes the PNGs Iron writes; `null` for anything else. */
function decode(buf: Buffer): Png | null {
  if (buf.readUInt32BE(0) !== 0x89504e47) return null;
  let at = 8;
  let width = 0;
  let height = 0;
  let channels = 0;
  let ok = false;
  const idat: Buffer[] = [];
  while (at + 8 <= buf.length) {
    const len = buf.readUInt32BE(at);
    const type = buf.toString("ascii", at + 4, at + 8);
    const body = buf.subarray(at + 8, at + 8 + len);
    if (type === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      const [depth, color, , , interlace] = [body[8], body[9], body[10], body[11], body[12]];
      channels = color === 6 ? 4 : color === 2 ? 3 : 0;
      ok = depth === 8 && channels > 0 && interlace === 0;
    } else if (type === "IDAT") idat.push(body);
    else if (type === "IEND") break;
    at += 12 + len;
  }
  if (!ok) return null;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const data = new Uint8Array(height * stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]!;
    const line = y * (stride + 1) + 1;
    const row = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? data[row + x - channels]! : 0;
      const b = y > 0 ? data[row - stride + x]! : 0;
      const c = x >= channels && y > 0 ? data[row - stride + x - channels]! : 0;
      let v = raw[line + x]!;
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const [pa, pb, pc] = [Math.abs(p - a), Math.abs(p - b), Math.abs(p - c)];
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      data[row + x] = v & 255;
    }
  }
  return { width, height, channels, data };
}

/** Average colour of rows `from`..`to` (fractions of the height), sampling every 6 px. */
function average(png: Png, from: number, to: number): Rgb {
  const sum: Rgb = [0, 0, 0];
  let n = 0;
  for (let y = Math.floor(png.height * from); y < Math.floor(png.height * to); y += 6) {
    for (let x = 0; x < png.width; x += 6) {
      const i = (y * png.width + x) * png.channels;
      sum[0] += png.data[i]!;
      sum[1] += png.data[i + 1]!;
      sum[2] += png.data[i + 2]!;
      n++;
    }
  }
  return n ? [sum[0] / n, sum[1] / n, sum[2] / n] : [20, 24, 32];
}

function hsv([r, g, b]: Rgb): [number, number, number] {
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  const h = d === 0 ? 0 : max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, max ? d / max : 0, max / 255];
}

/** The most vivid colour: hues in 12 bins weighted by saturation × brightness. */
function vivid(png: Png): Rgb {
  const bins = Array.from({ length: 12 }, () => ({ weight: 0, sum: [0, 0, 0] as Rgb }));
  for (let y = 0; y < png.height; y += 4) {
    for (let x = 0; x < png.width; x += 4) {
      const i = (y * png.width + x) * png.channels;
      const c: Rgb = [png.data[i]!, png.data[i + 1]!, png.data[i + 2]!];
      const [h, s, v] = hsv(c);
      if (s < 0.25 || v < 0.2) continue;
      const bin = bins[Math.floor(h / 30) % 12]!;
      const w = s * v;
      bin.weight += w;
      for (let k = 0; k < 3; k++) bin.sum[k]! += c[k]! * w;
    }
  }
  const best = bins.reduce((a, b) => (b.weight > a.weight ? b : a));
  if (best.weight === 0) return [124, 196, 255];
  return best.sum.map((v) => v / best.weight) as Rgb;
}

function rgb(h: number, s: number, v: number): Rgb {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return 255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1)));
  };
  return [f(5), f(3), f(1)];
}

const hex = (c: Rgb) => `#${c.map((n) => Math.round(n).toString(16).padStart(2, "0")).join("")}`;

/**
 * `c` as a dark backdrop tone: brightness capped at `max` (0..1), saturation lifted, and a
 * grey (a white or washed-out picture) given `accent`'s hue instead, so it never reads as
 * a flat grey close to the text.
 */
function deepen(c: Rgb, max: number, accent: Rgb): string {
  let [h, s, v] = hsv(c);
  if (s < 0.18) {
    h = hsv(accent)[0];
    s = 0.4;
  }
  return hex(rgb(h, Math.min(1, Math.max(0.3, s * 1.25 + 0.08)), Math.min(v, max)));
}

/** The accent, kept saturated and off white so its glow tints rather than brightens. */
function glowOf(c: Rgb): string {
  const [h, s, v] = hsv(c);
  return rgb(h, Math.max(s, 0.55), Math.min(v, 0.85)).map(Math.round).join(", ");
}

/** Mean luma, 0..1. */
function luma(png: Png): number {
  const [r, g, b] = average(png, 0, 1);
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

/** Colours for a screenshot, or `null` when it can't be read. */
export function palette(file: Buffer): Palette | null {
  const png = decode(file);
  if (!png) return null;
  const accent = vivid(png);
  return {
    top: deepen(average(png, 0, 0.4), 0.3, accent),
    bottom: deepen(average(png, 0.6, 1), 0.18, accent),
    glow: glowOf(accent),
    // 0.4 for a dark picture, down to 0.14 for a white one.
    picture: Math.round((0.4 - 0.26 * Math.min(1, Math.max(0, (luma(png) - 0.25) / 0.6))) * 100) / 100,
  };
}

/** A palette from the name's hash, for games without a screenshot. */
export function fallback(name: string): Palette {
  let hue = 0;
  for (const c of name) hue = (hue * 31 + c.charCodeAt(0)) % 360;
  const accent = rgb(hue, 0.6, 1);
  return {
    top: deepen(rgb(hue, 0.6, 0.5), 0.3, accent),
    bottom: deepen(rgb(hue + 40, 0.6, 0.3), 0.18, accent),
    glow: glowOf(accent),
    picture: 0.4,
  };
}
