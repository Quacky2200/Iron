// Emoji with their own show: a heart blushes the room red with a heartbeat, the poop has a fart
// and a sad trombone, the party popper has a kazoo and streamers, and so on. Everyone else gets
// a random entrance, pose and exit, and a random crowd.
import type { PoseName } from "./cast.ts";

export type Reaction = "applause" | "cheer" | "boo" | "gasp" | "aww" | "laugh" | "none";
export type Scene = "rain" | "storm" | "love" | "lights" | "ball";

export interface Special {
  /** An entrance or exit by name. */
  enter?: string;
  exit?: string;
  pose?: PoseName;
  scene?: Scene;
  react?: Reaction;
  /** No random weather on top. */
  calm?: boolean;
  /** Extra seconds the pose carries on (it repeats). */
  extra?: number;
  /** Seconds into the pose that the crowd reacts (0.5 by default). */
  reactAt?: number;
}

const love: Special = { pose: "beat", scene: "love", react: "aww" };

export const SPECIAL: Record<string, Special> = {
  "2764": { ...love, enter: "fade" },
  "1f970": love,
  "1f60d": love,
  "1f618": love,
  "1f4a9": { pose: "fart", react: "boo", reactAt: 3 },
  "1f389": { enter: "pop", exit: "fireworks", pose: "party", react: "cheer", reactAt: 1.6 },
  "1f973": { scene: "lights", exit: "fireworks", pose: "pump", react: "cheer", extra: 5 },
  "1f984": { scene: "lights", exit: "fireworks", pose: "spin", react: "cheer", extra: 5 },
  "1f60e": { scene: "ball", pose: "bow", react: "cheer", extra: 5 },
  "1f929": { scene: "ball", pose: "pump", react: "cheer", extra: 5 },
  "26a1": { pose: "zap", scene: "storm", react: "gasp", reactAt: 1.1 },
  "1f622": { scene: "rain", pose: "sway", react: "aww" },
  "1f62d": { scene: "storm", pose: "shake", react: "aww" },
  "1f976": { exit: "plunge", pose: "shake", react: "aww" },
  "1f680": { exit: "launch", pose: "float", react: "cheer", calm: true },
  "1f480": { scene: "storm", react: "gasp" },
  "1f47b": { scene: "storm", react: "gasp" },
  "1f921": { react: "laugh" },
  "1f3c6": { exit: "fireworks", pose: "pump", react: "cheer" },
  "1f4af": { exit: "fireworks", pose: "pump", react: "cheer" },
  "2b50": { exit: "fireworks", pose: "spin", react: "cheer" },
  "1f4a3": { exit: "explode", pose: "shake", react: "gasp" },
  "1f4a5": { exit: "explode", pose: "shake", react: "gasp" },
  "1f608": { react: "boo" },
  "1f47f": { react: "boo" },
  "1f479": { react: "boo" },
  "1f635": { react: "laugh" },
  "1f92a": { react: "laugh" },
  "1f47d": { react: "gasp" },
  "1f631": { react: "gasp" },
  "1f634": { react: "aww" },
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
