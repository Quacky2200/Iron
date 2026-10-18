// `@property` fields: compile-time metadata, read by the host through a generated table.
import { property } from "./property_stub";

export class Player {
  @property speed = 5;
  @property alive = true;
  @property target: Player | null = null;
  @property label: number | null = null;
  hidden = 3;
}

export class Hero extends Player {
  @property power = 9;
  hiddenToo = 1;
}

export function makePlayer(): Player {
  return new Player();
}

export function makeHero(): Hero {
  return new Hero();
}

/** What the fields add up to: speed * 100 + alive * 10 + hidden. */
export function score(p: Player): number {
  return p.speed * 100 + (p.alive ? 10 : 0) + p.hidden;
}

export function heroPower(h: Hero): number {
  return h.power;
}
