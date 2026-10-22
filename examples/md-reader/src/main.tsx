// A Markdown reader for Iron's own docs, drawn by Iron's UI: a file tree of docs/ and the
// repo's top-level .md files (toggled from the top bar), back / forward history (the
// buttons, Alt+Left / Alt+Right, or the mouse's side buttons), links between docs and to
// headings, tables, code, task lists and images. gen-docs.ts bundles the files at build time.
import "./reader.css";


import { DOCS, IMAGES, ROOT } from "./docs.gen.ts";
import { highlight } from "./highlight.ts";
import { type Block, type Doc, type Inline, parse, plainText } from "./markdown.ts";

/** The first page: Iron's `docs/README.md`, else the root's README, else the first doc. */
const HOME =
  ["docs/README.md", "README.md", "readme.md", "index.md"].find((p) => DOCS[p] !== undefined) ??
  Object.keys(DOCS).sort()[0] ??
  "";
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

function BlockView({
  block,
  ctx,
  joined = false,
  mark = null,
}: {
  block: Block;
  ctx: Ctx;
  joined?: boolean;
  /** Marks one of a list's items (`mark.item`). */
  mark?: Mark | null;
}): ReactNode {
  switch (block.kind) {
    case "heading":
      return (
        <Text className={`h h${block.level}`}>
          {inlines(block.children, ctx)}
        </Text>
      );
    case "paragraph":
      return <Flow list={block.children} ctx={ctx} className="p" />;
    case "code":
      return (
        <box className="pre">
          {block.lang ? <Text className="lang">{block.lang}</Text> : null}
          <Text className="pre-text">
            {highlight(block.text, block.lang).map((t, i) =>
              t.kind === "plain" ? (
                t.text
              ) : (
                <Span key={i} className={`tok-${t.kind}`}>
                  {t.text}
                </Span>
              ),
            )}
          </Text>
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
        <box className={joined ? "list joined" : "list"}>
          {block.items.map((item, i) => {
            const view = (
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
            );
            return i === mark?.item ? <Marked key={i} mark={mark}>{view}</Marked> : view;
          })}
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

/** The theme button's icon: a sun to switch to light, a moon to switch back to dark. */
function ThemeIcon({ light }: { light: boolean }) {
  const line = { fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round" } as const;
  return (
    <svg viewBox="0 0 24 24" style={{ width: 18, height: 18 }}>
      {light ? (
        <path d="M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5Z" {...line} strokeLinejoin="round" />
      ) : (
        <>
          <circle cx="12" cy="12" r="4" {...line} />
          <path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" {...line} />
        </>
      )}
    </svg>
  );
}

/** The folder at `path` in the file tree ("" is the top). */
function folderAt(path: string): Folder | undefined {
  let at: Folder | undefined = TREE;
  for (const part of path ? path.split("/") : []) at = at?.folders.find((f) => f.name === part);
  return at;
}

const isFolder = (path: string) => DOCS[path] === undefined && folderAt(path) !== undefined;

/** Links on a page resolve against this: a folder's listing sits inside the folder. */
const base = (path: string) => (isFolder(path) ? (path ? `${path}/` : "") : path);

/** A folder's listing, written as Markdown so it draws like any page. */
function folderMarkdown(path: string, folder: Folder): string {
  const rows = [
    ...folder.folders.map((f) => {
      const n = f.folders.length + f.files.length;
      return `| [${f.name}/](${f.name}/) | ${n} ${n === 1 ? "item" : "items"} |`;
    }),
    ...folder.files.map((p) => {
      const name = p.split("/").pop()!;
      return `| [${name}](${name}) | ${title(p).replaceAll("|", "\\|")} |`;
    }),
  ];
  return `# ${path || ROOT.name}/\n\n| Name | Title |\n|---|---|\n${rows.join("\n")}\n`;
}

const cache = new Map<string, Doc>();
const docAt = (path: string) => {
  let doc = cache.get(path);
  if (!doc) {
    const folder = isFolder(path) ? folderAt(path) : undefined;
    const source = folder ? folderMarkdown(path, folder) : DOCS[path];
    doc = parse(source ?? `# Not found\n\n\`${path}\` is not in the reader.`);
    cache.set(path, doc);
  }
  return doc;
};

// A new page mounts about a screenful at once, then a chunk a frame, so long docs show
// straight away and finish loading while you read. Sizes are in rough lines of text.
const FIRST_WEIGHT = 80;
const CHUNK_WEIGHT = 60;

/** About how many lines `block` draws: what mounting it costs. */
function weight(block: Block): number {
  switch (block.kind) {
    case "heading":
    case "rule":
      return 1;
    case "paragraph":
      return 1 + Math.floor(plainText(block.children).length / 100);
    case "code":
      return 1 + block.text.split("\n").length;
    case "quote":
      return block.children.reduce((n, b) => n + weight(b), 0);
    case "list":
      return block.items.reduce((n, item) => n + item.children.reduce((m, b) => m + weight(b), 0), 0);
    case "table":
      return 1 + block.rows.length * 2;
  }
}

/** One unit of progressive mounting: a block, or a run of a long list's items. */
interface Row {
  block: Block;
  weight: number;
  /** A list piece the next row continues (no gap under it). */
  joined: boolean;
}

const rowCache = new WeakMap<Doc, Row[]>();

/** The page as rows: long top-level lists are cut between items, so no one frame has to
 * mount a whole list. */
function rowsOf(doc: Doc): Row[] {
  let rows = rowCache.get(doc);
  if (rows) return rows;
  rows = [];
  for (const block of doc.blocks) {
    const w = weight(block);
    if (block.kind !== "list" || w <= CHUNK_WEIGHT / 2) {
      rows.push({ block, weight: w, joined: false });
      continue;
    }
    let from = 0;
    let size = 0;
    block.items.forEach((item, i) => {
      size += item.children.reduce((m, b) => m + weight(b), 0);
      const last = i === block.items.length - 1;
      if (size >= CHUNK_WEIGHT / 4 || last) {
        const items = block.items.slice(from, i + 1);
        rows!.push({ block: { ...block, items, start: block.start + from }, weight: size, joined: !last });
        from = i + 1;
        size = 0;
      }
    });
  }
  rowCache.set(doc, rows);
  return rows;
}

/** The rows that fit in `budget` lines after the first `from`. */
function grow(rows: Row[], from: number, budget: number): number {
  let n = from;
  let used = 0;
  while (n < rows.length && (used < budget || n === from)) used += rows[n++]!.weight;
  return n;
}

/** Rows cut into mounting chunks: the first holds the first `need` rows and a screenful. */
function chunksOf(rows: Row[], need: number): Row[][] {
  const out: Row[][] = [];
  let at = 0;
  while (at < rows.length) {
    const end = out.length ? grow(rows, at, CHUNK_WEIGHT) : grow(rows, need, FIRST_WEIGHT);
    out.push(rows.slice(at, end));
    at = end;
  }
  return out;
}

/** The row a jump went to: highlighted, then fading, and the element scrolled to. */
interface Mark {
  /** Its index among the chunk's rows, -1 for none. */
  at: number;
  /** For a list, the item marked instead of the whole row (-1 for the row). */
  item: number;
  faded: boolean;
  ref: { current: IronElement | null };
}

/** The highlighted box around a jump's target. */
function Marked({ mark, children }: { mark: Mark; children: ReactNode }) {
  return (
    <box
      className={mark.faded ? "mark faded" : "mark"}
      ref={(el: IronElement | null) => {
        mark.ref.current = el;
      }}
    >
      {children}
    </box>
  );
}

// Each chunk is its own box and memoized component: mounting the next one lays out and
// renders only it, not everything above it again.
const Chunk = memo(function Chunk({ rows, ctx, mark }: { rows: Row[]; ctx: Ctx; mark: Mark | null }) {
  return (
    <box className="chunk">
      {rows.map((row, i) => {
        const marked = i === mark?.at ? mark : null;
        if (marked && marked.item >= 0) {
          return <BlockView key={i} block={row.block} ctx={ctx} joined={row.joined} mark={marked} />;
        }
        const view = <BlockView key={i} block={row.block} ctx={ctx} joined={row.joined} />;
        return marked ? (
          <Marked key={i} mark={marked}>
            {view}
          </Marked>
        ) : (
          view
        );
      })}
    </box>
  );
});

/** The page, mounted progressively, a chunk a frame; the first `need` rows come at once.
 * `mark` is where a jump went. */
function Page({
  doc,
  need,
  mark,
  faded,
  markRef,
  ctx,
}: {
  doc: Doc;
  need: number;
  mark: Target;
  faded: boolean;
  markRef: Mark["ref"];
  ctx: Ctx;
}) {
  const rows = rowsOf(doc);
  const chunks = useMemo(() => chunksOf(rows, need), [rows, need]);
  // Only the chunk holding the mark gets one, so the others stay memoized.
  const marks = useMemo(() => {
    let start = 0;
    return chunks.map((chunk) => {
      const at = mark.row - start;
      start += chunk.length;
      return at >= 0 && at < chunk.length ? { at, item: mark.item, faded, ref: markRef } : null;
    });
  }, [chunks, mark, faded, markRef]);
  const [count, setCount] = useState(1);
  useEffect(() => {
    if (count >= chunks.length) return;
    // 1 ms, not 0: a timer set while timers run would otherwise fire in this same frame.
    const id = setTimeout(() => setCount((c) => c + 1), 1);
    return () => clearTimeout(id);
  }, [count, chunks]);
  return (
    <>
      {chunks.slice(0, count).map((chunk, i) => (
        <Chunk key={i} rows={chunk} ctx={ctx} mark={marks[i]!} />
      ))}
    </>
  );
}

/** Where a jump goes: a row, and in a list row one item (-1 for the whole row). */
interface Target {
  row: number;
  item: number;
}

const NOWHERE: Target = { row: -1, item: -1 };

/** The target an anchor names: a heading's id, or `L<n>` (a 1-based source line, as on
 * GitHub; `L<n>-L<m>` goes to the first) for the block or list item holding that line. */
function targetOf(doc: Doc, anchor: string): Target {
  const rows = rowsOf(doc);
  const line = /^L(\d+)(?:-L?\d+)?$/.exec(anchor);
  if (!line) {
    return { row: rows.findIndex((r) => r.block.kind === "heading" && r.block.id === anchor), item: -1 };
  }
  const want = parseInt(line[1]!, 10) - 1;
  let found = NOWHERE;
  rows.forEach((r, row) => {
    const b = r.block;
    if (b.kind !== "list") {
      if (b.line !== undefined && b.line <= want) found = { row, item: -1 };
      return;
    }
    b.items.forEach((it, item) => {
      if (it.line <= want) found = { row, item };
    });
  });
  return found;
}

/** How many rows must be mounted to show `entry` right: through its anchor, or all of
 * them when coming back to a scrolled page. */
function rowsNeeded(doc: Doc, entry: Entry): number {
  const rows = rowsOf(doc);
  if (entry.anchor) {
    const at = targetOf(doc, entry.anchor).row;
    return at < 0 ? rows.length : at;
  }
  return entry.scroll > 0 ? rows.length : 0;
}

/** `--goto "<doc>#<anchor>"` and `--root <folder>` from the command line (after `--`). */
function commandLine(): { goto: string | null; root: string | null } {
  const args = Application.args;
  const value = (name: string) => {
    const i = args.findIndex((a) => a === name || a.startsWith(`${name}=`));
    if (i < 0) return null;
    return args[i]!.includes("=") ? args[i]!.slice(name.length + 1) : (args[i + 1] ?? null);
  };
  return { goto: value("--goto"), root: value("--root") };
}

const clean = (path: string) => path.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");

/** The first history entry and any message about the command line. */
function startAt(): { entry: Entry; status: string } {
  const { goto, root } = commandLine();
  if (root !== null) {
    // Reading a folder while running needs file access Iron's scripts don't have yet (M6
    // plans the game and data folders only), so the option exists but stops here.
    console.error(
      `md-reader: --root ${root}: not supported yet (scripts can't read folders outside the game). ` +
        "Build with MD_READER_ROOT=<folder> bun run build instead.",
    );
    Application.quit(2);
  }
  if (!goto) return { entry: { path: HOME, anchor: null, scroll: 0 }, status: "" };
  const hash = goto.indexOf("#");
  const path = clean(hash < 0 ? goto : goto.slice(0, hash));
  const anchor = hash < 0 ? null : goto.slice(hash + 1) || null;
  if (path && DOCS[path] === undefined && !isFolder(path)) {
    return { entry: { path: HOME, anchor: null, scroll: 0 }, status: `--goto: no doc ${path} in ${ROOT.name}` };
  }
  return { entry: { path: path || HOME, anchor, scroll: 0 }, status: "" };
}

const START = startAt();

function App() {
  const [history, setHistory] = useState<{ entries: Entry[]; index: number }>({
    entries: [START.entry],
    index: 0,
  });
  const [sidebar, setSidebar] = useState(true);
  const [light, setLight] = useState(() => PlayerPrefs.getString("theme") === "light");
  const toggleTheme = () => {
    PlayerPrefs.setString("theme", light ? "dark" : "light");
    setLight(!light);
  };
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set(["docs"]));
  const [hovered, setHovered] = useState<string | null>(null);
  const [status, setStatus] = useState(START.status);
  const content = useRef<IronElement>(null);
  const scrollTop = useRef(0);
  const markRef = useRef<IronElement | null>(null);
  const [faded, setFaded] = useState(false);

  const entry = history.entries[history.index]!;
  const doc = docAt(entry.path);
  const mark = useMemo(() => (entry.anchor ? targetOf(doc, entry.anchor) : NOWHERE), [doc, entry.anchor]);

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
      let path = resolve(base(entry.path), file);
      // `folder/` opens the folder's listing; `folder` alone, its README if it has one.
      if (DOCS[path] === undefined && !file.endsWith("/") && DOCS[`${path}/README.md`] !== undefined) {
        path = `${path}/README.md`;
      }
      if (isFolder(path)) return go(path, null);
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
    const target = entry.anchor ? markRef.current : null;
    if (target) {
      target.scrollIntoView({ block: "start" });
    } else {
      el.scrollTo(0, entry.scroll);
      if (entry.anchor) setStatus((s) => s || `No #${entry.anchor} in ${entry.path}`);
    }
    // The target is highlighted, then fades.
    setFaded(false);
    const fade = setTimeout(() => setFaded(true), 1200);
    // Reveal the open file in the tree.
    const parts = entry.path ? entry.path.split("/") : [];
    const folders = isFolder(entry.path) ? parts : parts.slice(0, -1);
    setOpen((o) => {
      const want = folders.map((_, i) => folders.slice(0, i + 1).join("/"));
      return want.every((f) => o.has(f)) ? o : new Set([...o, ...want]);
    });
    return () => clearTimeout(fade);
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

  const ctx = useMemo<Ctx>(
    () => ({ path: base(entry.path), follow, hover: setHovered }),
    [entry.path, follow],
  );

  const toggle = (path: string) =>
    setOpen((o) => {
      const next = new Set(o);
      if (!next.delete(path)) next.add(path);
      return next;
    });

  const canBack = history.index > 0;
  const canForward = history.index < history.entries.length - 1;
  const footer = hovered ? (isExternal(hovered) ? hovered : resolve(base(entry.path), hovered)) : status;
  // The path bar, like Explorer's: every folder in it opens that folder.
  const parts = entry.path ? entry.path.split("/") : [];
  const crumbs = [{ name: ROOT.name, path: "" }, ...parts.map((name, i) => ({ name, path: parts.slice(0, i + 1).join("/") }))];

  return (
    <box
      className={light ? "app light" : "app"}
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
          <svg viewBox="0 0 24 24" style={{ width: 18, height: 18 }}>
            <path
              d="M3 11 L12 3.5 L21 11 M5.5 9 V20 H10 V14 H14 V20 H18.5 V9"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.8}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </box>
        <Text className="crumbs">
          {crumbs.map((c, i) => {
            const last = i === crumbs.length - 1;
            return (
              <Fragment key={c.path}>
                {i ? <Span className="crumb-sep">{" / "}</Span> : null}
                <Span
                  className={last ? "crumb last" : "crumb"}
                  onClick={last ? undefined : () => go(c.path, null)}
                >
                  {c.name}
                </Span>
              </Fragment>
            );
          })}
        </Text>
        <Text className="doc-title">{isFolder(entry.path) ? `${entry.path || ROOT.name}/` : title(entry.path)}</Text>
        <box className="button" onClick={toggleTheme}>
          <ThemeIcon light={light} />
        </box>
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
            <Page doc={doc} need={rowsNeeded(doc, entry)} mark={mark} faded={faded} markRef={markRef} ctx={ctx} />
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
