// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector's logic (editor/src/inspector.ts).

import {
  changedParts,
  colorHex,
  componentAt,
  componentJson,
  fitsAsset,
  formatNumber,
  hsv,
  labelOf,
  mixed,
  mixedParts,
  palette,
  parseHex,
  parseNumber,
  placeOf,
  sharedComponents,
  soundJson,
  vectorJson,
  withComponent,
  withParts,
} from "../src/inspector.ts";
import { eq, report, same, test, yes } from "./check.ts";

// M5-EDT-37
test("numbers are read as typed and shown short", () => {
  eq(parseNumber("1.5"), 1.5, "decimal");
  eq(parseNumber(" -2 "), -2, "negative, spaces");
  eq(parseNumber("2e3"), 2000, "exponent");
  eq(parseNumber(".5"), 0.5, "leading dot");
  yes(Number.isNaN(parseNumber("")), "empty");
  yes(Number.isNaN(parseNumber("1,5")), "a comma");
  yes(Number.isNaN(parseNumber("abc")), "letters");
  same(formatNumber(1.23456789), "1.2346", "four decimals");
  same(formatNumber(2), "2", "no trailing zeros");
  same(formatNumber(-0.00001), "0", "no -0");
});

// M5-EDT-37
test("field names become labels; vectors change one part", () => {
  same(labelOf("fieldOfView"), "Field Of View", "camel case");
  same(labelOf("spot_angle"), "Spot Angle", "snake case");
  same(labelOf("x"), "X", "one letter");
  eq(withComponent([1, 2, 3], 1, 9), [1, 9, 3], "y set");
  same(vectorJson([1, 0.5, 0]), "[1,0.5,0]", "as JSON");
});

// M5-EDT-37
test("a component is found again by kind and place", () => {
  const list: ComponentInfo[] = [
    { kind: "Light", handle: 4, opaque: false },
    { kind: "AudioSource", handle: 7, opaque: false },
    { kind: "AudioSource", handle: 9, opaque: false },
  ];
  eq(placeOf(list, "AudioSource", 9), 1, "the second AudioSource");
  eq(placeOf(list, "Light", 4), 0, "the Light");
  eq(placeOf(list, "Light", 5), -1, "not there");
  const rebuilt: ComponentInfo[] = [
    { kind: "Light", handle: 40, opaque: false },
    { kind: "AudioSource", handle: 70, opaque: false },
    { kind: "AudioSource", handle: 90, opaque: false },
  ];
  eq(componentAt(rebuilt, "AudioSource", 1)?.handle ?? -1, 90, "after a rebuild");
  yes(componentAt(rebuilt, "Camera", 0) === null, "none");
});

// M5-EDT-49
test("several objects: shared components, mixed values, edited vector parts", () => {
  const a: ComponentInfo[] = [
    { kind: "MeshRenderer", handle: 1, opaque: false },
    { kind: "Light", handle: 2, opaque: false },
    { kind: "Light", handle: 3, opaque: false },
  ];
  const b: ComponentInfo[] = [
    { kind: "Light", handle: 7, opaque: false },
    { kind: "MeshRenderer", handle: 8, opaque: false },
  ];
  eq(sharedComponents([a, b]), [{ kind: "MeshRenderer", place: 0 }, { kind: "Light", place: 0 }], "on both, by place");
  eq(sharedComponents([a]), [{ kind: "MeshRenderer", place: 0 }, { kind: "Light", place: 0 }, { kind: "Light", place: 1 }], "one object");
  eq(sharedComponents([]), [], "none");
  yes(mixed(["1", "2"]) && !mixed(["1", "1"]) && !mixed([]), "mixed");
  eq(mixedParts([[1, 2, 3], [1, 5, 3]]), [false, true, false], "parts");
  eq(changedParts([1, 2, 3], [1, 9, 3]), [1], "changed");
  eq(withParts([4, 5, 6], [1, 9, 3], [1]), [4, 9, 6], "own parts kept");
});

// M5-EDT-49
test("references, assets and colours as fields take them", () => {
  same(componentJson(null), "null", "no component");
  same(componentJson({ kind: "Rigidbody", handle: 4, opaque: false }), `{"$ref":"Component","kind":"Rigidbody","handle":4}`, "component");
  same(soundJson("sfx/a.wav"), `{"$type":"Sound","src":"sfx/a.wav"}`, "sound");
  yes(fitsAsset("Texture", "art/a.PNG") && !fitsAsset("Texture", "a.wav") && !fitsAsset("Mesh", "a.png"), "files by kind");
  same(colorHex([1, 0.5, 0, 1]), "#ff8000", "hex");
  eq(parseHex("#f80", 0.5), [1, 0.5333333333333333, 0, 0.5], "short hex");
  yes(parseHex("nope", 1) === null, "not hex");
  eq(hsv(120, 1, 1), [0, 1, 0], "green");
  eq(palette().length, 7, "six tones and greys");
  eq(palette()[6]![0], [1, 1, 1], "white first");
});

export function run(): string {
  return report();
}
