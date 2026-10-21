// What each emoji does: a heart blushes the room red with a heartbeat, the poop has a fart and
// a sad trombone, the diamond is tapped with a pickaxe until it cracks, the alien is beamed up,
// the zombie peeks round the curtain and screams, the pumpkin brings a haunted storm.
// `pose` names a signature in signatures.ts (or a base pose in poses.ts). Anything not listed
// gets a random entrance, pose and exit, and a random crowd.
export type Reaction = "applause" | "cheer" | "party" | "boo" | "gasp" | "aww" | "laugh" | "none";
export type Scene = "rain" | "storm" | "love" | "lights" | "ball" | "night" | "fashion" | "haunt";

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

export const SPECIAL: Record<string, Special> = {
  // Faces
  "1f602": { pose: "tears", react: "laugh" },
  "1f60d": { pose: "heartpop", scene: "love", react: "aww" },
  "1f60e": fashion,
  "1f62d": { scene: "storm", pose: "sobbing", react: "aww" },
  "1f621": { pose: "angry", react: "boo" },
  "1f92c": { pose: "swearing", react: "gasp" },
  "1f631": { pose: "scream", react: "gasp" },
  "1f976": { exit: "plunge", pose: "cold", react: "aww" },
  "1f92e": { pose: "sick", react: "gasp" },
  "1f634": { scene: "night", pose: "sleeping", react: "aww", calm: true },
  "1f914": { pose: "thinking" },
  "1f92f": { pose: "mindblown", react: "gasp" },
  "1f973": { scene: "lights", exit: "fireworks", pose: "pump", react: "party", extra: 5 },
  "1f911": { pose: "money", react: "party" },
  // Characters
  "1f608": { pose: "demon", react: "boo" },
  "1f47b": { scene: "haunt", pose: "ghost", react: "gasp", calm: true },
  "1f480": { scene: "storm", pose: "skull", react: "gasp" },
  "1f921": { pose: "clown", react: "laugh" },
  "1f47d": { pose: "alien", exit: "abduct", react: "gasp", calm: true },
  "1f916": { pose: "robot" },
  "1f4a9": { pose: "fart", react: "boo", reactAt: 3 },
  "1f9df": { enter: "peek", pose: "zombie", react: "gasp", calm: true },
  "1f9de": { pose: "genie" },
  "1f9d9": { pose: "wizard" },
  "1f9b8": { pose: "hero", react: "cheer" },
  "1f47e": { pose: "invader" },
  // Animals
  "1f435": { pose: "monkey", react: "laugh" },
  "1f436": { pose: "dog", react: "aww" },
  "1f431": { pose: "cat", react: "aww" },
  "1f984": { scene: "lights", exit: "fireworks", pose: "unicorn", react: "party", extra: 5 },
  "1f438": { pose: "frog" },
  "1f437": { pose: "pig", react: "laugh" },
  "1f989": { scene: "night", pose: "sway", react: "aww", extra: 2.5, calm: true },
  "1f419": { pose: "octopus" },
  "1f996": { pose: "trex", react: "gasp" },
  // Things
  "1f52b": { pose: "pistol" },
  "1f4a3": { exit: "explode", pose: "shake", react: "gasp" },
  "1f525": { pose: "fire" },
  "26a1": { pose: "zap", scene: "storm", react: "gasp", reactAt: 1.1 },
  "2764": { enter: "fade", pose: "heartpop", scene: "love", react: "aww" },
  "1f451": fashion,
  "1f48e": { pose: "diamond", react: "applause", reactAt: 5.4 },
  "1f3c6": { exit: "fireworks", pose: "pump", react: "party" },
  "1f389": { enter: "pop", exit: "fireworks", pose: "party", react: "party", reactAt: 1.6 },
  "1f388": { pose: "balloon", exit: "balloonaway" },
  "1f383": { scene: "haunt", pose: "pumpkin", react: "gasp", extra: 2, calm: true },
  "1f680": { exit: "launch", pose: "float", react: "party", calm: true },
  "1f319": { scene: "night", pose: "float", react: "aww", extra: 2.5, calm: true },
  // Drawn in code
  googly: { pose: "watch" },
  eyes: { pose: "watch" },
};

const CROWD: [Reaction, number][] = [["applause", 40], ["cheer", 20], ["laugh", 12], ["aww", 10], ["gasp", 8], ["boo", 10]];

export function randomReaction(): Reaction {
  let r = Math.random() * CROWD.reduce((n, [, w]) => n + w, 0);
  for (const [name, w] of CROWD) {
    r -= w;
    if (r <= 0) return name;
  }
  return "applause";
}

/** Now and then the weather turns, whoever is on. */
export function randomScene(): Scene | undefined {
  const r = Math.random();
  return r < 0.07 ? "rain" : r < 0.11 ? "storm" : undefined;
}
