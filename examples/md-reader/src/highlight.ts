// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Syntax highlighting for code blocks: a small single-pass tokenizer per language
// (TypeScript / JavaScript, Rust, WGSL, CSS, JSON, TOML, shell). It only has to look
// right, not parse: keywords, strings, comments, numbers, types, function calls.
// The colours are Atom's One Dark theme (MIT), in reader.css.

export type TokenKind =
  | "plain"
  | "keyword"
  | "string"
  | "comment"
  | "number"
  | "constant"
  | "type"
  | "function"
  | "property"
  | "macro"
  | "meta"
  | "operator"
  | "variable";

export interface Token {
  kind: TokenKind;
  text: string;
}

const words = (s: string) => new Set(s.split(/\s+/).filter(Boolean));

const TS_KEYWORDS = words(`
  abstract as async await break case catch class const continue debugger declare default delete
  do else enum export extends finally for from function get if implements import in instanceof
  interface is keyof let namespace new of private protected public readonly return satisfies set
  static super switch throw try type typeof var void while with yield`);
const TS_CONSTANTS = words("true false null undefined this NaN Infinity");
const TS_TYPES = words("string number boolean any unknown never object symbol bigint void");

const RUST_KEYWORDS = words(`
  as async await break const continue crate dyn else enum extern fn for if impl in let loop match
  mod move mut pub ref return static struct super trait type unsafe use where while`);
const RUST_CONSTANTS = words("true false self Self None Some Ok Err");
const RUST_TYPES = words(`
  i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option
  Result Box`);

const WGSL_KEYWORDS = words(`
  alias break case const const_assert continue continuing default diagnostic discard else enable
  fn for if let loop override requires return struct switch var while`);
const WGSL_TYPES = words(`
  bool f16 f32 i32 u32 vec2 vec3 vec4 vec2f vec3f vec4f vec2i vec3i vec4i vec2u vec3u vec4u mat2x2
  mat3x3 mat4x4 mat2x2f mat3x3f mat4x4f array atomic ptr sampler texture_2d texture_depth_2d`);

const SHELL_KEYWORDS = words(`
  if then else elif fi for in do done case esac while until function return export local set
  unset source`);

interface Lang {
  keywords: Set<string>;
  constants: Set<string>;
  types: Set<string>;
  lineComment?: string;
  blockComment?: boolean;
  /** Template strings (`...`). */
  backticks?: boolean;
  /** Rust: `name!` macros, `'a` lifetimes, `#[attr]`, raw strings. */
  rust?: boolean;
  /** `@decorator` (TS) or `@attribute` (WGSL). */
  at?: boolean;
}

const LANGS: Record<string, Lang> = {
  ts: { keywords: TS_KEYWORDS, constants: TS_CONSTANTS, types: TS_TYPES, lineComment: "//", blockComment: true, backticks: true, at: true },
  rust: { keywords: RUST_KEYWORDS, constants: RUST_CONSTANTS, types: RUST_TYPES, lineComment: "//", blockComment: true, rust: true },
  wgsl: { keywords: WGSL_KEYWORDS, constants: words("true false"), types: WGSL_TYPES, lineComment: "//", blockComment: true, at: true },
};

const ALIASES: Record<string, string> = {
  ts: "ts", tsx: "ts", typescript: "ts", js: "ts", jsx: "ts", javascript: "ts", mjs: "ts",
  rust: "rust", rs: "rust",
  wgsl: "wgsl",
  json: "json", jsonc: "json", json5: "json",
  toml: "toml",
  css: "css",
  bash: "shell", sh: "shell", shell: "shell", zsh: "shell", console: "shell", powershell: "shell", ps1: "shell",
};

/** Tokens for `code` in language `lang` (a fence's info string); one plain token if unknown. */
export function highlight(code: string, lang: string): Token[] {
  const name = ALIASES[lang.toLowerCase()];
  if (!name) return [{ kind: "plain", text: code }];
  const out: Token[] = [];
  const push = (kind: TokenKind, text: string) => {
    const last = out[out.length - 1];
    if (last && last.kind === kind) last.text += text;
    else out.push({ kind, text });
  };
  if (name === "json") json(code, push);
  else if (name === "toml") toml(code, push);
  else if (name === "css") css(code, push);
  else if (name === "shell") shell(code, push);
  else cLike(code, LANGS[name]!, push);
  return out;
}

type Push = (kind: TokenKind, text: string) => void;

function at(re: RegExp, src: string, i: number): string | null {
  re.lastIndex = i;
  return re.exec(src)?.[0] ?? null;
}

const SPACE = /\s+/y;
const NUMBER = /(?:0x[\da-fA-F_]+|0b[01_]+|0o[0-7_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)(?:[a-z]\w*)?/y;
const IDENT = /[A-Za-z_$][\w$]*/y;
const DQ = /"(?:[^"\\\n]|\\.)*"?/y;
const SQ = /'(?:[^'\\\n]|\\.)*'?/y;
const BT = /`(?:[^`\\]|\\.)*`?/y;
const BLOCK = /\/\*[\s\S]*?(?:\*\/|$)/y;
const RAW = /r(#*)"[\s\S]*?"\1/y;
const LIFETIME = /'[A-Za-z_]\w*(?!')/y;
const ATTR = /#!?\[[^\]\n]*\]?/y;
const OPERATOR = /[=!<>+\-*/%&|^~?:]+/y;

function cLike(src: string, lang: Lang, push: Push) {
  let i = 0;
  while (i < src.length) {
    let m: string | null;
    if ((m = at(SPACE, src, i))) push("plain", m);
    else if (lang.lineComment && src.startsWith(lang.lineComment, i)) {
      const end = src.indexOf("\n", i);
      m = src.slice(i, end < 0 ? src.length : end);
      push("comment", m);
    } else if (lang.blockComment && (m = at(BLOCK, src, i))) push("comment", m);
    else if (lang.rust && (m = at(ATTR, src, i))) push("meta", m);
    else if (lang.rust && (m = at(RAW, src, i))) push("string", m);
    else if (lang.rust && (m = at(LIFETIME, src, i))) push("type", m);
    else if ((m = at(DQ, src, i)) || (m = at(SQ, src, i))) push("string", m);
    else if (lang.backticks && (m = at(BT, src, i))) push("string", m);
    else if (lang.at && src[i] === "@" && (m = at(IDENT, src, i + 1))) {
      push("meta", `@${m}`);
      m = `@${m}`;
    } else if ((m = at(NUMBER, src, i)) && !/[\w$]/.test(src[i - 1] ?? " ")) push("number", m);
    else if ((m = at(IDENT, src, i))) push(identKind(src, i, m, lang), m);
    else if ((m = at(OPERATOR, src, i))) push("operator", m);
    else push("plain", (m = src[i]!));
    i += m.length;
  }
}

function identKind(src: string, start: number, word: string, lang: Lang): TokenKind {
  const after = start + word.length;
  if (lang.keywords.has(word)) return "keyword";
  if (lang.constants.has(word)) return "constant";
  if (lang.types.has(word)) return "type";
  if (lang.rust && src[after] === "!" && src[after + 1] !== "=") return "macro";
  let j = after;
  while (src[j] === " ") j++;
  if (src[j] === "(" || (src[j] === "<" && /^<[\w\s,<>[\]'&]*>\s*\(/.test(src.slice(j, j + 80)))) return "function";
  if (/^[A-Z]/.test(word)) return /^[A-Z0-9_]+$/.test(word) && word.length > 1 ? "constant" : "type";
  // `name:` in an object literal or a type (not `a ? b : c`): after `{`, `,`, `(` or
  // at the start of a line.
  if (src[after] === ":" && src[after + 1] !== ":" && lang !== LANGS.rust) {
    let k = start - 1;
    let newline = false;
    while (k >= 0 && /\s/.test(src[k]!)) newline ||= src[k--] === "\n";
    if (k < 0 || newline || "{,(".includes(src[k]!)) return "property";
  }
  return "plain";
}

function json(src: string, push: Push) {
  let i = 0;
  while (i < src.length) {
    let m: string | null;
    if ((m = at(SPACE, src, i))) push("plain", m);
    else if (src.startsWith("//", i)) {
      const end = src.indexOf("\n", i);
      push("comment", (m = src.slice(i, end < 0 ? src.length : end)));
    } else if ((m = at(DQ, src, i))) push(/^\s*:/.test(src.slice(i + m.length, i + m.length + 8)) ? "property" : "string", m);
    else if ((m = at(/-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/y, src, i))) push("number", m);
    else if ((m = at(/true|false|null/y, src, i))) push("constant", m);
    else push("plain", (m = src[i]!));
    i += m.length;
  }
}

function toml(src: string, push: Push) {
  for (const line of src.split(/(?<=\n)/)) {
    let i = 0;
    const section = /^\s*\[\[?[^\]]*\]\]?/.exec(line);
    if (section) {
      push("type", section[0]);
      i = section[0].length;
    } else {
      const key = /^\s*[\w."-]+(?=\s*=)/.exec(line);
      if (key) {
        push("property", key[0]);
        i = key[0].length;
      }
    }
    while (i < line.length) {
      let m: string | null;
      if (line[i] === "#") push("comment", (m = line.slice(i).replace(/\n$/, "")));
      else if ((m = at(DQ, line, i)) || (m = at(SQ, line, i))) push("string", m);
      else if ((m = at(/true|false/y, line, i))) push("constant", m);
      else if ((m = at(/[+-]?\d[\d_.:TZ-]*/y, line, i)) && !/\w/.test(line[i - 1] ?? " ")) push("number", m);
      else push("plain", (m = line[i]!));
      i += m.length;
    }
  }
}

function css(src: string, push: Push) {
  let i = 0;
  let depth = 0;
  while (i < src.length) {
    let m: string | null;
    const c = src[i]!;
    if ((m = at(SPACE, src, i))) push("plain", m);
    else if ((m = at(BLOCK, src, i))) push("comment", m);
    else if ((m = at(DQ, src, i)) || (m = at(SQ, src, i))) push("string", m);
    else if (c === "@" && (m = at(/@[\w-]+/y, src, i))) push("keyword", m);
    else if (c === "{" || c === "}") {
      depth += c === "{" ? 1 : -1;
      push("plain", (m = c));
    } else if (depth > 0 && (m = at(/--?[\w-]+(?=\s*:)|[a-z-]+(?=\s*:)/y, src, i))) push("property", m);
    else if ((m = at(/#[\da-fA-F]{3,8}\b/y, src, i)) && depth > 0) push("number", m);
    else if ((m = at(/-?\d*\.?\d+(?:px|em|rem|%|s|ms|deg|vh|vw|fr)?/y, src, i)) && !/[\w-]/.test(src[i - 1] ?? " ")) push("number", m);
    else if (depth > 0 && (m = at(/[\w-]+(?=\()/y, src, i))) push("function", m);
    else if (depth === 0 && (m = at(/[.#][\w-]+|::?[\w-]+/y, src, i))) push(m.startsWith(":") ? "keyword" : "type", m);
    else if (depth === 0 && (m = at(/[a-z][\w-]*/y, src, i))) push("function", m);
    else if ((m = at(/!important/y, src, i))) push("keyword", m);
    else push("plain", (m = c));
    i += m.length;
  }
}

function shell(src: string, push: Push) {
  for (const line of src.split(/(?<=\n)/)) {
    let i = 0;
    // The first word of a command (at the start, or after a pipe or separator).
    let command = true;
    while (i < line.length) {
      let m: string | null;
      const c = line[i]!;
      if ((m = at(/[ \t]+|\n/y, line, i))) push("plain", m);
      else if (c === "#" && (i === 0 || /\s/.test(line[i - 1]!))) push("comment", (m = line.slice(i).replace(/\n$/, "")));
      else if ((m = at(DQ, line, i)) || (m = at(SQ, line, i))) {
        push("string", m);
        command = false;
      } else if ((m = at(/\$\{[^}\n]*\}?|\$\w+|\$\(|%\w+%/y, line, i))) push("variable", m);
      else if ((m = at(/\|\||&&|[|;&]/y, line, i))) {
        push("operator", m);
        command = true;
      } else if ((m = at(/[^\s|;&"'$]+/y, line, i))) {
        const word = m;
        if (SHELL_KEYWORDS.has(word)) push("keyword", word);
        else if (/^[A-Za-z_]\w*=/.test(word)) {
          const eq = word.indexOf("=");
          push("variable", word.slice(0, eq));
          push("operator", "=");
          if (eq + 1 < word.length) push("string", word.slice(eq + 1));
          i += word.length;
          continue;
        } else if (command) {
          push("function", word);
          command = false;
        } else if (word.startsWith("-")) push("constant", word);
        else if (/^\d+$/.test(word)) push("number", word);
        else push("plain", word);
      } else push("plain", (m = c));
      i += m.length;
    }
  }
}
