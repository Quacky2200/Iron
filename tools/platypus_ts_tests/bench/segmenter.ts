// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Intl.Segmenter: graphemes, words, sentences. Every exported function without parameters returns
// a string; `gen_segmenter_expected.ts` runs this file in bun.

function parts(text: string, seg: Intl.Segmenter): string {
  const out: string[] = [];
  for (const s of seg.segment(text)) out.push(s.segment + "@" + s.index);
  return out.join("|");
}

export function graphemesAscii(): string {
  return parts("hello", new Intl.Segmenter("en", { granularity: "grapheme" }));
}

export function graphemesCombining(): string {
  return parts("e\u0301a\u0308b", new Intl.Segmenter("en", { granularity: "grapheme" }));
}

export function graphemesEmoji(): string {
  const text = "a\u{1F468}\u200D\u{1F469}\u200D\u{1F467}b\u{1F1E9}\u{1F1EA}\u{1F44D}\u{1F3FD}";
  const seg = new Intl.Segmenter("en", { granularity: "grapheme" });
  let n = 0;
  const lens: number[] = [];
  for (const s of seg.segment(text)) {
    n += 1;
    lens.push(s.segment.length);
  }
  return n + ":" + lens.join(",");
}

export function wordsLatin(): string {
  return parts("Hello, world! It's 3.14 o'clock.", new Intl.Segmenter("en", { granularity: "word" }));
}

export function wordLike(): string {
  const seg = new Intl.Segmenter("en", { granularity: "word" });
  const out: string[] = [];
  for (const s of seg.segment("Hi there, 42 friends!")) out.push(s.isWordLike ? "W" : "-");
  return out.join("");
}

export function sentences(): string {
  return parts("Hello there. How are you? I am fine! Dr. Smith went home.", new Intl.Segmenter("en", { granularity: "sentence" }));
}

export function containing(): string {
  const seg = new Intl.Segmenter("en", { granularity: "word" });
  const segs = seg.segment("Hello world");
  const a = segs.containing(1);
  const b = segs.containing(5);
  const c = segs.containing(99);
  return (a === undefined ? "none" : a.segment) + "," + (b === undefined ? "none" : b.segment) + "," + (c === undefined ? "none" : c.segment);
}

export function defaultGranularity(): string {
  const seg = new Intl.Segmenter();
  let n = 0;
  for (const s of seg.segment("abc")) n += s.segment.length;
  return String(n);
}

export function badGranularity(): string {
  try {
    new Intl.Segmenter("en", { granularity: "line" });
    return "no";
  } catch (e) {
    return (e as Error).name;
  }
}
