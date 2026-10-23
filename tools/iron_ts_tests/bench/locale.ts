// SPDX-License-Identifier: Apache-2.0

// Intl.Locale: parsing, parts, extension keywords, options, maximize and minimize. Every exported
// function without parameters returns a string; `gen_locale_expected.ts` runs this file in bun.

function show(v: string | undefined): string {
  return v === undefined ? "-" : v;
}

export function parts(): string {
  const l = new Intl.Locale("en-Latn-US");
  return [l.language, show(l.script), show(l.region), l.baseName, l.toString()].join(",");
}

export function canonicalisation(): string {
  const a = new Intl.Locale("EN-us");
  const b = new Intl.Locale("zh-hans-cn");
  const c = new Intl.Locale("sr-latn");
  return [a.toString(), b.toString(), c.toString(), show(c.region)].join(",");
}

export function extensions(): string {
  const l = new Intl.Locale("en-US-u-ca-buddhist-hc-h23-nu-latn-co-phonebk-kf-upper-kn");
  return [show(l.calendar), show(l.hourCycle), show(l.numberingSystem), show(l.collation), show(l.caseFirst), String(l.numeric), l.baseName].join(",");
}

export function options(): string {
  const l = new Intl.Locale("en", { region: "GB", calendar: "gregory", hourCycle: "h12" });
  return [l.toString(), show(l.region), show(l.calendar), show(l.hourCycle), l.baseName].join(",");
}

export function maximize(): string {
  return [new Intl.Locale("en").maximize().toString(), new Intl.Locale("zh").maximize().toString(), new Intl.Locale("sr").maximize().toString(), new Intl.Locale("ja-JP").maximize().toString(), new Intl.Locale("und").maximize().toString()].join(",");
}

export function minimize(): string {
  return [new Intl.Locale("en-Latn-US").minimize().toString(), new Intl.Locale("zh-Hans-CN").minimize().toString(), new Intl.Locale("zh-Hant-TW").minimize().toString(), new Intl.Locale("en-GB").minimize().toString()].join(",");
}

export function keepsExtensions(): string {
  return new Intl.Locale("en-u-ca-hebrew").maximize().toString() + "," + new Intl.Locale("en-Latn-US-u-nu-arab").minimize().toString();
}

export function invalid(): string {
  const out: string[] = [];
  for (const t of ["", "e", "en_US", "toolonglanguage", "en-"]) {
    try {
      new Intl.Locale(t);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  return out.join(",");
}
