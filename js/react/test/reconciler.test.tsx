import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { useEffect, useState } from "react";

import { resetForTests } from "../src/host.ts";
import { _resetRootForTests, createRoot, Span, Text } from "../src/index.ts";
import {
  APPEND_CHILD,
  CREATE,
  CREATE_TEXT,
  INSERT_BEFORE,
  type Op,
  REMOVE_CHILD,
  ROOT_ID,
  SET_ATTRS,
  SET_STYLE,
  SET_TEXT,
} from "../src/opcodes.ts";

/**
 * A tiny model of the Rust UiTree (crates/iron_ui/src/tree.rs) so tests can
 * check the tree the ops produce, not the exact IDs React happens to allocate.
 */
class ModelTree {
  children = new Map<number, number[]>([[ROOT_ID, []]]);
  styles = new Map<number, Record<string, unknown>>();
  parent = new Map<number, number>();
  kinds = new Map<number, string>();
  texts = new Map<number, string>();

  apply(ops: Op[]) {
    for (const [code, ...args] of ops) {
      const [a, b, c] = args as number[];
      switch (code) {
        case CREATE:
          expect(this.children.has(a!)).toBe(false);
          this.children.set(a!, []);
          this.styles.set(a!, {});
          this.kinds.set(a!, args[1] as string);
          break;
        case CREATE_TEXT:
          expect(this.children.has(a!)).toBe(false);
          this.children.set(a!, []);
          this.kinds.set(a!, "#text");
          this.texts.set(a!, args[1] as string);
          break;
        case SET_ATTRS:
          break;
        case SET_TEXT:
          expect(this.kinds.get(a!)).toBe("#text");
          this.texts.set(a!, args[1] as string);
          break;
        case SET_STYLE: {
          const style = this.styles.get(a!)!;
          for (let i = 1; i < args.length; i += 2) {
            const prop = args[i] as string;
            if (args[i + 1] === null) delete style[prop];
            else style[prop] = args[i + 1];
          }
          break;
        }
        case APPEND_CHILD:
        case INSERT_BEFORE: {
          this.detach(b!);
          const siblings = this.children.get(a!)!;
          const index = code === INSERT_BEFORE ? siblings.indexOf(c!) : siblings.length;
          expect(index).toBeGreaterThanOrEqual(0);
          siblings.splice(index, 0, b!);
          this.parent.set(b!, a!);
          break;
        }
        case REMOVE_CHILD: {
          expect(this.parent.get(b!)).toBe(a!);
          this.detach(b!);
          const stack = [b!];
          while (stack.length) {
            const id = stack.pop()!;
            stack.push(...(this.children.get(id) ?? []));
            this.children.delete(id);
            this.styles.delete(id);
          }
          break;
        }
        default:
          throw new Error(`unknown opcode ${code}`);
      }
    }
  }

  private detach(id: number) {
    const p = this.parent.get(id);
    if (p === undefined) return;
    const siblings = this.children.get(p)!;
    siblings.splice(siblings.indexOf(id), 1);
    this.parent.delete(id);
  }

  /** Text nodes and elements as a compact string, e.g. `text(Hello span(big))`. */
  markup(id = ROOT_ID): string {
    if (this.kinds.get(id) === "#text") return JSON.stringify(this.texts.get(id));
    const kids = (this.children.get(id) ?? []).map((c) => this.markup(c)).join(" ");
    return id === ROOT_ID ? kids : `${this.kinds.get(id)}(${kids})`;
  }

  /** The tree as nested `{ style, children }`, for readable assertions. */
  describe(id = ROOT_ID): unknown {
    const kids = (this.children.get(id) ?? []).map((c) => this.describe(c));
    return id === ROOT_ID ? kids : { style: this.styles.get(id), children: kids };
  }
}

let commits: Op[][] = [];
let tree: ModelTree;
beforeEach(() => {
  commits = [];
  tree = new ModelTree();
  (globalThis as { __iron_commit?: (ops: Op[]) => void }).__iron_commit = (ops) => {
    commits.push(ops);
    tree.apply(ops);
  };
  resetForTests();
  _resetRootForTests();
});
afterEach(() => {
  delete (globalThis as { __iron_commit?: unknown }).__iron_commit;
});

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const count = (ops: Op[] | undefined, code: number) => (ops ?? []).filter((op) => op[0] === code).length;
/** Child widths under the first root box: a compact way to read a list's order. */
const widths = () => tree.children.get(tree.children.get(ROOT_ID)![0]!)!.map((id) => tree.styles.get(id)!.width);

describe("@iron/react reconciler", () => {
  // M2-REC-01
  test("rendering a box creates it, styles it and appends it to the root", () => {
    createRoot().render(<box style={{ width: 10, backgroundColor: "red", height: undefined }} />);
    expect(commits).toEqual([
      [
        [CREATE, 1, "box"],
        [SET_STYLE, 1, "width", 10, "backgroundColor", "red"],
        [APPEND_CHILD, ROOT_ID, 1],
      ],
    ]);
  });

  // M2-REC-01, M2-REC-04
  test("a nested tree arrives in one commit with the right shape", () => {
    createRoot().render(
      <box style={{ padding: 4 }}>
        <box style={{ flexGrow: 1 }} />
        <box style={{ width: 5 }} />
      </box>,
    );
    expect(commits).toHaveLength(1);
    expect(count(commits[0], CREATE)).toBe(3);
    expect(tree.describe()).toEqual([
      {
        style: { padding: 4 },
        children: [
          { style: { flexGrow: 1 }, children: [] },
          { style: { width: 5 }, children: [] },
        ],
      },
    ]);
  });

  // M2-REC-02
  test("style changes send only the changed properties", () => {
    const root = createRoot();
    root.render(<box style={{ width: 10, backgroundColor: "red", opacity: 1 }} />);
    root.render(<box style={{ width: 20, opacity: 1, height: 5 }} />);
    expect(commits).toHaveLength(2);
    expect(commits[1]).toEqual([[SET_STYLE, 1, "width", 20, "height", 5, "backgroundColor", null]]);
    expect(tree.describe()).toEqual([{ style: { width: 20, opacity: 1, height: 5 }, children: [] }]);

    // An identical style object produces no ops and no commit.
    root.render(<box style={{ width: 20, opacity: 1, height: 5 }} />);
    expect(commits).toHaveLength(2);
  });

  // M2-REC-03
  test("removing and reordering keyed children moves nodes instead of recreating them", () => {
    const root = createRoot();
    const list = (items: number[]) => (
      <box>
        {items.map((w) => (
          <box key={w} style={{ width: w }} />
        ))}
      </box>
    );
    root.render(list([1, 2, 3]));
    expect(widths()).toEqual([1, 2, 3]);

    root.render(list([1, 3]));
    expect(commits[1]).toHaveLength(1);
    expect(count(commits[1], REMOVE_CHILD)).toBe(1);
    expect(widths()).toEqual([1, 3]);

    root.render(list([3, 1]));
    expect(count(commits[2], CREATE) + count(commits[2], REMOVE_CHILD)).toBe(0);
    expect(widths()).toEqual([3, 1]);

    root.render(list([9, 3, 1]));
    expect(count(commits[3], CREATE)).toBe(1);
    expect(count(commits[3], INSERT_BEFORE)).toBe(1);
    expect(count(commits[3], REMOVE_CHILD)).toBe(0);
    expect(widths()).toEqual([9, 3, 1]);
  });

  // M2-REC-04
  test("a state update from a timer commits once, with only incremental ops", async () => {
    function Counter() {
      const [n, setN] = useState(1);
      useEffect(() => {
        const id = setTimeout(() => setN(3), 1);
        return () => clearTimeout(id);
      }, []);
      return (
        <box style={{ width: n * 10 }}>
          {Array.from({ length: n }, (_, i) => (
            <box key={i} style={{ width: i }} />
          ))}
        </box>
      );
    }
    createRoot().render(<Counter />);
    expect(commits).toHaveLength(1);

    await wait(20);
    expect(commits).toHaveLength(2);
    expect(count(commits[1], CREATE)).toBe(2);
    expect(count(commits[1], REMOVE_CHILD)).toBe(0);
    expect(commits[1]).toContainEqual([SET_STYLE, tree.children.get(ROOT_ID)![0]!, "width", 30]);
    expect(widths()).toEqual([0, 1, 2]);
  });

  // M2-REC-05
  test("node IDs are numbers and never reused", () => {
    const root = createRoot();
    const two = (
      <box>
        <box />
      </box>
    );
    root.render(two);
    root.render(null);
    root.render(
      <box>
        <box />
      </box>,
    );
    const created = commits.flat().filter((op) => op[0] === CREATE).map((op) => op[1]);
    expect(created.every((id) => typeof id === "number")).toBe(true);
    expect(new Set(created).size).toBe(4);
    expect(Math.min(...(created.slice(2) as number[]))).toBeGreaterThan(Math.max(...(created.slice(0, 2) as number[])));
    expect(count(commits[1], REMOVE_CHILD)).toBe(1);
    expect(tree.children.size).toBe(3); // the root plus the two live boxes
  });

  test("unmount removes the root's children", () => {
    const root = createRoot();
    root.render(<box />);
    root.unmount();
    expect(commits[1]).toEqual([[REMOVE_CHILD, ROOT_ID, 1]]);
    expect(tree.describe()).toEqual([]);
  });

  // M3-TXT-07
  test("Text and Span render text elements with text nodes", () => {
    createRoot().render(
      <Text style={{ fontSize: 20 }}>
        Hello <Span style={{ color: "red" }}>big</Span> world
      </Text>,
    );
    expect(commits).toHaveLength(1);
    expect(tree.markup()).toBe('text("Hello " span("big") " world")');
    const textId = tree.children.get(ROOT_ID)![0]!;
    expect(tree.styles.get(textId)).toEqual({ fontSize: 20 });
  });

  // M3-TXT-07
  test("changing text sends SET_TEXT instead of recreating nodes", () => {
    const root = createRoot();
    const label = (n: number) => (
      <Text>
        Score: <Span>{String(n)}</Span>
      </Text>
    );
    root.render(label(1));
    root.render(label(42));
    expect(commits).toHaveLength(2);
    expect(commits[1]).toHaveLength(1);
    expect(commits[1]![0]![0]).toBe(SET_TEXT);
    expect(commits[1]![0]![2]).toBe("42");
    expect(tree.markup()).toBe('text("Score: " span("42"))');
  });

  // M3-TXT-07
  test("bare strings in a box become text nodes", () => {
    createRoot().render(<box>loose text</box>);
    expect(tree.markup()).toBe('box("loose text")');
    expect(commits[0]!.some((op) => op[0] === CREATE_TEXT)).toBe(true);
  });

  // M3-CSS-08 (attributes for selectors)
  test("className, id, data-* and state props are sent as attributes and diffed", () => {
    const root = createRoot();
    root.render(<box className="card big" id="main" data-kind="enemy" disabled style={{ width: 1 }} />);
    expect(commits[0]).toContainEqual([SET_ATTRS, 1, "className", "card big", "id", "main", "data-kind", "enemy", "disabled", true]);

    root.render(<box className="card" id="main" checked style={{ width: 1 }} />);
    expect(commits[1]).toEqual([[SET_ATTRS, 1, "className", "card", "checked", true, "data-kind", null, "disabled", null]]);

    // M3-TXT-09: `dir` is an attribute too.
    root.render(<box className="card" id="main" checked dir="rtl" style={{ width: 1 }} />);
    expect(commits[2]).toEqual([[SET_ATTRS, 1, "dir", "rtl"]]);

    // Text components forward attributes too.
    _resetRootForTests();
    commits = [];
    createRoot().render(<Text className="title">Hi</Text>);
    expect(commits[0]!.some((op) => op[0] === SET_ATTRS && op.includes("title"))).toBe(true);
  });

  test("unknown elements are reported with a clear message", () => {
    const errors = spyOn(console, "error").mockImplementation(() => {});
    try {
      const Unknown = "panel" as unknown as "box";
      createRoot().render(<Unknown />);
      expect(String(errors.mock.calls.at(-1)?.[0])).toContain("<panel> is not an Iron element");
    } finally {
      errors.mockRestore();
    }
  });

  test("only one root per game", () => {
    createRoot();
    expect(() => createRoot()).toThrow("only be called once");
  });
});
