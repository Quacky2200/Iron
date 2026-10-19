// ArrayBuffer, typed arrays over a buffer, subarray, .buffer and DataView. Every exported function
// without parameters returns a string; `gen_buffers_expected.ts` runs this file in bun.

export function viewsShareMemory(): string {
  const buf = new ArrayBuffer(16);
  const bytes = new Uint8Array(buf);
  const words = new Uint32Array(buf);
  const floats = new Float32Array(buf, 8, 2);
  bytes[0] = 1;
  bytes[1] = 2;
  words[1] = 0x01020304;
  floats[0] = 1.5;
  return [buf.byteLength, bytes.length, words.length, floats.length, bytes[0] + bytes[1] * 256, bytes[4], bytes[7], words[2]].join(",");
}

export function offsetsAndLengths(): string {
  const buf = new ArrayBuffer(24);
  const a = new Int16Array(buf, 4, 6);
  const b = new Int16Array(buf, 8);
  const c = new Uint8Array(buf, 20);
  return [a.length, a.byteOffset, a.byteLength, b.length, b.byteOffset, c.length, c.byteOffset].join(",");
}

export function viewErrors(): string {
  const buf = new ArrayBuffer(10);
  const out: string[] = [];
  try {
    new Int32Array(buf, 1);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    new Int32Array(buf);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    new Uint8Array(buf, 4, 8);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    new Uint8Array(buf, 11);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function subarrayShares(): string {
  const a = new Int32Array([1, 2, 3, 4, 5, 6]);
  const s = a.subarray(2, 5);
  s[0] = 30;
  s[2] = 50;
  const t = s.subarray(1);
  t[0] = 40;
  const u = a.subarray(-2);
  return a.join("-") + "|" + [s.length, s.byteOffset, t.length, t.byteOffset].join(",") + "|" + u.join("-");
}

export function bufferOfAnArray(): string {
  const a = new Float64Array([1, 2, 3]);
  const buf = a.buffer;
  const bytes = new Uint8Array(buf);
  bytes[7] = 0x40;
  bytes[6] = 0xf0;
  return [buf.byteLength, bytes.length, a[0], a.byteOffset].join(",");
}

export function bufferOfASubarray(): string {
  const a = new Uint16Array([10, 20, 30, 40]);
  const s = a.subarray(1, 3);
  const buf = s.buffer;
  s[0] = 99;
  return [buf.byteLength, s.byteOffset, s.byteLength, a[1], new Uint16Array(buf)[1]].join(",");
}

export function sliceOfABuffer(): string {
  const buf = new ArrayBuffer(8);
  const bytes = new Uint8Array(buf);
  for (let i = 0; i < 8; i++) bytes[i] = i * 3;
  const part = buf.slice(2, 6);
  const copy = new Uint8Array(part);
  copy[0] = 255;
  const tail = buf.slice(-3);
  return part.byteLength + "|" + copy.join("-") + "|" + bytes[2] + "|" + tail.byteLength + "|" + new Uint8Array(tail).join("-");
}

export function dataViewIntegers(): string {
  const buf = new ArrayBuffer(16);
  const dv = new DataView(buf);
  dv.setUint8(0, 200);
  dv.setInt8(1, -5);
  dv.setUint16(2, 0xabcd);
  dv.setUint16(4, 0xabcd, true);
  dv.setInt32(8, -123456789);
  dv.setUint32(12, 0xdeadbeef, true);
  const b = new Uint8Array(buf);
  return [
    dv.getUint8(0), dv.getInt8(1), dv.getUint8(1), dv.getUint16(2), dv.getUint16(2, true), dv.getInt16(4), dv.getUint16(4, true),
    dv.getInt32(8), dv.getUint32(8), dv.getUint32(12, true), dv.getInt32(12), b[2], b[3], b[4], b[5],
  ].join(",");
}

export function dataViewFloats(): string {
  const dv = new DataView(new ArrayBuffer(24));
  dv.setFloat32(0, 1.5);
  dv.setFloat32(4, -0.1, true);
  dv.setFloat64(8, Math.PI);
  dv.setFloat64(16, 1e300, true);
  return [dv.getFloat32(0), dv.getFloat32(4, true), dv.getFloat64(8), dv.getFloat64(16, true), dv.getUint8(0), dv.getUint8(3)].join(",");
}

export function dataViewWindow(): string {
  const buf = new ArrayBuffer(12);
  new Uint8Array(buf).set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
  const dv = new DataView(buf, 4, 6);
  const out: string[] = [dv.byteLength + "", dv.byteOffset + "", dv.getUint8(0) + "", dv.getUint32(1) + "", (dv.buffer === buf) + ""];
  try {
    dv.getUint32(3);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    dv.setUint8(6, 1);
    out.push("no");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function viewsSurviveCollection(): string {
  let total = 0;
  const keep: Float32Array[] = [];
  for (let i = 0; i < 200; i++) {
    const buf = new ArrayBuffer(64);
    const f = new Float32Array(buf, 16, 4);
    f[0] = i;
    f[3] = i * 2;
    if (i % 20 === 0) keep.push(f);
    const junk = new Uint8Array(1000 + i);
    junk[0] = 1;
    total += f[0] + f[3];
  }
  let kept = 0;
  for (const f of keep) kept += f[0] + f[3] + f.byteOffset;
  return total + "," + keep.length + "," + kept;
}
