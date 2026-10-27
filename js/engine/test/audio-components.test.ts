// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { AudioListener, AudioSource, GameObject, Quaternion, Vector3 } from "../src/index.ts";
import { _resetAudioComponentsForTests } from "../src/audio-components.ts";
import { _resetAudioForTests } from "../src/audio.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetAudioForTests();
  _resetAudioComponentsForTests();
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
});

const audioOps = () => ops.filter((o) => String(o[0]).startsWith("audio."));
const ended = (...ids: number[]) => (globalThis as { __platypus_audio: (e: number[]) => void }).__platypus_audio(ids);

// M5-AUD-01
describe("AudioSource", () => {
  test("a 2D source plays its clip on awake and applies volume, pitch and mute live", () => {
    const go = new GameObject("radio");
    const src = go.addComponent(AudioSource, { clip: "assets/song.ogg", loop: true, volume: 0.5 });
    expect(audioOps()).toEqual([
      ["audio.load", 1, "assets/song.ogg", false],
      ["audio.play", 1, 1, null, [0.5, 1, 0, true, 0, 0]],
    ]);
    expect(src.isPlaying).toBe(true);
    ops = [];
    src.pitch = 1.5;
    src.mute = true;
    expect(audioOps()).toEqual([
      ["audio.set", 1, "rate", 1.5, 0],
      ["audio.set", 1, "volume", 0, 0],
    ]);
    src.pause();
    expect(src.isPlaying).toBe(false);
    src.unPause();
    expect(src.isPlaying).toBe(true);
    src.stop();
    expect(src.isPlaying).toBe(false);
  });

  test("a 3D source plays from an emitter that follows its object", () => {
    const go = new GameObject("fire", { position: new Vector3(3, 0, 0) });
    const src = go.addComponent(AudioSource, { clip: "assets/crackle.wav", spatialBlend: 1, maxDistance: 20 });
    expect(audioOps()).toEqual([
      ["audio.load", 1, "assets/crackle.wav", false],
      ["audio.emitter", 1, [3, 0, 0], [1, 20], null],
      ["audio.play", 1, 1, 1, [1, 1, 0, false, 0, 0]],
    ]);
    ops = [];
    src.onPreRender();
    expect(audioOps()).toEqual([]);
    go.transform.position = new Vector3(4, 0, 1);
    src.onPreRender();
    expect(audioOps()).toEqual([["audio.emitterPosition", 1, [4, 0, 1], 0]]);
    ops = [];
    src.onDestroy();
    expect(audioOps()).toEqual([
      ["audio.stop", 1, 0],
      ["audio.removeTrack", 1],
    ]);
  });

  test("one-shots don't interrupt play(); finished ones stop getting updates", () => {
    const go = new GameObject("feet");
    const src = go.addComponent(AudioSource, { clip: "assets/hum.wav", loop: true });
    src.playOneShot("assets/step.wav", 0.5);
    expect(audioOps().at(-1)).toEqual(["audio.play", 2, 2, null, [0.5, 1, 0, false, 0, 0]]);
    ended(2);
    ops = [];
    src.volume = 0.8;
    expect(audioOps()).toEqual([["audio.set", 1, "volume", 0.8, 0]]);
    // The same path loads once.
    src.playOneShot("assets/step.wav");
    expect(audioOps().filter((o) => o[0] === "audio.load")).toEqual([]);
  });

  test("playClipAtPoint removes its emitter when the sound ends", () => {
    const p = AudioSource.playClipAtPoint("assets/boom.wav", new Vector3(0, 1, 0), 0.7);
    expect(audioOps().slice(-1)).toEqual([["audio.play", 1, 1, 1, [0.7, 1, 0, false, 0, 0]]]);
    ended(p.id);
    expect(audioOps().at(-1)).toEqual(["audio.removeTrack", 1]);
  });
});

// M5-AUD-02
describe("AudioListener", () => {
  test("sends its pose on awake and when it moves or turns", () => {
    const cam = new GameObject("camera", { position: new Vector3(0, 2, 5) });
    const l = cam.addComponent(AudioListener);
    expect(audioOps()).toEqual([["audio.listener", [0, 2, 5], [0, 0, 0, 1]]]);
    ops = [];
    l.onPreRender();
    expect(audioOps()).toEqual([]);
    cam.transform.rotation = Quaternion.euler(0, 90, 0);
    l.onPreRender();
    expect(audioOps().length).toBe(1);
    AudioListener.volume = 0.25;
    expect(audioOps().at(-1)).toEqual(["audio.trackVolume", null, 0.25, 0]);
    expect(AudioListener.volume).toBe(0.25);
  });
});
