// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Unity-style audio components over the handles in audio.ts: an `AudioSource` plays
// sounds from its GameObject (an emitter that follows it) and the `AudioListener` puts
// the player's ears on its object (usually the camera).

import { property } from "./property.ts";
import { audio, type Bus, type Emitter, type Playback, type PlayOptions, Sound } from "./audio.ts";
import type { Vector3Like } from "./math.ts";
import { Component } from "./scene.ts";

const clips = new Map<string, Sound>();

/** A clip given as a path loads once and is shared. */
function clipOf(clip: Sound | string): Sound {
  if (clip instanceof Sound) return clip;
  let sound = clips.get(clip);
  if (!sound) {
    sound = audio.load(clip);
    clips.set(clip, sound);
  }
  return sound;
}

/**
 * Plays sounds from its GameObject (Unity's `AudioSource`). With `spatialBlend` 0 (the
 * default) it plays flat, on `outputBus` if set; above 0 it plays from where its object
 * is, panning and fading with the listener's distance and direction (there's no partial
 * blend: any value above 0 is fully 3D). Plays `clip` when added if `playOnAwake`.
 */
export class AudioSource extends Component {
  /** The sound `play()` plays: a loaded `Sound` or a path in the game folder. */
  @property
  clip: Sound | string | null = null;
  @property
  playOnAwake = true;
  /** 0 plays flat (2D), above 0 plays from the object (3D). */
  @property
  spatialBlend = 0;
  /** Full volume within this distance (metres, 3D only). */
  @property
  minDistance = 1;
  /** Silent beyond this distance (metres, 3D only). */
  @property
  maxDistance = 50;
  /** The bus it plays on (or its emitter mixes into). */
  outputBus: Bus | null = null;
  private _volume = 1;
  private _pitch = 1;
  private _panStereo = 0;
  private _loop = false;
  private _mute = false;
  private main: Playback | null = null;
  private paused = false;
  private readonly all = new Set<Playback>();
  private emitter: Emitter | null = null;
  private emitterKey = "";
  private sentPosition: Vector3Like | null = null;

  /** Linear gain (1 = as recorded). */
  @property
  get volume(): number {
    return this._volume;
  }
  set volume(v: number) {
    this._volume = v;
    this.eachPlaying((p) => p.setVolume(this.gain()));
  }
  /** Speed and pitch together (1 = as recorded). */
  @property
  get pitch(): number {
    return this._pitch;
  }
  set pitch(r: number) {
    this._pitch = r;
    this.eachPlaying((p) => p.setRate(r));
  }
  /** -1 left to 1 right, for 2D sounds. */
  @property
  get panStereo(): number {
    return this._panStereo;
  }
  set panStereo(p: number) {
    this._panStereo = p;
    this.eachPlaying((pb) => pb.setPan(p));
  }
  @property
  get mute(): boolean {
    return this._mute;
  }
  set mute(m: boolean) {
    this._mute = m;
    this.eachPlaying((p) => p.setVolume(this.gain()));
  }
  /** Whether `play()` repeats the clip. Applies from the next `play()`. */
  @property
  get loop(): boolean {
    return this._loop;
  }
  set loop(l: boolean) {
    this._loop = l;
  }

  /** Whether the clip from `play()` is playing (paused counts as not). */
  get isPlaying(): boolean {
    return !!this.main?.playing && !this.paused;
  }

  private gain(): number {
    return this._mute ? 0 : this._volume;
  }

  private eachPlaying(f: (p: Playback) => void): void {
    for (const p of this.all) f(p);
  }

  /** Where its sounds go: its emitter (made or remade when the 3D settings change) or bus. */
  private output(): Bus | Emitter | undefined {
    if (this.spatialBlend <= 0) return this.outputBus ?? undefined;
    const key = `${this.minDistance},${this.maxDistance},${this.outputBus?.id ?? ""}`;
    if (!this.emitter || key !== this.emitterKey) {
      // Remaking it stops what played on the old one, as moving the source would in Unity.
      this.emitter?.remove();
      this.all.clear();
      this.main = null;
      this.sentPosition = this.transform.position;
      this.emitter = audio.createEmitter(this.sentPosition, {
        minDistance: this.minDistance,
        maxDistance: this.maxDistance,
        bus: this.outputBus ?? undefined,
      });
      this.emitterKey = key;
    }
    return this.emitter;
  }

  private start(sound: Sound, options: PlayOptions): Playback {
    const p = sound.play({ ...options, on: this.output() });
    this.all.add(p);
    p.onEnded = () => this.all.delete(p);
    return p;
  }

  /** Plays `clip` from the start, replacing what `play()` played before. */
  play(): void {
    if (!this.clip) {
      console.warn(`AudioSource on ${this.gameObject.name} has no clip`);
      return;
    }
    this.main?.stop();
    this.paused = false;
    this.main = this.start(clipOf(this.clip), {
      volume: this.gain(),
      rate: this._pitch,
      pan: this._panStereo,
      loop: this._loop,
    });
  }

  /** Stops the clip from `play()` (one-shots carry on). */
  stop(): void {
    this.main?.stop();
    this.main = null;
    this.paused = false;
  }

  pause(): void {
    if (!this.main?.playing || this.paused) return;
    this.main.pause();
    this.paused = true;
  }

  unPause(): void {
    if (!this.paused || !this.main) return;
    this.main.resume();
    this.paused = false;
  }

  /** Plays `clip` once over whatever else plays (footsteps, hits), at `volumeScale` × `volume`. */
  playOneShot(clip: Sound | string, volumeScale = 1): void {
    this.start(clipOf(clip), { volume: this.gain() * volumeScale, rate: this._pitch, pan: this._panStereo });
  }

  /** Plays `clip` once at `position` in the world, from a temporary emitter. */
  static playClipAtPoint(clip: Sound | string, position: Vector3Like, volume = 1): Playback {
    const emitter = audio.createEmitter(position);
    const p = clipOf(clip).play({ volume, on: emitter });
    p.onEnded = () => emitter.remove();
    return p;
  }

  override onAwake(): void {
    if (this.playOnAwake && this.clip) this.play();
  }

  /** Keeps the emitter on the object (only sending real moves). */
  override onPreRender(): void {
    if (!this.emitter) return;
    const p = this.transform.position;
    const s = this.sentPosition;
    if (s && p.x === s.x && p.y === s.y && p.z === s.z) return;
    this.emitter.setPosition(p);
    this.sentPosition = p;
  }

  override onDestroy(): void {
    for (const p of this.all) p.stop();
    this.all.clear();
    this.main = null;
    this.emitter?.remove();
    this.emitter = null;
  }
}

let masterVolume = 1;

/**
 * Where the player hears from (Unity's `AudioListener`): put one on the camera's object.
 * With none, the listener stays at the origin facing -z.
 */
export class AudioListener extends Component {
  private sent: { x: number; y: number; z: number; rx: number; ry: number; rz: number; rw: number } | null = null;

  /** The volume of everything (linear). */
  static get volume(): number {
    return masterVolume;
  }
  static set volume(v: number) {
    masterVolume = v;
    audio.setMasterVolume(v);
  }

  override onAwake(): void {
    this.send();
  }

  override onPreRender(): void {
    this.send();
  }

  private send(): void {
    const p = this.transform.position;
    const r = this.transform.rotation;
    const s = this.sent;
    if (s && s.x === p.x && s.y === p.y && s.z === p.z && s.rx === r.x && s.ry === r.y && s.rz === r.z && s.rw === r.w) {
      return;
    }
    audio.setListener(p, r);
    this.sent = { x: p.x, y: p.y, z: p.z, rx: r.x, ry: r.y, rz: r.z, rw: r.w };
  }
}

/** Clears the shared clips. Only for tests. */
export function _resetAudioComponentsForTests() {
  clips.clear();
  masterVolume = 1;
}
