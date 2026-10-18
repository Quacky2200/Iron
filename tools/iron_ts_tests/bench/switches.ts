// switch (numbers, enums, string unions, fall-through, default in the middle) and enums.
export enum Dir {
  Up,
  Down,
  Left = 5,
  Right,
}

export const enum Flag {
  A = 1,
  B = A << 1,
  C = B << 1,
}

export enum Mood {
  Happy = "happy",
  Sad = "sad",
}

export type Kind = "a" | "b" | "c" | "d";

export interface Item {
  v: number;
}

export function dirCode(d: Dir): number {
  switch (d) {
    case Dir.Up:
      return 10;
    case Dir.Down:
      return 20;
    case Dir.Left:
    case Dir.Right:
      return 30;
    default:
      return -1;
  }
}

/** The enum values themselves: Up 0, Down 1, Left 5, Right 6. */
export function dirValues(): number {
  return Dir.Up * 1000 + Dir.Down * 100 + Dir.Left * 10 + Dir.Right;
}

/** Members that refer to earlier members: A 1, B 2, C 4. */
export function flags(): number {
  const both = Flag.A | Flag.C;
  return both * 100 + Flag.B;
}

export function turn(d: Dir): Dir {
  return d === Dir.Up ? Dir.Right : Dir.Up;
}

export function dirOf(n: number): Dir {
  return n > 2 ? Dir.Left : Dir.Down;
}

/** Fall-through and a `default` in the middle: it runs when nothing matches and falls into case 4. */
export function fallThrough(n: number): number {
  let r = 0;
  switch (n) {
    case 1:
      r += 1;
    case 2:
      r += 10;
      break;
    case 3:
      r += 100;
    default:
      r += 1000;
    case 4:
      r += 10000;
  }
  return r;
}

export function kindCode(k: Kind): number {
  switch (k) {
    case "a":
      return 1;
    case "b":
    case "c":
      return 2;
    default:
      return 0;
  }
}

export function kindDemo(): number {
  const ks: Kind[] = ["a", "b", "c", "d"];
  let s = 0;
  for (const k of ks) s = s * 10 + kindCode(k);
  return s;
}

export function moodDemo(): number {
  const moods: Mood[] = [Mood.Happy, Mood.Sad];
  let s = 0;
  for (const m of moods) {
    switch (m) {
      case Mood.Happy:
        s += 1;
        break;
      case Mood.Sad:
        s += 20;
        break;
    }
  }
  return s * 100 + (Mood.Happy === "happy" ? 1 : 0);
}

/** `continue` inside a switch continues the loop; `break` only leaves the switch. */
export function loopWithSwitch(n: number): number {
  let s = 0;
  for (let i = 0; i < n; i++) {
    switch (i % 3) {
      case 0:
        continue;
      case 1:
        s += 1;
        break;
      default:
        s += 10;
    }
    s += 100;
  }
  return s;
}

/** A switch without a match and without a default does nothing. */
export function noMatch(n: number): number {
  let r = 7;
  switch (n) {
    case 1:
      r = 1;
      break;
  }
  return r;
}

/** The discriminant and the case expressions allocate, evaluated once and in order. */
function make(v: number): Item {
  return { v };
}

export function allocatingSwitch(n: number): number {
  let total = 0;
  for (let i = 0; i < n; i++) {
    switch (make(i % 4).v) {
      case make(1).v:
        total += 1;
        break;
      case make(2).v:
        total += 20;
      case make(3).v:
        total += 300;
        break;
      default:
        total += 4000;
    }
  }
  return total;
}

/** `let` in a case is visible to the later cases of the same switch. */
export function sharedScope(n: number): number {
  switch (n) {
    case 1: {
      const x = 5;
      return x;
    }
    case 2: {
      const x = 6;
      return x * 2;
    }
  }
  return 0;
}

/** A labelled switch, left from inside a nested loop. */
export function breakOutOfSwitchLabel(): number {
  let s = 0;
  pick: switch (3) {
    case 3:
      for (let i = 0; i < 10; i++) {
        s += i;
        if (i === 4) break pick;
      }
      s += 1000;
  }
  return s;
}
