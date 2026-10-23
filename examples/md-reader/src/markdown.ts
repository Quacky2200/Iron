// SPDX-License-Identifier: Apache-2.0

// A small Markdown parser: the subset Iron's docs use (CommonMark blocks plus GitHub
// tables, task lists and strikethrough). It makes a plain tree the reader draws.

export type Inline =
  | { kind: "text"; text: string }
  | { kind: "strong" | "em" | "del"; children: Inline[] }
  | { kind: "code"; text: string }
  | { kind: "link"; href: string; children: Inline[] }
  | { kind: "image"; src: string; alt: string }
  | { kind: "br" };

export type Align = "left" | "center" | "right" | null;

export type Block = BlockKind & {
  /** The source line it starts on (0-based), for `#L<n>` links. */
  line?: number;
};

type BlockKind =
  | { kind: "heading"; level: number; id: string; children: Inline[] }
  | { kind: "paragraph"; children: Inline[] }
  | { kind: "code"; lang: string; text: string }
  | { kind: "list"; ordered: boolean; start: number; items: ListItem[] }
  | { kind: "quote"; children: Block[] }
  | { kind: "table"; align: Align[]; head: Inline[][]; rows: Inline[][][] }
  | { kind: "rule" };

export interface ListItem {
  /** The source line the item starts on (0-based). */
  line: number;
  /** `[ ]` / `[x]` task items; `null` for plain ones. */
  checked: boolean | null;
  children: Block[];
}

export interface Doc {
  blocks: Block[];
  /** Headings in order, for the outline and `#anchor` links. */
  headings: { level: number; id: string; text: string }[];
}

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([^`\s]*)/;
const HEADING = /^ {0,3}(#{1,6})(?:\s+(.*?))?\s*#*\s*$/;
const RULE = /^ {0,3}([-*_])(?:\s*\1){2,}\s*$/;
const BULLET = /^(\s*)([-*+]|\d{1,9}[.)])(\s+|$)/;
const QUOTE = /^ {0,3}> ?/;
const TABLE_RULE = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;

export function parse(source: string): Doc {
  const headings: Doc["headings"] = [];
  const used = new Map<string, number>();
  const blocks = parseBlocks(source.split("\n"), (level, children) => {
    const text = plainText(children);
    let id = slug(text);
    const n = used.get(id) ?? 0;
    used.set(id, n + 1);
    if (n > 0) id = `${id}-${n}`;
    headings.push({ level, id, text });
    return id;
  });
  return { blocks, headings };
}

/** GitHub's heading anchors: lower case, punctuation dropped, spaces to dashes. */
export function slug(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .replace(/\s/g, "-");
}

export function plainText(inlines: Inline[]): string {
  return inlines
    .map((i) => {
      switch (i.kind) {
        case "text":
        case "code":
          return i.text;
        case "image":
          return i.alt;
        case "br":
          return " ";
        default:
          return plainText(i.children);
      }
    })
    .join("");
}

type HeadingId = (level: number, children: Inline[]) => string;

const startsBlock = (line: string) =>
  FENCE.test(line) || HEADING.test(line) || RULE.test(line) || QUOTE.test(line) || BULLET.test(line);

/** Blocks from `lines`, which start at source line `base`. */
function parseBlocks(lines: string[], heading: HeadingId, base = 0): Block[] {
  const out: Block[] = [];
  let i = 0;
  // Blocks pushed since `at` started at source line `from`.
  let at = 0;
  let from = 0;
  const locate = () => {
    for (; at < out.length; at++) out[at]!.line ??= base + from;
    from = i;
  };
  while (i < lines.length) {
    locate();
    const line = lines[i]!;
    if (!line.trim()) {
      i++;
      continue;
    }
    // HTML comments (the docs use them for notes) are skipped.
    if (line.trimStart().startsWith("<!--")) {
      while (i < lines.length && !lines[i]!.includes("-->")) i++;
      i++;
      continue;
    }
    const fence = FENCE.exec(line);
    if (fence) {
      const marker = fence[1]!;
      const indent = line.length - line.trimStart().length;
      const body: string[] = [];
      i++;
      while (i < lines.length && !lines[i]!.trimStart().startsWith(marker)) {
        body.push(lines[i]!.replace(new RegExp(`^ {0,${indent}}`), ""));
        i++;
      }
      i++;
      out.push({ kind: "code", lang: fence[2] ?? "", text: body.join("\n") });
      continue;
    }
    const h = HEADING.exec(line);
    if (h) {
      const level = h[1]!.length;
      const children = parseInline(h[2] ?? "");
      out.push({ kind: "heading", level, id: heading(level, children), children });
      i++;
      continue;
    }
    if (RULE.test(line)) {
      out.push({ kind: "rule" });
      i++;
      continue;
    }
    if (QUOTE.test(line)) {
      const body: string[] = [];
      const first = i;
      while (i < lines.length && lines[i]!.trim() && (QUOTE.test(lines[i]!) || !startsBlock(lines[i]!))) {
        body.push(lines[i]!.replace(QUOTE, ""));
        i++;
      }
      out.push({ kind: "quote", children: parseBlocks(body, heading, base + first) });
      continue;
    }
    if (BULLET.test(line)) {
      i = parseList(lines, i, heading, out, base);
      continue;
    }
    if (line.includes("|") && i + 1 < lines.length && TABLE_RULE.test(lines[i + 1]!)) {
      const align = cells(lines[i + 1]!).map((c): Align => {
        const t = c.trim();
        if (t.startsWith(":") && t.endsWith(":")) return "center";
        if (t.endsWith(":")) return "right";
        return t.startsWith(":") ? "left" : null;
      });
      const head = cells(line).map(parseInline);
      const rows: Inline[][][] = [];
      i += 2;
      while (i < lines.length && lines[i]!.includes("|") && lines[i]!.trim()) {
        rows.push(cells(lines[i]!).map(parseInline));
        i++;
      }
      out.push({ kind: "table", align, head, rows });
      continue;
    }
    // A paragraph runs to a blank line or the start of another block.
    const body: string[] = [];
    while (i < lines.length && lines[i]!.trim() && !(body.length && startsBlock(lines[i]!))) {
      body.push(lines[i]!);
      i++;
    }
    out.push({ kind: "paragraph", children: parseInline(joinLines(body)) });
  }
  locate();
  return out;
}

/** Paragraph lines joined with spaces; two trailing spaces or a backslash break the line. */
function joinLines(lines: string[]): string {
  return lines
    .map((l, n) => {
      const last = n === lines.length - 1;
      const text = l.trim();
      if (!last && (/ {2,}$/.test(l) || text.endsWith("\\"))) return `${text.replace(/\\$/, "")}\u0000`;
      return text;
    })
    .join(" ")
    .replace(/\u0000 /g, "\u0000");
}

function parseList(lines: string[], start: number, heading: HeadingId, out: Block[], base: number): number {
  const first = BULLET.exec(lines[start]!)!;
  const ordered = /\d/.test(first[2]!);
  const indent = first[1]!.length;
  const items: ListItem[] = [];
  let i = start;
  while (i < lines.length) {
    const m = BULLET.exec(lines[i]!);
    if (!m || m[1]!.length !== indent || /\d/.test(m[2]!) !== ordered) break;
    // The item's content column: continuation lines indented this far belong to it.
    const column = m[0].length;
    const itemLine = i;
    const body = [lines[i]!.slice(column)];
    i++;
    while (i < lines.length) {
      const l = lines[i]!;
      const lead = l.length - l.trimStart().length;
      if (!l.trim()) {
        // A blank line ends the item unless indented content follows.
        const next = lines[nextFilled(lines, i + 1)];
        if (next === undefined || next.length - next.trimStart().length <= indent) break;
        body.push("");
        i++;
        continue;
      }
      if (lead > indent) body.push(l.slice(Math.min(lead, column)));
      else if (!startsBlock(l) && body[body.length - 1]!.trim()) body.push(l.trim());
      else break;
      i++;
    }
    let checked: boolean | null = null;
    const task = /^\[([ xX])\]\s+/.exec(body[0]!);
    if (task) {
      checked = task[1] !== " ";
      body[0] = body[0]!.slice(task[0].length);
    }
    items.push({ line: base + itemLine, checked, children: parseBlocks(body, heading, base + itemLine) });
    // Blank lines between items.
    while (i < lines.length && !lines[i]!.trim()) {
      const next = lines[nextFilled(lines, i)];
      const n = next && BULLET.exec(next);
      if (!n || n[1]!.length !== indent) return finish();
      i++;
    }
  }
  return finish();

  function finish() {
    out.push({ kind: "list", ordered, start: ordered ? parseInt(first[2]!, 10) : 1, items });
    return i;
  }
}

/** The index of the first non-blank line from `from` on (`lines.length` if none). */
function nextFilled(lines: string[], from: number): number {
  let i = from;
  while (i < lines.length && !lines[i]!.trim()) i++;
  return i;
}

/** A table row's cells, ignoring `|` inside code spans or escaped. */
function cells(row: string): string[] {
  let s = row.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|") && !s.endsWith("\\|")) s = s.slice(0, -1);
  const out: string[] = [];
  let cell = "";
  let tick = 0;
  for (let i = 0; i < s.length; i++) {
    const c = s[i]!;
    if (c === "\\" && s[i + 1] === "|") {
      cell += "|";
      i++;
    } else if (c === "`") {
      let run = 1;
      while (s[i + run] === "`") run++;
      tick = tick === 0 ? run : tick === run ? 0 : tick;
      cell += "`".repeat(run);
      i += run - 1;
    } else if (c === "|" && tick === 0) {
      out.push(cell.trim());
      cell = "";
    } else cell += c;
  }
  out.push(cell.trim());
  return out;
}

const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', nbsp: " ", apos: "'" };

// Sticky patterns, matched at an index instead of on a copy of the rest of the line.
const PLAIN = /[^\\`&<![~*_h\u0000]+/y;
const ENTITY = /&(#x[0-9a-f]+|#\d+|[a-z]+);/iy;
const AUTOLINK = /<((?:https?|mailto):[^>\s]+)>/y;
const BR = /<br\s*\/?>/iy;
const IMG = /<img\s[^>]*>/iy;
const URL = /https?:\/\/[^\s<>()]*[^\s<>().,;:!?'"*_]/y;

function matchAt(re: RegExp, src: string, i: number): RegExpExecArray | null {
  re.lastIndex = i;
  return re.exec(src);
}

export function parseInline(src: string): Inline[] {
  const out: Inline[] = [];
  let text = "";
  const flush = () => {
    if (text) out.push({ kind: "text", text });
    text = "";
  };
  let i = 0;
  while (i < src.length) {
    // Runs of ordinary characters in one step.
    const plain = matchAt(PLAIN, src, i);
    if (plain) {
      text += plain[0];
      i += plain[0].length;
      continue;
    }
    const c = src[i]!;
    if (c === "\u0000") {
      flush();
      out.push({ kind: "br" });
      i++;
      continue;
    }
    if (c === "\\" && i + 1 < src.length && /[!-/:-@[-`{-~]/.test(src[i + 1]!)) {
      text += src[i + 1];
      i += 2;
      continue;
    }
    if (c === "`") {
      let run = 1;
      while (src[i + run] === "`") run++;
      const close = src.indexOf("`".repeat(run), i + run);
      if (close > 0 && src[close + run] !== "`") {
        flush();
        let code = src.slice(i + run, close);
        if (/^ .* $/.test(code)) code = code.slice(1, -1);
        out.push({ kind: "code", text: code });
        i = close + run;
        continue;
      }
      text += "`".repeat(run);
      i += run;
      continue;
    }
    if (c === "&") {
      const m = matchAt(ENTITY, src, i);
      if (m) {
        const e = m[1]!;
        const ch = e.startsWith("#x")
          ? String.fromCodePoint(parseInt(e.slice(2), 16))
          : e.startsWith("#")
            ? String.fromCodePoint(parseInt(e.slice(1), 10))
            : ENTITIES[e.toLowerCase()];
        if (ch !== undefined) {
          text += ch;
          i += m[0].length;
          continue;
        }
      }
    }
    if (c === "<") {
      const auto = matchAt(AUTOLINK, src, i);
      if (auto) {
        flush();
        out.push({ kind: "link", href: auto[1]!, children: [{ kind: "text", text: auto[1]! }] });
        i += auto[0].length;
        continue;
      }
      if (src.startsWith("<!--", i)) {
        const end = src.indexOf("-->", i);
        i = end < 0 ? src.length : end + 3;
        continue;
      }
      const br = matchAt(BR, src, i);
      if (br) {
        flush();
        out.push({ kind: "br" });
        i += br[0].length;
        continue;
      }
      const img = matchAt(IMG, src, i);
      if (img) {
        flush();
        const attr = (name: string) => new RegExp(`${name}\\s*=\\s*["']([^"']*)["']`, "i").exec(img[0])?.[1] ?? "";
        out.push({ kind: "image", src: attr("src"), alt: attr("alt") });
        i += img[0].length;
        continue;
      }
    }
    if (c === "!" && src[i + 1] === "[") {
      const link = linkAt(src, i + 1);
      if (link) {
        flush();
        out.push({ kind: "image", src: link.href, alt: link.label });
        i = link.end;
        continue;
      }
    }
    if (c === "[") {
      const link = linkAt(src, i);
      if (link) {
        flush();
        out.push({ kind: "link", href: link.href, children: parseInline(link.label) });
        i = link.end;
        continue;
      }
    }
    if (c === "~" && src[i + 1] === "~") {
      const close = src.indexOf("~~", i + 2);
      if (close > i + 2) {
        flush();
        out.push({ kind: "del", children: parseInline(src.slice(i + 2, close)) });
        i = close + 2;
        continue;
      }
    }
    if (c === "*" || c === "_") {
      const double = src[i + 1] === c;
      const mark = double ? c + c : c;
      const before = src[i - 1] ?? " ";
      const after = src[i + mark.length] ?? " ";
      // `_` inside words (snake_case) is not emphasis; nor is a marker before a space.
      const opens = !/\s/.test(after) && (c === "*" || !/[\p{L}\p{N}]/u.test(before));
      const close = opens ? findClose(src, i + mark.length, mark) : -1;
      if (close > 0) {
        flush();
        out.push({ kind: double ? "strong" : "em", children: parseInline(src.slice(i + mark.length, close)) });
        i = close + mark.length;
        continue;
      }
      text += mark;
      i += mark.length;
      continue;
    }
    const url = c === "h" ? matchAt(URL, src, i) : null;
    if (url && !/[\p{L}\p{N}]/u.test(src[i - 1] ?? " ")) {
      flush();
      out.push({ kind: "link", href: url[0], children: [{ kind: "text", text: url[0] }] });
      i += url[0].length;
      continue;
    }
    text += c;
    i++;
  }
  flush();
  return out;
}

/** The closing `mark` for emphasis opened before `from`, skipping code spans. */
function findClose(src: string, from: number, mark: string): number {
  for (let i = from; i < src.length; i++) {
    const c = src[i]!;
    if (c === "\\") {
      i++;
      continue;
    }
    if (c === "`") {
      const close = src.indexOf("`", i + 1);
      if (close > 0) i = close;
      continue;
    }
    if (src.startsWith(mark, i) && i > from && !/\s/.test(src[i - 1]!)) {
      const after = src[i + mark.length] ?? " ";
      if (mark[0] === "_" && /[\p{L}\p{N}]/u.test(after)) continue;
      // `**` must not be read as the end of a single `*`.
      if (mark.length === 1 && src[i + 1] === mark && src[i - 1] !== mark) {
        i++;
        continue;
      }
      return i;
    }
  }
  return -1;
}

/** `[label](href "title")` starting at `start` (the `[`). */
function linkAt(src: string, start: number): { label: string; href: string; end: number } | null {
  let depth = 0;
  let i = start;
  for (; i < src.length; i++) {
    const c = src[i];
    if (c === "\\") i++;
    else if (c === "`") {
      const close = src.indexOf("`", i + 1);
      if (close > 0) i = close;
    } else if (c === "[") depth++;
    else if (c === "]" && --depth === 0) break;
  }
  if (src[i] !== "]" || src[i + 1] !== "(") return null;
  const label = src.slice(start + 1, i);
  let j = i + 2;
  let parens = 1;
  for (; j < src.length; j++) {
    if (src[j] === "(") parens++;
    else if (src[j] === ")" && --parens === 0) break;
  }
  if (j >= src.length) return null;
  let href = src.slice(i + 2, j).trim();
  href = href.replace(/\s+("[^"]*"|'[^']*')$/, "");
  if (href.startsWith("<") && href.endsWith(">")) href = href.slice(1, -1);
  return { label, href, end: j + 1 };
}
