// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// What each emoji does: a heart blushes the room red with a heartbeat, the poop has a fart and
// a sad trombone, the diamond is tapped with a pickaxe until it cracks, the alien is beamed up,
// the zombie peeks round the curtain and screams, the pumpkin brings a haunted storm.
// `pose` names a signature in signatures.ts (or a base pose in poses.ts). Anything not listed
// gets a random entrance, pose and exit, and a random crowd.
export type Reaction = string;
export type Scene = string;

export interface Special {
  /** An entrance or exit by name. */
  enter?: string;
  exit?: string;
  /** A signature or base pose by name. */
  pose?: string;
  scene?: Scene;
  react?: Reaction;
  /** No random weather on top. */
  calm?: boolean;
  /** Extra seconds the pose carries on (it repeats). */
  extra?: number;
  /** Seconds into the pose that the crowd reacts (0.5 by default). */
  reactAt?: number;
}

const fashion: Special = { enter: "walk", exit: "walkaway", scene: "fashion", pose: "model", react: "applause" };

function makeSpecials(): Map<string, Special> {
  const m = new Map<string, Special>();
  // Faces
  m.set("1f602", { pose: "tears", react: "laugh" });
  m.set("1f60d", { pose: "heartpop", scene: "love", react: "aww" });
  m.set("1f60e", fashion);
  m.set("1f62d", { scene: "storm", pose: "sobbing", react: "aww" });
  m.set("1f621", { pose: "angry", react: "boo" });
  m.set("1f92c", { pose: "swearing", react: "gasp" });
  m.set("1f631", { pose: "scream", react: "gasp" });
  m.set("1f976", { exit: "plunge", pose: "cold", react: "aww" });
  m.set("1f92e", { pose: "sick", react: "gasp" });
  m.set("1f634", { scene: "night", pose: "sleeping", react: "aww", calm: true });
  m.set("1f914", { pose: "thinking" });
  m.set("1f92f", { pose: "mindblown", react: "gasp" });
  m.set("1f973", { scene: "lights", exit: "fireworks", pose: "pump", react: "party", extra: 5 });
  m.set("1f911", { pose: "money", react: "party" });
  // Characters
  m.set("1f608", { pose: "demon", react: "boo" });
  m.set("1f47b", { scene: "haunt", pose: "ghost", react: "gasp", calm: true });
  m.set("1f480", { scene: "storm", pose: "skull", react: "gasp" });
  m.set("1f921", { pose: "clown", react: "laugh" });
  m.set("1f47d", { pose: "alien", exit: "abduct", react: "gasp", calm: true });
  m.set("1f916", { pose: "robot" });
  m.set("1f4a9", { pose: "fart", react: "boo", reactAt: 3 });
  m.set("1f9df", { enter: "peek", pose: "zombie", react: "gasp", calm: true });
  m.set("1f9de", { pose: "genie" });
  m.set("1f9d9", { pose: "wizard" });
  m.set("1f9b8", { pose: "hero", react: "cheer" });
  m.set("1f47e", { pose: "invader" });
  // Animals
  m.set("1f435", { pose: "monkey", react: "laugh" });
  m.set("1f436", { pose: "dog", react: "aww" });
  m.set("1f431", { pose: "cat", react: "aww" });
  m.set("1f984", { scene: "lights", exit: "fireworks", pose: "unicorn", react: "party", extra: 5 });
  m.set("1f438", { pose: "frog" });
  m.set("1f437", { pose: "pig", react: "laugh" });
  m.set("1f989", { scene: "night", pose: "sway", react: "aww", extra: 2.5, calm: true });
  m.set("1f419", { pose: "octopus" });
  m.set("1f996", { pose: "trex", react: "gasp" });
  // Things
  m.set("1f52b", { pose: "pistol" });
  m.set("1f4a3", { exit: "explode", pose: "shake", react: "gasp" });
  m.set("1f525", { pose: "fire" });
  m.set("26a1", { pose: "zap", scene: "storm", react: "gasp", reactAt: 1.1 });
  m.set("2764", { enter: "fade", pose: "heartpop", scene: "love", react: "aww" });
  m.set("1f451", fashion);
  m.set("1f48e", { pose: "diamond", react: "applause", reactAt: 5.4 });
  m.set("1f3c6", { exit: "fireworks", pose: "pump", react: "party" });
  m.set("1f389", { enter: "pop", exit: "fireworks", pose: "party", react: "party", reactAt: 1.6 });
  m.set("1f388", { pose: "balloon", exit: "balloonaway" });
  m.set("1f383", { scene: "haunt", pose: "pumpkin", react: "gasp", extra: 2, calm: true });
  m.set("1f680", { exit: "launch", pose: "float", react: "party", calm: true });
  m.set("1f319", { scene: "night", pose: "float", react: "aww", extra: 2.5, calm: true });
  // Drawn in code
  m.set("googly", { pose: "watch" });
  m.set("eyes", { pose: "watch" });
  return m;
}

export const SPECIAL: Map<string, Special> = makeSpecials();

const CROWD_NAMES: string[] = ["applause", "cheer", "laugh", "aww", "gasp", "boo"];
const CROWD_WEIGHTS: number[] = [40, 20, 12, 10, 8, 10];

export function randomReaction(): Reaction {
  let total = 0;
  for (const w of CROWD_WEIGHTS) total += w;
  let r = Math.random() * total;
  for (let i = 0; i < CROWD_NAMES.length; i++) {
    r -= CROWD_WEIGHTS[i]!;
    if (r <= 0) return CROWD_NAMES[i]!;
  }
  return "applause";
}

/** Now and then the weather turns, whoever is on. */
export function randomScene(): Scene | undefined {
  const r = Math.random();
  return r < 0.07 ? "rain" : r < 0.11 ? "storm" : undefined;
}
