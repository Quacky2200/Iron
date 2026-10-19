// Component properties and lookups for the Rust host: `@property` values read and written through
// the generated table, and `getComponent` / `addComponent` of classes of this program.
import { property } from "./property_stub";

let awakes = 0;
let log = "";

class Health extends Component {
  @property hp: number = 10;
  @property armor: number | null = null;
  @property alive: boolean = true;
  @property label: string = "hero";
  @property buddy: Health | null = null;
  hits: number = 0;

  onAwake(): void {
    awakes++;
  }

  hit(n: number): void {
    this.hp -= n;
    this.hits++;
  }
}

/// Reaches its sibling and adds another component when it starts.
class Brain extends Component {
  onStart(): void {
    const h = this.getComponent(Health);
    if (h) {
      h.hit(3);
      log += "found" + h.hp;
    }
    const extra = this.gameObject!.addComponent(Health, { hp: 99, label: "added" });
    log += ",added" + extra.hp + extra.label;
    if (this.getComponent(Brain) === this) log += ",self";
  }
}

/// Looks for a component its object does not have.
class Lonely extends Component {
  onStart(): void {
    const h = this.getComponent(Health);
    log += h ? "has" : "none";
  }
}

register(Health);
register(Brain);
register(Lonely);

export function awakeCount(): number {
  return awakes;
}

export function logText(): string {
  return log;
}
