// What the `Words` test API of tests/lang/lang3.rs does, for bun.
var Words = {
  join: (words, sep) => words.join(sep),
  count: (words) => words.filter((w) => w !== "").length,
  longest: (words) => words.reduce((a, w) => (w.length > a.length ? w : a), ""),
};
