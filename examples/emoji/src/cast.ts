// The cast: fifty emoji, each with a show of its own (see specials.ts and signatures.ts).
// Art is Noto Emoji (assets/emoji/<codepoint>.svg). Two performers are drawn in code instead,
// because their eyes follow the mouse.

export type PoseName = string;

export interface Star {
  /** The Noto file (its codepoint), or a stand-in id for the performers drawn in code. */
  code: string;
  name: string;
  /** Poses to pick from when it has no signature one. */
  poses: PoseName[];
  /** Drawn in code, with eyes that follow the mouse. */
  look?: string;
  /** The picture on the spinner wheel. */
  chip: string;
}

// [file, name]
const ROWS: string[][] = [
  // Faces
  ["1f602", "Tears of joy"],
  ["1f60d", "Heart eyes"],
  ["1f60e", "Cool"],
  ["1f62d", "Sobbing"],
  ["1f621", "Angry"],
  ["1f92c", "Swearing"],
  ["1f631", "Screaming"],
  ["1f976", "Freezing"],
  ["1f92e", "Sick"],
  ["1f634", "Sleeping"],
  ["1f914", "Thinking"],
  ["1f92f", "Mind blown"],
  ["1f973", "Party"],
  ["1f911", "Money mouth"],
  // Characters
  ["1f608", "Devil"],
  ["1f47b", "Ghost"],
  ["1f480", "Skull"],
  ["1f921", "Clown"],
  ["1f47d", "Alien"],
  ["1f916", "Robot"],
  ["1f4a9", "Poop"],
  ["1f9df", "Zombie"],
  ["1f9de", "Genie"],
  ["1f9d9", "Wizard"],
  ["1f9b8", "Superhero"],
  ["1f47e", "Space invader"],
  // Animals
  ["1f435", "Monkey"],
  ["1f436", "Dog"],
  ["1f431", "Cat"],
  ["1f984", "Unicorn"],
  ["1f438", "Frog"],
  ["1f437", "Pig"],
  ["1f989", "Owl"],
  ["1f419", "Octopus"],
  ["1f996", "T-rex"],
  // Things
  ["1f52b", "Water pistol"],
  ["1f4a3", "Bomb"],
  ["1f525", "Fire"],
  ["26a1", "Lightning"],
  ["2764", "Heart"],
  ["1f451", "Crown"],
  ["1f48e", "Diamond"],
  ["1f3c6", "Trophy"],
  ["1f389", "Party popper"],
  ["1f388", "Balloon"],
  ["1f383", "Pumpkin"],
  ["1f680", "Rocket"],
  ["1f319", "Moon"],
];

function makeCast(): Star[] {
  const out: Star[] = [];
  for (const row of ROWS) out.push({ code: row[0]!, name: row[1]!, poses: ["bow", "pump"], chip: row[0]! });
  out.push({ code: "eyes", name: "Eyes", poses: ["watch"], look: "eyes", chip: "1f440" });
  out.push({ code: "googly", name: "Googly", poses: ["watch", "bow"], look: "googly", chip: "1f92a" });
  return out;
}

export const CAST: Star[] = makeCast();
