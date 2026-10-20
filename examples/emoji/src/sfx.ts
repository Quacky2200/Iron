// Sound effects: the WAV files in assets/sfx, made by tools/make_sfx.py.
import { audio, type Playback, type Sound } from "@iron/engine";

export const NAMES = [
  "tick", "ding", "tada", "drumroll",
  "clap", "cheer", "boo", "aww", "gasp", "laugh",
  "fart", "wah", "slide", "slide_down", "kazoo", "pop",
  "heartbeat", "thunder", "rain", "splash",
  "rocket", "rumble", "splat", "crash", "skid",
  "scamper", "whoosh", "boing", "thud", "step",
  "zap", "freeze", "sparkle", "disco",
  "whistle", "boom", "crackle", "explosion", "fuse",
  "drag", "beep", "liftoff", "poof",
] as const;

export type Sfx = (typeof NAMES)[number];

const MASTER = 0.8;
/** Files that aren't WAVs. */
const FILE: Partial<Record<Sfx, string>> = { liftoff: "mp3" };
/** Sounds that are cut off this many seconds in (the NASA clip goes on to a second announcement). */
const CUT: Partial<Record<Sfx, number>> = { liftoff: 6 };
const sounds = new Map<Sfx, Sound>();
const loops = new Map<Sfx, Playback>();
const active = new Map<Sfx, Set<Playback>>();

export const sfx = {
  /** Loads every sound up front, so the first play doesn't stutter. */
  preload(): void {
    for (const n of NAMES) if (!sounds.has(n)) sounds.set(n, audio.load(`assets/sfx/${n}.${FILE[n] ?? "wav"}`));
  },

  play(name: Sfx, volume = 1, rate = 1): Playback {
    sfx.preload();
    const playback = sounds.get(name)!.play({ volume: volume * MASTER, rate });
    const cut = CUT[name];
    if (cut !== undefined) setTimeout(() => playback.stop(0.5), (cut * 1000) / rate);
    // Remembered while it plays, so `stopPlaying` can end it early.
    const set = active.get(name) ?? new Set<Playback>();
    active.set(name, set);
    set.add(playback);
    playback.onEnded = () => set.delete(playback);
    return playback;
  },

  /** Ends every copy of a one-shot sound still playing, fading out over `fade` seconds. */
  stopPlaying(name: Sfx, fade = 0.3): void {
    for (const p of active.get(name) ?? []) p.stop(fade);
  },

  /** Plays on repeat until `stop`. */
  loop(name: Sfx, volume = 1): void {
    if (loops.has(name)) return;
    sfx.preload();
    loops.set(name, sounds.get(name)!.play({ volume: volume * MASTER, loop: true }));
  },

  stop(name: Sfx, fade = 0.4): void {
    loops.get(name)?.stop(fade);
    loops.delete(name);
  },

  stopAll(fade = 0.4): void {
    for (const n of [...loops.keys()]) sfx.stop(n, fade);
  },
};
