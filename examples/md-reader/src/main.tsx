// A Markdown reader for Iron's own docs, drawn by Iron's UI: a file tree of docs/ and the
// repo's top-level .md files (toggled from the top bar), back / forward history (the
// buttons, Alt+Left / Alt+Right, or the mouse's side buttons), links between docs and to
// headings, tables, code, task lists and images. gen-docs.ts bundles the files at build time.
import "./reader.css";

import { input } from "@iron/engine";
import {
  createRoot,
  Image,
  type IronElement,
  type IronPointerEvent,
  type IronScrollEvent,
  Span,
  Text,
} from "@iron/react";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";

import { DOCS, IMAGES } from "./docs.gen.ts";
import { type Block, type Doc, type Inline, parse } from "./markdown.ts";

const HOME = DOCS["docs/README.md"] !== undefined ? "docs/README.md" : Object.keys(DOCS)[0]!;
const IMAGE_FILES = new Set(IMAGES);
const VIDEO = /\.(mp4|webm|mov|mkv|avi|m4v|ogv)$/i;

interface Entry {
  path: string;
  anchor: string | null;
  /** Where the page was scrolled to when we left it. */
  scroll: number;
}

// --- Paths -----------------------------------------------------------------------------

/** `href` relative to the doc at `from`, as a repo path (`..` resolved). */
function resolve(from: string, href: string): string {
  const parts = href.startsWith("/") ? [] : from.split("/").slice(0, -1);
  for (const part of href.split("/")) {
    if (part === "..") parts.pop();
    else if (part && part !== ".") parts.push(part);
  }
  return parts.join("/");
}

const isExternal = (href: string) => /^[a-z][a-z0-9+.-]*:/i.test(href);

// --- File tree -------------------------------------------------------------------------

interface Folder {
  name: string;
  path: string;
  folders: Folder[];
  files: string[];
}

function buildTree(paths: string[]): Folder {
  const root: Folder = { name: "", path: "", folders: [], files: [] };
  for (const path of paths) {
    const parts = path.split("/");
    let at = root;
    for (const part of parts.slice(0, -1)) {
      let next = at.folders.find((f) => f.name === part);
      if (!next) {
        next = { name: part, path: at.path ? `${at.path}/${part}` : part, folders: [], files: [] };
        at.folders.push(next);
      }
      at = next;
    }
    at.files.push(path);
  }
  // README first in each folder, then by name.
  const sort = (f: Folder) => {
    const name = (p: string) => p.split("/").pop()!.toLowerCase();
    f.files.sort((a, b) => Number(name(b) === "readme.md") - Number(name(a) === "readme.md") || name(a).localeCompare(name(b)));
    f.folders.sort((a, b) => a.name.localeCompare(b.name));
    f.folders.forEach(sort);
  };
  sort(root);
  return root;
}

const TREE = buildTree(Object.keys(DOCS));

const title = (path: string) => {
  const heading = /^#\s+(.+)$/m.exec(DOCS[path] ?? "")?.[1];
  return heading?.replace(/[`*_]/g, "") ?? path.split("/").pop()!;
};

function FileTree({
  folder,
  depth,
  current,
  open,
  onToggle,
  onOpen,
}: {
  folder: Folder;
  depth: number;
  current: string;
  open: ReadonlySet<string>;
  onToggle: (path: string) => void;
  onOpen: (path: string) => void;
}) {
  const pad = { paddingLeft: 10 + depth * 14 };
  return (
    <>
      {folder.folders.map((f) => (
        <box key={f.path} className="tree-group">
          <box className="tree-row folder" style={pad} onClick={() => onToggle(f.path)}>
            <box className="twisty">
              <box className={open.has(f.path) ? "chevron open" : "chevron"} />
            </box>
            <Text className="tree-label">{f.name}/</Text>
          </box>
          {open.has(f.path) ? (
            <FileTree
              folder={f}
              depth={depth + 1}
              current={current}
              open={open}
              onToggle={onToggle}
              onOpen={onOpen}
            />
          ) : null}
        </box>
      ))}
      {folder.files.map((p) => (
        <box
          key={p}
          className={p === current ? "tree-row file current" : "tree-row file"}
          style={pad}
          onClick={() => onOpen(p)}
        >
          <box className="twisty" />
          <Text className="tree-label">{p.split("/").pop()}</Text>
        </box>
      ))}
    </>
  );
}

// --- Markdown to Iron UI ---------------------------------------------------------------

interface Ctx {
  path: string;
  follow: (href: string) => void;
  hover: (href: string | null) => void;
  headings: Map<string, IronElement>;
}

function inlines(list: Inline[], ctx: Ctx): ReactNode[] {
  return list.map((node, i) => {
    switch (node.kind) {
      case "text":
        return node.text;
      case "br":
        return "\n";
      case "code":
        return (
          <Span key={i} className="code">
            {node.text}
          </Span>
        );
      case "strong":
        return (
          <Span key={i} className="strong">
            {inlines(node.children, ctx)}
          </Span>
        );
      case "em":
        return (
          <Span key={i} className="em">
            {inlines(node.children, ctx)}
          </Span>
        );
      case "del":
        return (
          <Span key={i} className="del">
            {inlines(node.children, ctx)}
          </Span>
        );
      case "link":
        return (
          <Span
            key={i}
            className={isExternal(node.href) ? "link external" : "link"}
            onClick={() => ctx.follow(node.href)}
            onPointerEnter={() => ctx.hover(node.href)}
            onPointerLeave={() => ctx.hover(null)}
          >
            {inlines(node.children, ctx)}
          </Span>
        );
      case "image":
        // Images can't sit inside text; `Flow` lifts them out first.
        return `[${node.alt}]`;
    }
  });
}

function Picture({ src, alt, ctx }: { src: string; alt: string; ctx: Ctx }) {
  const path = isExternal(src) ? src : resolve(ctx.path, src);
  if (VIDEO.test(path)) {
    return <Text className="missing">{`▶ Video not shown: ${alt || path}`}</Text>;
  }
  if (!IMAGE_FILES.has(path)) {
    return <Text className="missing">{`Image not available: ${alt || path}`}</Text>;
  }
  return (
    <box className="figure">
      <Image className="picture" src={`content/${path}`} />
      {alt ? <Text className="caption">{alt}</Text> : null}
    </box>
  );
}

/** Inline content as text, with any images split out between the runs of text. */
function Flow({ list, ctx, className }: { list: Inline[]; ctx: Ctx; className: string }) {
  const unwrap = (n: Inline): Inline => (n.kind === "link" && n.children.length === 1 && n.children[0]!.kind === "image" ? n.children[0]! : n);
  const items = list.map(unwrap);
  if (!items.some((n) => n.kind === "image")) return <Text className={className}>{inlines(items, ctx)}</Text>;
  const parts: ReactNode[] = [];
  let run: Inline[] = [];
  const flush = () => {
    if (run.some((n) => n.kind !== "text" || n.text.trim())) {
      parts.push(
        <Text key={parts.length} className={className}>
          {inlines(run, ctx)}
        </Text>,
      );
    }
    run = [];
  };
  for (const n of items) {
    if (n.kind === "image") {
      flush();
      parts.push(<Picture key={parts.length} src={n.src} alt={n.alt} ctx={ctx} />);
    } else run.push(n);
  }
  flush();
  return <box className="flow">{parts}</box>;
}

function Blocks({ blocks, ctx }: { blocks: Block[]; ctx: Ctx }) {
  return <>{blocks.map((b, i) => <BlockView key={i} block={b} ctx={ctx} />)}</>;
}

function BlockView({ block, ctx }: { block: Block; ctx: Ctx }): ReactNode {
  switch (block.kind) {
    case "heading":
      return (
        <Text
          className={`h h${block.level}`}
          ref={(el: IronElement | null) => {
            if (el) ctx.headings.set(block.id, el);
          }}
        >
          {inlines(block.children, ctx)}
        </Text>
      );
    case "paragraph":
      return <Flow list={block.children} ctx={ctx} className="p" />;
    case "code":
      return (
        <box className="pre">
          {block.lang ? <Text className="lang">{block.lang}</Text> : null}
          <Text className="pre-text">{block.text}</Text>
        </box>
      );
    case "rule":
      return <box className="rule" />;
    case "quote":
      return (
        <box className="quote">
          <Blocks blocks={block.children} ctx={ctx} />
        </box>
      );
    case "list":
      return (
        <box className="list">
          {block.items.map((item, i) => (
            <box key={i} className="item">
              {item.checked === null ? (
                <Text className="marker">{block.ordered ? `${block.start + i}.` : "•"}</Text>
              ) : (
                <box className="marker task">
                  <box className={item.checked ? "check done" : "check"}>{item.checked ? <box className="tick" /> : null}</box>
                </box>
              )}
              <box className="item-body">
                <Blocks blocks={item.children} ctx={ctx} />
              </box>
            </box>
          ))}
        </box>
      );
    case "table": {
      const cols = Math.max(block.head.length, ...block.rows.map((r) => r.length));
      const cell = (content: Inline[] | undefined, c: number, head: boolean, key: string, odd: boolean) => (
        <box key={key} className={head ? "cell head" : odd ? "cell odd" : "cell"}>
          <Flow list={content ?? []} ctx={ctx} className={`cell-text ${block.align[c] ?? "left"}`} />
        </box>
      );
      return (
        <box className="table-wrap">
          <box className="table" style={{ gridTemplateColumns: `repeat(${cols}, auto)` }}>
            {Array.from({ length: cols }, (_, c) => cell(block.head[c], c, true, `h${c}`, false))}
            {block.rows.flatMap((row, r) =>
              Array.from({ length: cols }, (_, c) => cell(row[c], c, false, `${r}:${c}`, r % 2 === 1)),
            )}
          </box>
        </box>
      );
    }
  }
}

// --- The app ---------------------------------------------------------------------------

const cache = new Map<string, Doc>();
const docAt = (path: string) => {
  let doc = cache.get(path);
  if (!doc) {
    doc = parse(DOCS[path] ?? `# Not found\n\n\`${path}\` is not in the reader.`);
    cache.set(path, doc);
  }
  return doc;
};

function App() {
  const [history, setHistory] = useState<{ entries: Entry[]; index: number }>({
    entries: [{ path: HOME, anchor: null, scroll: 0 }],
    index: 0,
  });
  const [sidebar, setSidebar] = useState(true);
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set(["docs"]));
  const [hovered, setHovered] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const content = useRef<IronElement>(null);
  const scrollTop = useRef(0);
  const headings = useRef(new Map<string, IronElement>());

  const entry = history.entries[history.index]!;
  const doc = docAt(entry.path);

  const go = useCallback((path: string, anchor: string | null) => {
    setStatus("");
    setHistory(({ entries, index }) => {
      const here = entries[index]!;
      if (here.path === path && here.anchor === anchor && !anchor) return { entries, index };
      const kept = entries.slice(0, index + 1);
      kept[index] = { ...here, scroll: scrollTop.current };
      return { entries: [...kept, { path, anchor, scroll: 0 }], index: index + 1 };
    });
  }, []);

  const step = useCallback((by: -1 | 1) => {
    setHistory(({ entries, index }) => {
      const to = index + by;
      if (to < 0 || to >= entries.length) return { entries, index };
      const kept = [...entries];
      kept[index] = { ...kept[index]!, scroll: scrollTop.current };
      // Coming back lands where the page was left, not on its anchor.
      kept[to] = { ...kept[to]!, anchor: by < 0 || kept[to]!.scroll ? null : kept[to]!.anchor };
      return { entries: kept, index: to };
    });
  }, []);

  const follow = useCallback(
    (href: string) => {
      if (isExternal(href)) {
        setStatus(`External link (open it in a browser): ${href}`);
        return;
      }
      const [file = "", anchor = null] = href.split("#") as [string, string?];
      if (!file) return go(entry.path, anchor || null);
      let path = resolve(entry.path, file);
      if (DOCS[path] === undefined && DOCS[`${path}/README.md`] !== undefined) path = `${path}/README.md`;
      if (DOCS[path] === undefined) {
        setStatus(`Not a doc, so not shown here: ${path}`);
        return;
      }
      go(path, anchor || null);
    },
    [entry.path, go],
  );

  // Show the page: at its anchor, where it was left, or at the top.
  useEffect(() => {
    const el = content.current;
    if (!el) return;
    const target = entry.anchor ? headings.current.get(entry.anchor) : undefined;
    if (target) {
      // Scrolling into view goes the shortest way; from the bottom, that puts the
      // heading at the top.
      el.scrollTo(0, 1e9);
      target.scrollIntoView();
    } else {
      el.scrollTo(0, entry.scroll);
    }
    // Reveal the open file in the tree.
    const folders = entry.path.split("/").slice(0, -1);
    setOpen((o) => {
      const want = folders.map((_, i) => folders.slice(0, i + 1).join("/"));
      return want.every((f) => o.has(f)) ? o : new Set([...o, ...want]);
    });
  }, [entry]);

  // Alt+Left / Alt+Right, like a browser; Ctrl+B toggles the file tree.
  useEffect(() => {
    return input.on("keydown", (e) => {
      const alt = input.getKey("AltLeft") || input.getKey("AltRight");
      const ctrl = input.getKey("ControlLeft") || input.getKey("ControlRight");
      if (alt && e.code === "ArrowLeft") step(-1);
      else if (alt && e.code === "ArrowRight") step(1);
      else if (ctrl && e.code === "KeyB") setSidebar((s) => !s);
    });
  }, [step]);

  const ctx = useMemo<Ctx>(() => {
    headings.current = new Map();
    return { path: entry.path, follow, hover: setHovered, headings: headings.current };
  }, [entry.path, follow]);

  const toggle = (path: string) =>
    setOpen((o) => {
      const next = new Set(o);
      if (!next.delete(path)) next.add(path);
      return next;
    });

  const canBack = history.index > 0;
  const canForward = history.index < history.entries.length - 1;
  const footer = hovered ? (isExternal(hovered) ? hovered : resolve(entry.path, hovered)) : status;

  return (
    <box
      className="app"
      onPointerDown={(e: IronPointerEvent) => {
        // The mouse's side buttons: 3 back, 4 forward.
        if (e.button === 3) step(-1);
        else if (e.button === 4) step(1);
      }}
    >
      <box className="bar">
        <box className={sidebar ? "button on" : "button"} onClick={() => setSidebar(!sidebar)}>
          <box className="burger">
            <box />
            <box />
            <box />
          </box>
        </box>
        <box className="button" disabled={!canBack} onClick={() => step(-1)}>
          <Text>{"←"}</Text>
        </box>
        <box className="button" disabled={!canForward} onClick={() => step(1)}>
          <Text>{"→"}</Text>
        </box>
        <box className="button" onClick={() => go(HOME, null)}>
          <Text>Home</Text>
        </box>
        <Text className="crumbs">
          {entry.path.split("/").map((part, i, all) => (
            <Span key={i} className={i === all.length - 1 ? "crumb last" : "crumb"}>
              {i ? ` / ${part}` : part}
            </Span>
          ))}
        </Text>
        <Text className="doc-title">{title(entry.path)}</Text>
      </box>
      <box className="body">
        {sidebar ? (
          <box className="sidebar">
            <FileTree folder={TREE} depth={0} current={entry.path} open={open} onToggle={toggle} onOpen={(p) => go(p, null)} />
          </box>
        ) : null}
        <box
          className="content"
          ref={content}
          onScroll={(e: IronScrollEvent) => {
            scrollTop.current = e.scrollTop;
          }}
        >
          <box key={entry.path} className="page">
            <Blocks blocks={doc.blocks} ctx={ctx} />
          </box>
        </box>
      </box>
      <box className="status">
        <Text className="status-text">{footer || `${Object.keys(DOCS).length} docs · Alt+← / Alt+→ history · Ctrl+B file tree`}</Text>
      </box>
    </box>
  );
}

createRoot().render(<App />);
