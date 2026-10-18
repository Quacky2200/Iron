// Intl, step I1: localeCompare, Intl.Collator, normalize, toLocaleUpperCase/LowerCase. Every
// exported function without parameters returns a string; `gen_intl_expected.ts` runs this file in
// bun and `tests/intl.rs` compares.

function fail(f: () => number): string {
  try {
    return String(f());
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function localeCompareBasics(): string {
  return [
    "a".localeCompare("B"), "a".localeCompare("A"), "A".localeCompare("a"), "a".localeCompare("a"), "ä".localeCompare("z", "de"),
    "ä".localeCompare("z", "sv"), "résumé".localeCompare("resume"), "z".localeCompare("a"), "".localeCompare("a"), "a".localeCompare(""),
    "a".localeCompare("b", ["xx-XX", "de"]), "ß".localeCompare("ss", "de"), "ö".localeCompare("p", "tr"), "I".localeCompare("ı", "tr"),
  ].join(",");
}

export function localeCompareOptions(): string {
  return [
    "a".localeCompare("á", undefined, { sensitivity: "base" }), "a".localeCompare("á", "en", { sensitivity: "accent" }),
    "a".localeCompare("A", "en", { sensitivity: "case" }), "a".localeCompare("A", "en", { sensitivity: "base" }),
    "a".localeCompare("á", "en", { sensitivity: "case" }), "a".localeCompare("A", "en", { sensitivity: "variant" }),
    "a10".localeCompare("a2", undefined, { numeric: true }), "a10".localeCompare("a2", undefined, { numeric: false }),
    "a".localeCompare("A", "en", { caseFirst: "upper" }), "a".localeCompare("A", "en", { caseFirst: "lower" }),
    "a-b".localeCompare("ab", "en", { ignorePunctuation: true }), "a-b".localeCompare("ab", "en", { ignorePunctuation: false }),
    "a".localeCompare("b", "en", { usage: "search" }),
  ].join(",");
}

export function localeCompareErrors(): string {
  return [
    fail(() => "a".localeCompare("b", "xx-invalid-tag!")),
    fail(() => "a".localeCompare("b", "en", { sensitivity: "foo" })),
    fail(() => "a".localeCompare("b", "en", { caseFirst: "middle" })),
    fail(() => "a".localeCompare("b", "en", { usage: "other" })),
    fail(() => "a".localeCompare("b", ["en", "bad tag"])),
  ].join("\n");
}

export function normalizeForms(): string {
  const s = "ẛ̣";
  const parts = [s.normalize("NFC"), s.normalize("NFD"), s.normalize("NFKC"), s.normalize("NFKD"), s.normalize(), "abc".normalize("NFC")];
  let out = "";
  for (const p of parts) {
    out += p.length + ":";
    for (let i = 0; i < p.length; i++) out += p.charCodeAt(i).toString(16) + ".";
    out += "|";
  }
  const lone = "\uD800x́".normalize("NFC");
  out += lone.length + ":" + lone.charCodeAt(0) + "," + lone.charCodeAt(1) + "," + lone.charCodeAt(2);
  out += "|" + "é".normalize("NFD").length + "," + "é".normalize("NFC").length + "," + "ﬁ".normalize("NFKC") + "," + "Å".normalize("NFD").normalize("NFC").length;
  out += "|" + "한".normalize("NFD").length + "," + "한".normalize("NFC").length;
  return out;
}

export function normalizeErrors(): string {
  return [fail(() => "abc".normalize("nfc").length), fail(() => "abc".normalize("NFX").length), fail(() => "abc".normalize("").length)].join("\n");
}

export function localeCase(): string {
  return [
    "istanbul".toLocaleUpperCase("tr"), "İ".toLocaleLowerCase("tr"), "ß".toLocaleUpperCase(), "I".toLocaleLowerCase("lt"), "ǆ".toLocaleUpperCase("en"),
    "i".toLocaleUpperCase("az"), "I".toLocaleLowerCase("tr"), "ΑΣ".toLocaleLowerCase(), "ΑΣ Σ".toLocaleLowerCase("el"), "straße".toLocaleUpperCase("de"),
    "ŉ".toLocaleUpperCase(), "ǰ".toLocaleUpperCase("en"), "Ω".toLocaleLowerCase(), "ÀÉÎÕÜ".toLocaleLowerCase("fr"),
  ].join(",");
}

export function collatorSorts(): string {
  const en = new Intl.Collator("en");
  const de = new Intl.Collator("de", { numeric: true });
  const sv = new Intl.Collator("sv");
  const words = ["b10", "b9", "ä", "a", "Z", "z", "résumé", "resume", "Resume", "résume", "_", "-", "1", " ", "aa", "ZZ", "α", "Привет", "한국", "あ", "ア", "大阪", "東京"];
  return [
    words.slice().sort(en.compare).join(","),
    words.slice().sort(de.compare).join(","),
    ["ä", "a", "z", "å", "ö", "o"].sort(sv.compare).join(","),
    ["ä", "a", "z", "å", "ö", "o"].sort(new Intl.Collator("de").compare).join(","),
    ["東京", "大阪", "京都"].sort(new Intl.Collator("ja").compare).join(","),
    ["東京", "大阪", "京都"].sort(new Intl.Collator("zh").compare).join(","),
    ["b", "a", "C", "A", "B", "c"].sort(new Intl.Collator("en", { caseFirst: "upper" }).compare).join(","),
    ["b", "a", "C", "A", "B", "c"].sort(new Intl.Collator("en", { sensitivity: "base" }).compare).join(","),
  ].join("\n");
}

export function collatorCompare(): string {
  const c = new Intl.Collator("en", { sensitivity: "base" });
  const d = new Intl.Collator();
  return [c.compare("a", "A"), c.compare("a", "b"), c.compare("b", "a"), d.compare("a", "B"), d.compare("a", "a")].join(",");
}

export function collatorErrors(): string {
  return [
    fail(() => new Intl.Collator("not a tag").compare("a", "b")),
    fail(() => new Intl.Collator("en", { sensitivity: "huge" }).compare("a", "b")),
  ].join("\n");
}
