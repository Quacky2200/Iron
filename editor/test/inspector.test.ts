// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector's logic (editor/src/inspector.ts).

import { componentAt, formatNumber, labelOf, parseNumber, placeOf, vectorJson, withComponent } from "../src/inspector.ts";
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

export function run(): string {
  return report();
}
