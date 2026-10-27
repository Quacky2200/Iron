// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// PlayerPrefs: numbers and strings by key, with and without fallbacks, compiled against the native
// API and run in bun against js/engine/src/prefs.ts. The saved file had "saved.n" = 7.9 and
// "saved.s" = "hello".

export function reads(): string {
  const n1 = PlayerPrefs.getInt("saved.n");
  const n2 = PlayerPrefs.getFloat("saved.n");
  const s1 = PlayerPrefs.getString("saved.s");
  const m1 = PlayerPrefs.getInt("missing");
  const m2 = PlayerPrefs.getInt("missing", 5);
  const m3 = PlayerPrefs.getFloat("missing", 1.5);
  const m4 = PlayerPrefs.getString("missing", "dflt");
  const w1 = PlayerPrefs.getString("saved.n", "wrong type");
  const w2 = PlayerPrefs.getInt("saved.s", 3);
  return `${n1},${n2},${s1},${m1},${m2},${m3},${m4},${w1},${w2}`;
}

export function writes(): string {
  PlayerPrefs.setInt("a", 3.9);
  PlayerPrefs.setFloat("b", 2.5);
  PlayerPrefs.setString("c", "text");
  PlayerPrefs.setInt("neg", -2.7);
  const s = `${PlayerPrefs.getInt("a")},${PlayerPrefs.getFloat("b")},${PlayerPrefs.getString("c")},${PlayerPrefs.getInt("neg")},${PlayerPrefs.hasKey("a")},${PlayerPrefs.hasKey("zzz")}`;
  PlayerPrefs.deleteKey("a");
  const after = `${PlayerPrefs.hasKey("a")},${PlayerPrefs.hasKey("b")}`;
  PlayerPrefs.save();
  PlayerPrefs.deleteAll();
  return `${s};${after};${PlayerPrefs.hasKey("b")}${PlayerPrefs.hasKey("saved.n")}`;
}
