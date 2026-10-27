// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";

import { audio } from "../src/index.ts";
import { _resetAudioForTests } from "../src/audio.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetAudioForTests();
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
});

describe("audio", () => {
  test("sounds, buses and emitters become ops", () => {
    const music = audio.createBus({ volume: 0.5 });
    const song = audio.load("assets/song.ogg", { stream: true });
    const p = song.play({ on: music, loop: true, fadeIn: 2 });
    p.setVolume(0.2, 1);
    const fire = audio.createEmitter({ x: 3, y: 0, z: 0 }, { maxDistance: 20 });
    audio.play("assets/crackle.wav", { on: fire, volume: 0.8 });
    fire.setPosition({ x: 4, y: 0, z: 0 }, 0.5);
    audio.setMasterVolume(0.9);
    expect(ops).toEqual([
      ["audio.bus", 1, null, 0.5],
      ["audio.load", 1, "assets/song.ogg", true],
      ["audio.play", 1, 1, 1, [1, 1, 0, true, 2, 0]],
      ["audio.set", 1, "volume", 0.2, 1],
      ["audio.emitter", 2, [3, 0, 0], [1, 20], null],
      ["audio.load", 2, "assets/crackle.wav", false],
      ["audio.play", 2, 2, 2, [0.8, 1, 0, false, 0, 0]],
      ["audio.emitterPosition", 2, [4, 0, 0], 0.5],
      ["audio.trackVolume", null, 0.9, 0],
    ]);
  });

  test("audio.play loads each file once", () => {
    audio.play("click.wav");
    audio.play("click.wav");
    expect(ops.filter((o) => o[0] === "audio.load")).toHaveLength(1);
    expect(ops.filter((o) => o[0] === "audio.play")).toHaveLength(2);
  });

  test("ended playbacks call onEnded once", () => {
    const a = audio.play("a.wav");
    const b = audio.play("b.wav");
    const ended: string[] = [];
    a.onEnded = () => ended.push("a");
    b.onEnded = () => {
      throw new Error("boom");
    };
    const error = spyOn(console, "error").mockImplementation(() => {});
    globalThis.__platypus_audio([a.id, b.id]);
    globalThis.__platypus_audio([a.id, 99]);
    expect(ended).toEqual(["a"]);
    expect(a.playing).toBe(false);
    expect(error).toHaveBeenCalledTimes(1);
    error.mockRestore();
  });
});
