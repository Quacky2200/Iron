// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Sound for gameplay and UI code, mixed by kira in Rust (crates/platypus_audio). Like
// physics, scripts hold handles with numeric IDs and changes go to the engine as
// `audio.*` ops. Rust calls `__platypus_audio(ended)` when playing sounds finish.
//
// Volumes are linear gains (1 = as recorded, 0 = silent); times are seconds.

import { Assets } from "./assets.ts";
import type { QuaternionLike as Quat, Vector3Like as Vec3 } from "./math.ts";

export interface PlayOptions {
  /** Linear gain. Default 1. */
  volume?: number;
  /** Speed and pitch. Default 1. */
  rate?: number;
  /** -1 left, 0 centre, 1 right. Default 0. */
  pan?: number;
  loop?: boolean;
  /** Seconds to fade in over. */
  fadeIn?: number;
  /** Seconds into the sound to start. */
  start?: number;
  /** Play on a bus (its volume applies) or an emitter (positioned in the world). */
  on?: Bus | Emitter;
}

export interface LoadOptions {
  /** Decode while playing instead of up front: for music and long ambience. */
  stream?: boolean;
}

let nextSound = 1;
let nextPlayback = 1;
let nextTrack = 1;
const playing = new Map<number, Playback>();

/** A loaded sound file. Play it as many times as you like, even at once. */
export class Sound {
  readonly id: number;
  readonly src: string;

  /** @internal Use `audio.load()`. */
  constructor(source: string, options: LoadOptions = {}) {
    // A GUID becomes its file's path; the file's `.meta` can ask for streaming.
    const src = Assets.resolve(source);
    this.id = nextSound++;
    this.src = src;
    if (options.stream === undefined && Assets.importer(src).stream === true) options = { ...options, stream: true };
    __platypus_op("audio.load", this.id, src, !!options.stream);
  }

  play(options: PlayOptions = {}): Playback {
    return new Playback(this, options);
  }

  /** Frees the sound's data. Playing copies finish first. */
  unload(): void {
    __platypus_op("audio.unload", this.id);
  }
}

/** One playing copy of a sound. */
export class Playback {
  readonly id: number;
  readonly sound: Sound;
  /** False once it has finished (played out or stopped). */
  playing = true;
  /** Called when it finishes. */
  onEnded?: () => void;

  /** @internal Use `sound.play()`. */
  constructor(sound: Sound, o: PlayOptions) {
    this.id = nextPlayback++;
    this.sound = sound;
    playing.set(this.id, this);
    __platypus_op("audio.play", this.id, sound.id, o.on?.id ?? null, [
      o.volume ?? 1,
      o.rate ?? 1,
      o.pan ?? 0,
      !!o.loop,
      o.fadeIn ?? 0,
      o.start ?? 0,
    ]);
  }

  /** Changes the volume, over `fade` seconds. */
  setVolume(volume: number, fade = 0): void {
    __platypus_op("audio.set", this.id, "volume", volume, fade);
  }

  setRate(rate: number, fade = 0): void {
    __platypus_op("audio.set", this.id, "rate", rate, fade);
  }

  setPan(pan: number, fade = 0): void {
    __platypus_op("audio.set", this.id, "pan", pan, fade);
  }

  pause(fade = 0): void {
    __platypus_op("audio.pause", this.id, fade);
  }

  resume(fade = 0): void {
    __platypus_op("audio.resume", this.id, fade);
  }

  /** Stops it (fading out over `fade` seconds); `onEnded` follows. */
  stop(fade = 0): void {
    __platypus_op("audio.stop", this.id, fade);
  }

  /** Jumps to `seconds` into the sound. */
  seek(seconds: number): void {
    __platypus_op("audio.seek", this.id, seconds);
  }
}

/** A mixer bus (`music`, `sfx`, `ui`): its volume and pause apply to everything on it. */
export class Bus {
  readonly id: number;

  /** @internal Use `audio.createBus()`. */
  constructor(volume: number, parent: Bus | null) {
    this.id = nextTrack++;
    __platypus_op("audio.bus", this.id, parent?.id ?? null, volume);
  }

  setVolume(volume: number, fade = 0): void {
    __platypus_op("audio.trackVolume", this.id, volume, fade);
  }

  pause(fade = 0): void {
    __platypus_op("audio.trackPause", this.id, fade);
  }

  resume(fade = 0): void {
    __platypus_op("audio.trackResume", this.id, fade);
  }

  /** Removes the bus; sounds on it stop. */
  remove(): void {
    __platypus_op("audio.removeTrack", this.id);
  }
}

export interface EmitterOptions {
  /** Full volume within this distance (metres). Default 1. */
  minDistance?: number;
  /** Silent beyond this distance. Default 50. */
  maxDistance?: number;
  /** Mix the emitter into this bus. */
  bus?: Bus;
}

/**
 * A point in the world that sounds play from: they pan and fade with its distance and
 * direction from the listener (`audio.setListener`).
 */
export class Emitter {
  readonly id: number;

  /** @internal Use `audio.createEmitter()`. */
  constructor(position: Vec3, options: EmitterOptions) {
    this.id = nextTrack++;
    __platypus_op(
      "audio.emitter",
      this.id,
      [position.x, position.y, position.z],
      [options.minDistance ?? 1, options.maxDistance ?? 50],
      options.bus?.id ?? null,
    );
  }

  /** Moves the emitter, gliding over `glide` seconds. */
  setPosition(p: Vec3, glide = 0): void {
    __platypus_op("audio.emitterPosition", this.id, [p.x, p.y, p.z], glide);
  }

  setVolume(volume: number, fade = 0): void {
    __platypus_op("audio.trackVolume", this.id, volume, fade);
  }

  /** Removes the emitter; sounds on it stop. */
  remove(): void {
    __platypus_op("audio.removeTrack", this.id);
  }
}

const cache = new Map<string, Sound>();

export const audio = {
  /** Loads a sound file (WAV, OGG, MP3, FLAC) relative to the game folder. */
  load(src: string, options?: LoadOptions): Sound {
    return new Sound(src, options);
  },

  /** Plays a file, loading it the first time (handy for UI clicks and one-off effects). */
  play(src: string, options?: PlayOptions): Playback {
    let sound = cache.get(src);
    if (!sound) {
      sound = new Sound(src);
      cache.set(src, sound);
    }
    return sound.play(options);
  },

  createBus(options: { volume?: number; parent?: Bus } = {}): Bus {
    return new Bus(options.volume ?? 1, options.parent ?? null);
  },

  createEmitter(position: Vec3, options: EmitterOptions = {}): Emitter {
    return new Emitter(position, options);
  },

  /** Where the player hears from (usually the camera). Facing -z with no rotation. */
  setListener(position: Vec3, rotation: Quat = { x: 0, y: 0, z: 0, w: 1 }): void {
    __platypus_op("audio.listener", [position.x, position.y, position.z], [rotation.x, rotation.y, rotation.z, rotation.w]);
  },

  /** The volume of everything. */
  setMasterVolume(volume: number, fade = 0): void {
    __platypus_op("audio.trackVolume", null, volume, fade);
  },
};

/** Clears all handles. Only for tests. */
export function _resetAudioForTests() {
  playing.clear();
  cache.clear();
  nextSound = nextPlayback = nextTrack = 1;
}

globalThis.__platypus_audio = (ended: number[]) => {
  for (const id of ended) {
    const p = playing.get(id);
    if (!p) continue;
    playing.delete(id);
    p.playing = false;
    try {
      p.onEnded?.();
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[audio] error in onEnded: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
    }
  }
};
