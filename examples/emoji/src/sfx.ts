// Sound effects: the WAV files in assets/sfx, made by tools/make_sfx.py.

export const NAMES: string[] = [
  "tick", "ding", "tada", "drumroll",
  "clap", "cheer", "boo", "aww", "gasp", "laugh",
  "fart", "wah", "slide", "slide_down", "kazoo", "pop",
  "heartbeat", "thunder", "rain", "splash",
  "rocket", "rumble", "splat", "crash", "skid",
  "scamper", "whoosh", "boing", "thud", "step",
  "zap", "freeze", "sparkle", "disco",
  "whistle", "boom", "crackle", "explosion", "fuse",
  "drag", "beep", "liftoff", "poof",
  "hoot", "crickets", "shutter", "runway",
  "dink", "crack", "tinkle", "bark", "meow", "ribbit", "oink", "roar", "monkey", "snore", "growl", "scream", "honk", "squeak",
  "bleep", "ufo", "servo", "sizzle", "squirt", "evil", "wail", "bones", "ting", "harp", "blip", "hum",
  "chatter", "sob", "gulp", "ooh",
  "witch", "organ",
];

/** A sound's name: the file `assets/sfx/<name>.wav`. */
export type Sfx = string;

const MASTER = 0.8;

/** Files that are not WAVs. */
function extension(name: string): string {
  return name === "liftoff" ? "mp3" : "wav";
}

/** Sounds that are cut off this many seconds in (the NASA clip goes on to a second announcement); 0: not cut. */
function cutAt(name: string): number {
  return name === "liftoff" ? 6 : 0;
}

/** A loaded sound (engine handles are boxed to live in a Map). */
class Loaded {
  sound: Sound;
  constructor(sound: Sound) {
    this.sound = sound;
  }
}

/** One playing copy of a sound. */
class Voice {
  playback: Playback;
  done: boolean = false;
  constructor(playback: Playback) {
    this.playback = playback;
  }
}

class SfxPlayer {
  private sounds: Map<string, Loaded> = new Map<string, Loaded>();
  private loops: Map<string, Voice> = new Map<string, Voice>();
  private active: Map<string, Voice[]> = new Map<string, Voice[]>();

  /** Loads every sound up front, so the first play doesn't stutter. */
  preload(): void {
    for (const n of NAMES) {
      if (!this.sounds.has(n)) this.sounds.set(n, new Loaded(audio.load(`assets/sfx/${n}.${extension(n)}`)));
    }
  }

  play(name: string, volume: number = 1, rate: number = 1): Playback {
    this.preload();
    const playback = this.sounds.get(name)!.sound.play({ volume: volume * MASTER, rate: rate });
    const cut = cutAt(name);
    if (cut > 0) setTimeout(() => playback.stop(0.5), (cut * 1000) / rate);
    // Remembered while it plays, so `stopPlaying` can end it early.
    let list = this.active.get(name);
    if (list === undefined) {
      list = [];
      this.active.set(name, list);
    }
    const kept: Voice[] = [];
    for (const v of list) if (!v.done) kept.push(v);
    const voice = new Voice(playback);
    kept.push(voice);
    this.active.set(name, kept);
    playback.onEnded(() => {
      voice.done = true;
    });
    return playback;
  }

  /** Ends every copy of a one-shot sound still playing, fading out over `fade` seconds. */
  stopPlaying(name: string, fade: number = 0.3): void {
    const list = this.active.get(name);
    if (list === undefined) return;
    for (const v of list) if (!v.done) v.playback.stop(fade);
  }

  /** Plays on repeat until `stop`. */
  loop(name: string, volume: number = 1): void {
    if (this.loops.has(name)) return;
    this.preload();
    this.loops.set(name, new Voice(this.sounds.get(name)!.sound.play({ volume: volume * MASTER, loop: true })));
  }

  stop(name: string, fade: number = 0.4): void {
    const v = this.loops.get(name);
    if (v !== undefined) v.playback.stop(fade);
    this.loops.delete(name);
  }

  stopAll(fade: number = 0.4): void {
    const names: string[] = [];
    for (const n of this.loops.keys()) names.push(n);
    for (const n of names) this.stop(n, fade);
  }
}

export const sfx = new SfxPlayer();
