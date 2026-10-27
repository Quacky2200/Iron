// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, spyOn, test } from "bun:test";

import { Debug, input, Log, Random, Rng, Task, Time, TimeSince, TimeUntil, Vector3 } from "../src/index.ts";
import { _resetTimeForTests } from "../src/clock.ts";
import { _resetInputForTests } from "../src/input.ts";
import { Console, formatValue } from "../src/log.ts";

const tick = (dt = 1 / 60) => globalThis.__platypus_tick(dt);

beforeEach(() => {
  _resetTimeForTests();
  _resetInputForTests();
});

// M5-HLP-01
describe("logging", () => {
  test("Unity, s&box and C# names reach the console at the right level", () => {
    const log = spyOn(console, "log").mockImplementation(() => {});
    const warn = spyOn(console, "warn").mockImplementation(() => {});
    const error = spyOn(console, "error").mockImplementation(() => {});
    const info = spyOn(console, "info").mockImplementation(() => {});
    Debug.log("pos", new Vector3(1, 2, 3), { hp: 3 }, [1, 2]);
    Debug.logWarning("careful");
    Log.info("hello");
    Log.error("bad");
    Console.writeLine("line");
    Console.warning("w");
    Debug.assert(1 + 1 === 2, "fine");
    Debug.assert(false, "broken");
    expect(log.mock.calls).toEqual([["pos (1, 2, 3) {\"hp\":3} [1,2]"], ["line"]]);
    expect(warn.mock.calls).toEqual([["careful"], ["w"]]);
    expect(info.mock.calls).toEqual([["hello"]]);
    expect(error.mock.calls[0]).toEqual(["bad"]);
    expect(String(error.mock.calls[1]![0])).toStartWith("Assertion failed: broken");
    for (const s of [log, warn, error, info]) s.mockRestore();
    const loop: { self?: unknown } = {};
    loop.self = loop;
    expect(formatValue(loop)).toBe('{"self":"[circular]"}');
    expect(formatValue(new Map([["a", 1]]))).toBe('{"a":1}');
  });
});

// M5-HLP-02
describe("time", () => {
  test("Time advances per tick and timeScale scales what components get", () => {
    expect(tick(0.5)).toBe(0.5);
    Time.timeScale = 0.5;
    expect(tick(0.5)).toBe(0.25);
    expect([Time.time, Time.unscaledTime, Time.deltaTime, Time.frameCount]).toEqual([0.75, 1, 0.25, 2]);
    Time.timeScale = -1;
    expect(Time.timeScale).toBe(1);
  });

  test("TimeSince, TimeUntil and Task.frame", async () => {
    const since = TimeSince.now();
    const until = TimeUntil.in(1);
    tick(0.5);
    expect(since.seconds).toBe(0.5);
    expect(since.over(0.4)).toBe(true);
    expect(until.done).toBe(false);
    expect(until.fraction).toBe(0.5);
    tick(0.5);
    expect(until.done).toBe(true);
    expect(until.seconds).toBe(0);

    let resumed = false;
    const wait = Task.frame().then(() => {
      resumed = true;
    });
    await Promise.resolve();
    expect(resumed).toBe(false);
    tick();
    await wait;
    expect(resumed).toBe(true);
  });
});

// M5-HLP-03
describe("random", () => {
  test("seeded generators repeat and stay in range", () => {
    const a = new Rng(42);
    const b = new Rng(42);
    const seq = Array.from({ length: 5 }, () => a.next(10));
    expect(Array.from({ length: 5 }, () => b.next(10))).toEqual(seq);
    for (let i = 0; i < 1000; i++) {
      const n = a.int(-3, 3);
      expect(n >= -3 && n < 3 && Number.isInteger(n)).toBe(true);
      const f = a.float(1, 2);
      expect(f >= 1 && f < 2).toBe(true);
      expect(a.insideUnitSphere().magnitude).toBeLessThanOrEqual(1);
      expect(a.onUnitSphere().magnitude).toBeCloseTo(1, 6);
      expect(a.insideUnitCircle().magnitude).toBeLessThanOrEqual(1);
      const q = a.rotation();
      expect(Math.hypot(q.x, q.y, q.z, q.w)).toBeCloseTo(1, 6);
    }
    expect(a.shuffle([1, 2, 3, 4]).sort()).toEqual([1, 2, 3, 4]);
    expect(a.pick([])).toBeUndefined();
    Random.initState(7);
    const first = Random.value;
    Random.initState(7);
    expect(Random.value).toBe(first);
  });
});

// M5-HLP-04
describe("Unity-style input", () => {
  test("getKeyDown lasts one tick; axes combine keys and sticks", () => {
    globalThis.__platypus_input("keydown", "KeyD", "d", false);
    expect(input.getKeyDown("KeyD")).toBe(false); // not until the tick starts
    tick();
    expect(input.getKeyDown("KeyD")).toBe(true);
    expect(input.getKey("KeyD")).toBe(true);
    expect(input.getAxis("Horizontal")).toBe(1);
    globalThis.__platypus_input("keydown", "KeyD", "d", true); // repeat: not a new press
    tick();
    expect(input.getKeyDown("KeyD")).toBe(false);
    globalThis.__platypus_input("keyup", "KeyD", "d");
    tick();
    expect(input.getKeyUp("KeyD")).toBe(true);
    expect(input.getAxis("Horizontal")).toBe(0);

    globalThis.__platypus_input("gamepadconnected", 0, "Pad");
    globalThis.__platypus_input("gamepadaxis", 0, "LeftStickY", -0.6);
    expect(input.getAxis("Vertical")).toBe(-0.6);
    globalThis.__platypus_input("gamepadbutton", 0, "South", 1);
    expect(input.getButton("Jump")).toBe(true);
    globalThis.__platypus_input("keydown", "Space", " ", false);
    tick();
    expect(input.getButtonDown("Jump")).toBe(true);
  });
});
