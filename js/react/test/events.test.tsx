import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { useRef, useState } from "react";

import {
  dispatchUiEvent,
  type IronDragEvent,
  type IronFocusEvent,
  type IronInputEvent,
  type IronKeyboardEvent,
  type IronPointerEvent,
  type IronScrollEvent,
  type IronWheelEvent,
} from "../src/events.ts";
import { type Instance, instances, resetForTests } from "../src/host.ts";
import { _resetRootForTests, createRoot, Dialog, Input, Text } from "../src/index.ts";
import { CREATE, type Op, SET_ATTRS, SET_STYLE } from "../src/opcodes.ts";

let commits: Op[][] = [];
/** Element ids by `data-name`, from the CREATE + SET_ATTRS ops. */
const ids = new Map<string, number>();

beforeEach(() => {
  commits = [];
  ids.clear();
  (globalThis as { __iron_commit?: (ops: Op[]) => void }).__iron_commit = (ops) => {
    commits.push(ops);
    for (const op of ops) {
      if (op[0] !== CREATE) continue;
      const name = (instances.get(op[1] as number)?.props as Record<string, unknown>)["data-name"];
      if (typeof name === "string") ids.set(name, op[1] as number);
    }
  };
  resetForTests();
  _resetRootForTests();
});
afterEach(() => {
  delete (globalThis as { __iron_commit?: unknown }).__iron_commit;
});

/** The path for `names` (target first), as Rust sends it. */
const path = (...names: string[]) => names.map((n) => ids.get(n)!);
const data = (x = 0, y = 0, button = 0, buttons = 0, detail = 1, dx = 0, dy = 0, mods = 0) => [
  x,
  y,
  button,
  buttons,
  detail,
  dx,
  dy,
  mods,
];

describe("@iron/react events", () => {
  test("installs the global the engine calls", () => {
    expect(globalThis.__iron_ui_event).toBe(dispatchUiEvent);
  });

  // M3-EVT-02
  test("capture runs root-down, then bubbling runs target-up", () => {
    const log: string[] = [];
    const on = (name: string) => (e: IronPointerEvent) =>
      log.push(`${name}:${e.eventPhase}:${(e.currentTarget?.props as { "data-name": string })["data-name"]}`);
    createRoot().render(
      <box data-name="outer" onClick={on("bubble")} onClickCapture={on("capture")}>
        <box data-name="middle" onClick={on("bubble")} onClickCapture={on("capture")}>
          <box data-name="inner" onClick={on("bubble")} onClickCapture={on("capture")} />
        </box>
      </box>,
    );
    const prevented = dispatchUiEvent("click", path("inner", "middle", "outer"), data());
    expect(prevented).toBe(false);
    expect(log).toEqual([
      "capture:1:outer",
      "capture:1:middle",
      "capture:2:inner",
      "bubble:2:inner",
      "bubble:3:middle",
      "bubble:3:outer",
    ]);
  });

  // M3-EVT-02
  test("stopPropagation stops later elements; preventDefault is reported back", () => {
    const log: string[] = [];
    createRoot().render(
      <box data-name="outer" onPointerDown={() => log.push("outer")}>
        <box
          data-name="inner"
          onPointerDown={(e) => {
            log.push("inner pointer");
            e.stopPropagation();
            e.preventDefault();
          }}
          onMouseDown={() => log.push("inner mouse")}
        />
      </box>,
    );
    expect(dispatchUiEvent("pointerdown", path("inner", "outer"), data())).toBe(true);
    // The rest of the stopping element's handlers still run.
    expect(log).toEqual(["inner pointer", "inner mouse"]);
  });

  test("stopping during capture skips the target and bubbling", () => {
    const log: string[] = [];
    createRoot().render(
      <box
        data-name="outer"
        onClickCapture={(e) => {
          log.push("outer capture");
          e.stopPropagation();
        }}
      >
        <box data-name="inner" onClick={() => log.push("inner")} />
      </box>,
    );
    dispatchUiEvent("click", path("inner", "outer"), data());
    expect(log).toEqual(["outer capture"]);
  });

  test("enter and leave don't bubble", () => {
    const log: string[] = [];
    createRoot().render(
      <box data-name="outer" onPointerEnter={() => log.push("outer")}>
        <box data-name="inner" onPointerEnter={() => log.push("inner")} onMouseEnter={() => log.push("mouse")} />
      </box>,
    );
    dispatchUiEvent("pointerenter", path("inner"), data());
    expect(log).toEqual(["inner", "mouse"]);
  });

  test("event fields come from the data array", () => {
    let pointer: IronPointerEvent | undefined;
    let wheel: IronWheelEvent | undefined;
    createRoot().render(
      <box
        data-name="b"
        onDoubleClick={(e) => {
          pointer = e;
        }}
        onWheel={(e) => {
          wheel = e;
        }}
      />,
    );
    dispatchUiEvent("dblclick", path("b"), data(12, 34, 0, 1, 2, 0, 0, 1 | 8));
    expect(pointer).toMatchObject({
      type: "dblclick",
      clientX: 12,
      clientY: 34,
      x: 12,
      button: 0,
      buttons: 1,
      detail: 2,
      shiftKey: true,
      ctrlKey: false,
      metaKey: true,
    });
    expect(pointer!.target.id).toBe(ids.get("b")!);
    expect(pointer!.currentTarget).toBeNull();
    dispatchUiEvent("wheel", path("b"), data(0, 0, 0, 0, 0, 3, -40));
    expect(wheel).toMatchObject({ type: "wheel", deltaX: 3, deltaY: -40, deltaMode: 0 });
  });

  // M3-EVT-03
  test("state set in a click handler commits before dispatch returns", () => {
    function Counter() {
      const [n, setN] = useState(0);
      return <box data-name="button" style={{ width: n }} onClick={() => setN(n + 1)} />;
    }
    createRoot().render(<Counter />);
    expect(commits).toHaveLength(1);
    dispatchUiEvent("click", path("button"), data());
    expect(commits).toHaveLength(2);
    expect(commits[1]).toEqual([[SET_STYLE, ids.get("button")!, "width", 1]]);
    // The handler from the new render sees the new state.
    dispatchUiEvent("click", path("button"), data());
    expect(commits[2]).toEqual([[SET_STYLE, ids.get("button")!, "width", 2]]);
  });

  test("handler errors are logged and dispatch continues", () => {
    const error = spyOn(console, "error").mockImplementation(() => {});
    const log: string[] = [];
    createRoot().render(
      <box data-name="outer" onClick={() => log.push("outer")}>
        <Text
          data-name="inner"
          onClick={() => {
            throw new Error("boom");
          }}
        >
          hi
        </Text>
      </box>,
    );
    dispatchUiEvent("click", path("inner", "outer"), data());
    expect(log).toEqual(["outer"]);
    expect(error.mock.calls[0]![0]).toContain("boom");
    error.mockRestore();
  });

  test("unknown targets, removed elements and unknown types are ignored", () => {
    const log: string[] = [];
    const root = createRoot();
    root.render(<box data-name="b" onClick={() => log.push("b")} />);
    const id = ids.get("b")!;
    expect(dispatchUiEvent("click", [999], data())).toBe(false);
    expect(dispatchUiEvent("keydown", [id], data())).toBe(false);
    root.render(null);
    expect(instances.has(id)).toBe(false);
    expect(dispatchUiEvent("click", [id], data())).toBe(false);
    expect(log).toEqual([]);
  });
  // M3-EVT-04
  test("key events carry key, code, repeat and modifiers, and bubble", () => {
    const log: string[] = [];
    let event: IronKeyboardEvent | undefined;
    createRoot().render(
      <box data-name="form" onKeyDown={(e) => log.push(`form ${e.key}`)}>
        <box
          data-name="field"
          tabIndex={0}
          onKeyDown={(e) => {
            event = e;
            log.push(`field ${e.key}`);
          }}
        />
      </box>,
    );
    const repeatShift = [0, 0, 0, 0, 0, 0, 0, 1, 1, 0];
    dispatchUiEvent("keydown", path("field", "form"), repeatShift, ["A", "KeyA"]);
    expect(log).toEqual(["field A", "form A"]);
    expect(event).toMatchObject({ type: "keydown", key: "A", code: "KeyA", repeat: true, shiftKey: true, ctrlKey: false });
  });

  // M3-EVT-04
  test("focusin/focusout run onFocus/onBlur, bubbling, with relatedTarget", () => {
    const log: string[] = [];
    let focus: IronFocusEvent | undefined;
    createRoot().render(
      <box data-name="form" onFocus={(e) => log.push(`form focus ${e.target.id}`)} onBlur={() => log.push("form blur")}>
        <box data-name="a" tabIndex={0} />
        <box
          data-name="b"
          tabIndex={0}
          onFocus={(e) => {
            focus = e;
          }}
        />
      </box>,
    );
    const related = (id: number) => [0, 0, 0, 0, 0, 0, 0, 0, 0, id];
    dispatchUiEvent("focusout", path("a", "form"), related(ids.get("b")!));
    dispatchUiEvent("focusin", path("b", "form"), related(ids.get("a")!));
    expect(log).toEqual(["form blur", `form focus ${ids.get("b")}`]);
    expect(focus!.relatedTarget!.id).toBe(ids.get("a")!);
  });

  // M3-EVT-04
  test("tabIndex is an attribute and refs can focus and blur through ops", () => {
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    let ref: { current: Instance | null } = { current: null };
    function Field() {
      ref = useRef<Instance>(null);
      return <box data-name="field" ref={ref} tabIndex={0} />;
    }
    createRoot().render(<Field />);
    const id = ids.get("field")!;
    expect(commits[0]).toContainEqual([SET_ATTRS, id, "data-name", "field", "tabIndex", 0]);
    ref.current!.focus();
    ref.current!.blur();
    expect(ops).toEqual([
      ["focus", id],
      ["blur", id],
    ]);
    delete (globalThis as { __iron_op?: unknown }).__iron_op;
  });
  // M3-EVT-05
  test("onScroll gets the scroll position and doesn't bubble; refs scroll through ops", () => {
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    const log: string[] = [];
    let event: IronScrollEvent | undefined;
    let ref: { current: Instance | null } = { current: null };
    function List() {
      ref = useRef<Instance>(null);
      return (
        <box data-name="page" onScroll={() => log.push("page")}>
          <box
            data-name="list"
            ref={ref}
            onScroll={(e) => {
              event = e;
              log.push("list");
            }}
          />
        </box>
      );
    }
    createRoot().render(<List />);
    const data = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 30, 100, 400, 100, 200];
    dispatchUiEvent("scroll", path("list"), data);
    expect(log).toEqual(["list"]);
    expect(event).toMatchObject({
      scrollLeft: 5,
      scrollTop: 30,
      scrollWidth: 100,
      scrollHeight: 400,
      clientWidth: 100,
      clientHeight: 200,
    });
    const id = ids.get("list")!;
    ref.current!.scrollTo(0, 50);
    ref.current!.scrollBy(0, -10);
    ref.current!.scrollIntoView();
    expect(ops).toEqual([
      ["scrollTo", id, 0, 50],
      ["scrollBy", id, 0, -10],
      ["scrollIntoView", id],
    ]);
    delete (globalThis as { __iron_op?: unknown }).__iron_op;
  });
  // M3-DLG-01
  test("dialogs send open/modal/closedBy and get onCancel then onClose, which don't bubble", () => {
    const log: string[] = [];
    function Menu() {
      const [open, setOpen] = useState(true);
      return (
        // `close` doesn't bubble: the page's handler must not run.
        <box data-name="page" {...({ onClose: () => log.push("page close") } as object)}>
          <Dialog
            data-name="dialog"
            open={open}
            modal
            closedBy="any"
            onCancel={() => log.push("cancel")}
            onClose={() => {
              log.push("close");
              setOpen(false);
            }}
          />
        </box>
      );
    }
    createRoot().render(<Menu />);
    const id = ids.get("dialog")!;
    const attrs = commits.flat().find((op) => op[0] === SET_ATTRS && op[1] === id);
    expect(attrs).toEqual([SET_ATTRS, id, "data-name", "dialog", "open", true, "modal", true, "closedBy", "any"]);
    dispatchUiEvent("cancel", path("dialog", "page"), data());
    dispatchUiEvent("close", path("dialog", "page"), data());
    expect(log).toEqual(["cancel", "close"]);
    const closed = commits.flat().filter((op) => op[0] === SET_ATTRS && op[1] === id);
    expect(closed.at(-1)).toEqual([SET_ATTRS, id, "open", false]);
  });
  // M3-ANI-02
  test("onTransitionEnd gets the property name and elapsed seconds, and bubbles", () => {
    const log: string[] = [];
    createRoot().render(
      <box data-name="page" onTransitionEnd={(e) => log.push(`page ${e.propertyName}`)}>
        <box data-name="card" onTransitionEnd={(e) => log.push(`card ${e.propertyName} ${e.elapsedTime}`)} />
      </box>,
    );
    const data = [0, 0, 0, 0, 0, 0.25, 0, 0];
    dispatchUiEvent("transitionend", path("card", "page"), data, ["background-color"]);
    expect(log).toEqual(["card background-color 0.25", "page background-color"]);
  });
  // M3-ANI-03
  test("onAnimationEnd gets the animation name", () => {
    let name = "";
    createRoot().render(<box data-name="card" onAnimationEnd={(e) => (name = e.animationName)} />);
    dispatchUiEvent("animationend", path("card"), [0, 0, 0, 0, 0, 1, 0, 0], ["spin"]);
    expect(name).toBe("spin");
  });
  // M3-EVT-06
  test("pointer capture: ops go out, hasPointerCapture follows the engine's events", () => {
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    let ref: { current: Instance | null } = { current: null };
    const log: string[] = [];
    function Slider() {
      ref = useRef<Instance>(null);
      return (
        <box
          data-name="thumb"
          ref={ref}
          onPointerDown={() => ref.current!.setPointerCapture(1)}
          onGotPointerCapture={() => log.push("got")}
          onLostPointerCapture={() => log.push("lost")}
        />
      );
    }
    createRoot().render(<Slider />);
    const id = ids.get("thumb")!;
    dispatchUiEvent("pointerdown", path("thumb"), data());
    expect(ops).toEqual([["setPointerCapture", id]]);
    expect(ref.current!.hasPointerCapture(1)).toBe(false);
    dispatchUiEvent("gotpointercapture", path("thumb"), data());
    expect(ref.current!.hasPointerCapture(1)).toBe(true);
    dispatchUiEvent("lostpointercapture", path("thumb"), data());
    expect(ref.current!.hasPointerCapture(1)).toBe(false);
    expect(log).toEqual(["got", "lost"]);
    delete (globalThis as { __iron_op?: unknown }).__iron_op;
  });
  // M3-INP-01
  test("onChange sees the new text in e.target.value and controlled fields commit it", () => {
    let event: IronInputEvent | undefined;
    let seenTarget = "";
    function Form() {
      const [name, setName] = useState("");
      return (
        <Input
          data-name="name"
          value={name}
          placeholder="Name"
          onChange={(e) => {
            event = e;
            seenTarget = e.target.value!;
            setName(e.target.value!.toUpperCase());
          }}
        />
      );
    }
    createRoot().render(<Form />);
    const id = ids.get("name")!;
    expect(commits[0]).toContainEqual([CREATE, id, "input"]);
    expect(commits[0]).toContainEqual([SET_ATTRS, id, "data-name", "name", "value", "", "placeholder", "Name"]);

    dispatchUiEvent("input", path("name"), data(), ["ab"]);
    expect(event!.value).toBe("ab");
    expect(seenTarget).toBe("ab");
    // The handler upper-cased it: the new value goes back to the engine.
    expect(commits.at(-1)).toEqual([[SET_ATTRS, id, "value", "AB"]]);
    expect(instances.get(id)!.value).toBe("AB");
  });

  test("a controlled field whose handler rejects the edit goes back to its value", () => {
    function Digits() {
      const [v, setV] = useState("12");
      return <Input data-name="digits" value={v} onChange={(e) => /^\d*$/.test(e.target.value!) && setV(e.target.value!)} />;
    }
    createRoot().render(<Digits />);
    const id = ids.get("digits")!;
    dispatchUiEvent("input", path("digits"), data(), ["12x"]);
    expect(commits.at(-1)).toEqual([[SET_ATTRS, id, "value", "12"]]);
    expect(instances.get(id)!.value).toBe("12");
    dispatchUiEvent("input", path("digits"), data(), ["123"]);
    expect(commits.at(-1)).toEqual([[SET_ATTRS, id, "value", "123"]]);
  });

  test("uncontrolled fields keep what was typed", () => {
    const seen: string[] = [];
    createRoot().render(<Input data-name="free" defaultValue="hi" onChange={(e) => seen.push(e.target.value!)} />);
    const id = ids.get("free")!;
    expect(instances.get(id)!.value).toBe("hi");
    const before = commits.length;
    dispatchUiEvent("input", path("free"), data(), ["hi!"]);
    expect(seen).toEqual(["hi!"]);
    expect(commits.length).toBe(before);
  });
});

describe("drag and drop", () => {
  // M5-DND-06
  test("one dataTransfer carries data from dragstart to drop; dragover accepts by preventing", () => {
    const log: string[] = [];
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    createRoot().render(
      <box data-name="root">
        <box
          data-name="item"
          draggable
          onDragStart={(e: IronDragEvent) => {
            e.dataTransfer.setData("text/plain", "asset.png");
            e.dataTransfer.setDragImage(e.target, 4, 5);
            log.push("start");
          }}
          onDragEnd={(e: IronDragEvent) => log.push(`end dropped=${e.dropped} effect=${e.dataTransfer.dropEffect}`)}
        />
        <box
          data-name="folder"
          onDragOver={(e: IronDragEvent) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
          }}
          onDrop={(e: IronDragEvent) => log.push(`drop ${e.dataTransfer.getData("text/plain")} ${e.dataTransfer.types.join(",")}`)}
        />
      </box>,
    );
    const item = ids.get("item")!;
    expect(commits.flat().some((op) => op[0] === SET_ATTRS && op[1] === item && op.includes("draggable"))).toBe(true);
    expect(dispatchUiEvent("dragstart", path("item", "root"), data())).toBe(false);
    expect(ops).toEqual([["dragImage", item, 4, 5]]);
    expect(dispatchUiEvent("dragover", path("folder", "root"), data())).toBe(true);
    dispatchUiEvent("drop", path("folder", "root"), data());
    dispatchUiEvent("dragend", path("item", "root"), data(0, 0, 0, 0, 1));
    expect(log).toEqual(["start", "drop asset.png text/plain", "end dropped=true effect=move"]);
    delete (globalThis as { __iron_op?: unknown }).__iron_op;
  });

  // M5-DND-06
  test("files from the system arrive as dataTransfer.files", () => {
    const got: string[] = [];
    createRoot().render(
      <box
        data-name="drop"
        onDragOver={(e: IronDragEvent) => e.preventDefault()}
        onDrop={(e: IronDragEvent) => got.push(...e.dataTransfer.files.map((f) => `${f.name}@${f.path}`), e.dataTransfer.types.join())}
      />,
    );
    const files = ["C:\\in\\a.png", "/home/me/b.txt"];
    expect(dispatchUiEvent("dragenter", path("drop"), data(), files)).toBe(false);
    expect(dispatchUiEvent("dragover", path("drop"), data(), files)).toBe(true);
    dispatchUiEvent("drop", path("drop"), data(), files);
    expect(got).toEqual(["a.png@C:\\in\\a.png", "b.txt@/home/me/b.txt", "Files"]);
  });
});
