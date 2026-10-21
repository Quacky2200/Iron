// The sundial face: a garden sundial under a sun placed for today's date and the time. The
// dial is a real horizontal sundial for latitude 51.5° N: the gnomon's edge rises at the
// latitude, so it lies parallel to the Earth's axis, and the hour lines sit where
// tan θ = sin(latitude) · tan(hour angle). Its soft shadow (a directional light) falls on the
// hour. The sun is placed so noon is 12:00 on the clock; a real dial shows solar time, which
// the time zone, summer time and the equation of time move a little. The sky follows the sun's
// height, from day through dusk to night. At night there's no shadow to read, so it shows a day
// passing instead; D does that by day too (D again to stop).

import { Face, label, obj, two, under } from "./common.ts";

const LATITUDE = 51.5;
const DEG = Math.PI / 180;
const TOP = 1.0;
const RADIUS = 1.25;
const STONE = Material.create({ color: Color.hex("#a49c8e"), smoothness: 0.2 });
const BRONZE = Material.create({ color: Color.hex("#c49a5c"), smoothness: 0.45, metallic: 0.25 });
// The face isn't metallic: a metal mostly mirrors the sky, which no shadow darkens.
const DIAL = Material.create({ color: Color.hex("#c9a368"), smoothness: 0.25 });
const ENGRAVED = Material.create({ color: Color.hex("#3a2c18"), smoothness: 0.3 });
const GRASS = Material.create({ color: Color.hex("#4d7a3a"), smoothness: 0.1 });
const LEAVES = Material.create({ color: Color.hex("#2f5e2c"), smoothness: 0.15 });
const ROMAN = ["VI", "VII", "VIII", "IX", "X", "XI", "XII", "I", "II", "III", "IV", "V", "VI"];

/** The sun for latitude `LATITUDE` on day `day` of the year, `hours` after midnight (noon at 12). */
export function sunPosition(day: number, hours: number): { altitude: number; azimuth: number } {
  const phi = LATITUDE * DEG;
  const decl = -23.44 * DEG * Math.cos(((2 * Math.PI) / 365) * (day + 10));
  const h = (hours - 12) * 15 * DEG;
  const sinAlt = Math.sin(phi) * Math.sin(decl) + Math.cos(phi) * Math.cos(decl) * Math.cos(h);
  const altitude = Math.asin(sinAlt);
  // Azimuth from north, clockwise (east is 90°).
  const cosAz = (Math.sin(decl) - sinAlt * Math.sin(phi)) / (Math.cos(altitude) * Math.cos(phi));
  let azimuth = Math.acos(Math.max(-1, Math.min(1, cosAz)));
  if (h > 0) azimuth = 2 * Math.PI - azimuth;
  return { altitude, azimuth };
}

/** Towards the sun, in the world: north is -z, east is +x, up is +y. */
const towardSun = (altitude: number, azimuth: number) =>
  new Vector3(Math.sin(azimuth) * Math.cos(altitude), Math.sin(altitude), -Math.cos(azimuth) * Math.cos(altitude));

/** The hour line for `hour` on the clock: its direction on the dial from the centre. */
function hourLine(hour: number): Vector3 {
  const h = (hour - 12) * 15 * DEG;
  const theta = Math.abs(hour - 12) >= 6 ? Math.sign(hour - 12) * (Math.PI / 2) : Math.atan(Math.sin(LATITUDE * DEG) * Math.tan(h));
  // Noon points north; morning shadows fall west, afternoon ones east.
  return new Vector3(Math.sin(theta), 0, -Math.cos(theta));
}

/** The gnomon: a thin triangular plate on the noon line, its edge rising north at the latitude. */
function gnomon(length: number, thickness: number): Mesh {
  const rise = length * Math.tan(LATITUDE * DEG);
  const t = thickness / 2;
  // The profile in the y-z plane: the centre, the north foot, the top.
  const profile = [new Vector3(0, 0, 0), new Vector3(0, 0, -length), new Vector3(0, rise, -length)];
  const vertices: Vector3[] = [];
  const normals: Vector3[] = [];
  const triangles: number[] = [];
  const face = (corners: Vector3[], normal: Vector3) => {
    const base = vertices.length;
    for (const c of corners) {
      vertices.push(c);
      normals.push(normal);
    }
    for (let i = 1; i < corners.length - 1; i++) triangles.push(base, base + i, base + i + 1);
  };
  const side = (x: number) => profile.map((p) => new Vector3(x, p.y, p.z));
  const [l, r] = [side(-t), side(t)];
  face([r[0]!, r[1]!, r[2]!], new Vector3(1, 0, 0));
  face([l[0]!, l[2]!, l[1]!], new Vector3(-1, 0, 0));
  face([l[0]!, l[1]!, r[1]!, r[0]!], new Vector3(0, -1, 0));
  face([l[1]!, l[2]!, r[2]!, r[1]!], new Vector3(0, 0, -1));
  // The style: the sloping edge, facing up and south.
  const slope = new Vector3(0, length, rise).normalized;
  face([l[0]!, r[0]!, r[2]!, l[2]!], slope);
  return Mesh.create({ vertices, normals, triangles });
}

/** A primitive with a material, placed, rotated and scaled. */
function solid(name: string, mesh: "cube" | "cylinder" | "sphere" | "plane", material: Material, at: Vector3, scale: Vector3, rotation = Quaternion.identity): GameObject {
  const go = obj(name, at);
  go.transform.rotation = rotation;
  go.transform.localScale = scale;
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}

export class SundialFace extends Face {
  readonly name = "Sundial";
  readonly root: GameObject = new GameObject("Sundial face");
  private sun!: Light;
  private plaque!: TextMesh;
  private lapse = false;
  private lapseStart = 0;
  private wasNight = false;
  private shownPlaque = "";
  private saved: (() => void) | null = null;

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    const sun = obj("sun");
    this.sun = sun.addComponent(Light, { type: "directional", intensity: 1.6, shadows: "soft", shadowStrength: 0.85 });

    solid("lawn", "plane", GRASS, Vector3.zero, new Vector3(30, 1, 30));
    solid("pedestal", "cylinder", STONE, new Vector3(0, TOP / 2 - 0.05, 0), new Vector3(0.7, TOP / 2 - 0.05, 0.7));
    solid("pedestal cap", "cylinder", STONE, new Vector3(0, TOP - 0.04, 0), new Vector3(RADIUS * 2 + 0.12, 0.04, RADIUS * 2 + 0.12));
    solid("dial", "cylinder", DIAL, new Vector3(0, TOP + 0.02, 0), new Vector3(RADIUS * 2, 0.02, RADIUS * 2));
    const face = TOP + 0.045;

    // Hour lines and numerals, six in the morning round to six at night.
    for (let hour = 6; hour <= 18; hour++) {
      const dir = hourLine(hour);
      const inner = 0.22;
      const outer = RADIUS - 0.32;
      const mid = dir.scale((inner + outer) / 2);
      const yaw = Math.atan2(dir.x, -dir.z) / DEG;
      solid(`hour line ${hour}`, "cube", ENGRAVED, new Vector3(mid.x, face, mid.z), new Vector3(0.022, 0.006, outer - inner), Quaternion.euler(0, -yaw, 0));
      const n = label(`numeral ${hour}`, { text: ROMAN[hour - 6]!, fontSize: 30, fontWeight: 700, color: Color.hex("#2a1f10"), pixelsPerUnit: 260 }, new Vector3(0, 0, 0));
      const at = dir.scale(RADIUS - 0.17);
      n.transform.position = new Vector3(at.x, face + 0.002, at.z);
      // Flat on the dial, its top toward the edge, read from outside.
      n.transform.rotation = Quaternion.euler(-90, -yaw, 0);
    }
    const g = obj("gnomon", new Vector3(0, face, 0));
    g.addComponent(MeshRenderer, { mesh: gnomon(RADIUS - 0.2, 0.09), material: BRONZE });

    // The readout, engraved on the dial's south half (the shadow never falls there).
    this.plaque = label("plaque", { fontSize: 24, fontWeight: 700, color: Color.hex("#3a2c18"), pixelsPerUnit: 260, alignment: "center" }, new Vector3(0, face + 0.002, 0.62));
    this.plaque.transform.rotation = Quaternion.euler(-90, 0, 0);

    // Bushes round the lawn for the sun to cast shadows from.
    const bushes: [number, number, number][] = [[-4.4, -3.8, 0.9], [4.8, -4.6, 1.1], [-5.6, 0.6, 0.7], [5.4, 0.9, 0.8], [0.8, -7.0, 1.2]];
    bushes.forEach(([x, z, s], i) => solid(`bush ${i}`, "sphere", LEAVES, new Vector3(x, s * 0.4, z), new Vector3(s * 1.2, s * 0.9, s * 1.2)));
  }

  override show(camera: Camera): void {
    const t = camera.transform;
    const before = {
      ortho: camera.orthographic,
      fov: camera.fieldOfView,
      pos: t.position,
      rot: t.rotation,
      clear: camera.clearFlags,
      ambientMode: RenderSettings.ambientMode,
      ambient: RenderSettings.ambientLight,
    };
    // Ambient light from a dimmed sky colour (set each frame), so shadows keep their contrast.
    RenderSettings.ambientMode = "color";
    camera.orthographic = false;
    camera.fieldOfView = 42;
    camera.clearFlags = "skybox";
    // From the south-east and above, so the shadow isn't hidden behind the gnomon.
    t.position = new Vector3(2.6, 3.9, 4.0);
    t.rotation = Quaternion.lookRotation(new Vector3(0, TOP + 0.2, -0.8).sub(t.position));
    this.saved = () => {
      camera.orthographic = before.ortho;
      camera.fieldOfView = before.fov;
      camera.clearFlags = before.clear;
      t.position = before.pos;
      t.rotation = before.rot;
      RenderSettings.skybox = null;
      RenderSettings.ambientMode = before.ambientMode;
      RenderSettings.ambientLight = before.ambient;
    };
  }

  override hide(): void {
    this.saved?.();
    this.saved = null;
  }

  update(now: DateTime, t: number): void {
    const day = Math.floor((Date.UTC(now.year, now.month - 1, now.day) - Date.UTC(now.year, 0, 1)) / 86_400_000) + 1;
    if (input.getKeyDown("KeyD")) {
      this.lapse = !this.lapse;
      this.lapseStart = t;
    }
    const real = now.hour + now.minute / 60 + now.second / 3600;
    let hours = real;
    const night = Math.sin(sunPosition(day, real).altitude) <= 0;
    if (night && !this.wasNight) this.lapseStart = t;
    this.wasNight = night;
    if (this.lapse || night) {
      // Sunrise to sunset in about 24 seconds, over and over.
      const phi = LATITUDE * DEG;
      const decl = -23.44 * DEG * Math.cos(((2 * Math.PI) / 365) * (day + 10));
      const half = Math.acos(Math.max(-1, Math.min(1, -Math.tan(phi) * Math.tan(decl)))) / DEG / 15;
      hours = 12 - half - 0.3 + (((t - this.lapseStart) * 0.45) % (2 * half + 0.6));
    }
    const { altitude, azimuth } = sunPosition(day, hours);
    const up = Math.sin(altitude);
    const dir = towardSun(altitude, azimuth);

    // The light shines from the sun; low suns are warmer. A low sun barely lights a flat dial,
    // so it's turned up there to keep the shadow readable until it sets.
    this.sun.transform.rotation = Quaternion.lookRotation(dir.scale(-1));
    const day01 = Math.max(0, Math.min(1, up * 12));
    const warm = Math.max(0, Math.min(1, 1 - up * 2.5));
    this.sun.intensity = day01 * (1.6 + 2.2 * (1 - Math.min(1, up * 2)));
    this.sun.color = new Color(1, 0.95 - warm * 0.35, 0.88 - warm * 0.55, 1);

    // The sky: blue by day, orange low down, deep blue at night.
    const dark = Math.max(0, Math.min(1, -up * 6));
    const mixc = (a: Color, b: Color, k: number) => new Color(a.r + (b.r - a.r) * k, a.g + (b.g - a.g) * k, a.b + (b.b - a.b) * k, 1);
    const dayZenith = Color.hex("#3a78c9");
    const dayHorizon = Color.hex("#a9cdee");
    const duskHorizon = Color.hex("#f0a35e");
    const nightSky = Color.hex("#0b1428");
    const horizon = mixc(mixc(dayHorizon, duskHorizon, warm * day01 + (1 - day01) * (1 - dark)), nightSky, dark);
    const zenith = mixc(dayZenith, nightSky, Math.max(dark, 1 - day01) * 0.85);
    RenderSettings.skybox = { sunDirection: up > -0.05 ? dir : null, zenith, horizon, ground: Color.hex("#2b3a26"), sunColor: this.sun.color };
    const sky = mixc(zenith, horizon, 0.5);
    RenderSettings.ambientLight = new Color(sky.r * 0.32 + 0.03, sky.g * 0.32 + 0.03, sky.b * 0.32 + 0.04, 1);

    const hhmm = (h: number) => `${two(Math.floor(h) % 24)}:${two(Math.floor((h % 1) * 60))}`;
    const plaque = night
      ? `NIGHT ${hhmm(real)} · SHOWING ${hhmm(hours)}`
      : up > 0
        ? `${this.lapse ? "DAY-LAPSE " : ""}${hhmm(hours)} · SUN ${Math.round(altitude / DEG)}° UP`
        : `${this.lapse ? "DAY-LAPSE " : ""}${hhmm(hours)} · SUN DOWN`;
    if (plaque !== this.shownPlaque) {
      this.shownPlaque = plaque;
      this.plaque.text = plaque;
    }
  }
}
