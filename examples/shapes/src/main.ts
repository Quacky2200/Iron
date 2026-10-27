// SPDX-License-Identifier: Apache-2.0

// Shapes: Platypus's GDI-style 2D drawing in one place. Each shape is a LineRenderer2D (an
// outline, a fill, or both) with points from `Shapes`, labelled with a TextMesh. The stars
// show the two fill rules; the last shape turns to show outlines and fills follow their object.

const ink = Color.hex("#e8edf5");

/** What a shape is drawn with, besides its points. */
interface Look {
  width?: number;
  loop?: boolean;
  fillColor?: Color;
  fillRule?: string;
}

/** A shape with a label under it, `at` the centre of its cell. */
function shape(name: string, at: Vector3, points: Vector2[], look: Look): GameObject {
  const go = new GameObject(name, { position: at });
  const line = go.addComponent(LineRenderer2D);
  line.points = points;
  line.width = look.width !== undefined ? look.width : 0.06;
  line.color = ink;
  line.loop = look.loop !== undefined ? look.loop : true;
  if (look.fillColor !== undefined) line.fillColor = look.fillColor;
  if (look.fillRule !== undefined) line.fillRule = look.fillRule;
  const label = new GameObject(`${name} label`, { position: at.add(new Vector3(0, -1.25, 0)) });
  label.addComponent(TextMesh, { text: name, fontSize: 26, color: Color.hex("#9fb0c8") });
  return go;
}

/** A five-pointed star drawn in one stroke, so its outline crosses itself. */
function star(r: number): Vector2[] {
  const out: Vector2[] = [];
  for (let i = 0; i < 5; i++) out.push(new Vector2(Math.cos((90 + 144 * i) * (Math.PI / 180)) * r, Math.sin((90 + 144 * i) * (Math.PI / 180)) * r));
  return out;
}

class Spin extends Component {
  override onUpdate(dt: number): void {
    this.transform.rotate(new Vector3(0, 0, 40 * dt));
  }
}

class Gallery extends Component {
  override onAwake(): void {
    new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: Color.hex("#141922"),
    });
    const title = new GameObject("Title", { position: new Vector3(0, 4.1, 0) });
    title.addComponent(TextMesh, { text: "Shapes, fills and TextMesh", fontSize: 44, fontWeight: 700, color: ink });

    const row1 = 1.6;
    const row2 = -2.2;
    shape("rectangle", new Vector3(-6, row1, 0), Shapes.rectangle(-0.9, -0.6, 1.8, 1.2), { fillColor: Color.hex("#3a6fd8") });
    shape("rounded", new Vector3(-3, row1, 0), Shapes.roundedRectangle(-0.9, -0.6, 1.8, 1.2, 0.35), { fillColor: Color.hex("#2fa37a") });
    shape("circle", new Vector3(0, row1, 0), Shapes.circle(Vector2.zero, 0.8), { fillColor: new Color(0.9, 0.3, 0.3, 0.6) });
    shape("pie", new Vector3(3, row1, 0), Shapes.pie(Vector2.zero, 0.85, 0.85, 30, 270), { fillColor: Color.hex("#e8a33d") });
    shape("arc", new Vector3(6, row1, 0), Shapes.arc(Vector2.zero, 0.85, 0.6, 200, 220), { loop: false, width: 0.1 });
    shape("star, even-odd", new Vector3(-6, row2, 0), star(0.95), { fillColor: Color.hex("#7a5af0") });
    shape("star, non-zero", new Vector3(-3, row2, 0), star(0.95), { fillColor: Color.hex("#7a5af0"), fillRule: "nonzero" });
    shape("bezier", new Vector3(0, row2, 0), Shapes.bezier(new Vector2(-0.9, -0.6), new Vector2(-0.6, 1.2), new Vector2(0.6, -1.2), new Vector2(0.9, 0.6)), { loop: false, width: 0.1 });
    shape("closed curve", new Vector3(3, row2, 0), Shapes.closedCurve([new Vector2(-0.9, -0.5), new Vector2(0, 0.8), new Vector2(0.9, -0.5), new Vector2(0, -0.1)]), { fillColor: Color.hex("#2fa37a") });
    const hex = shape("turning hexagon", new Vector3(6, row2, 0), Shapes.regularPolygon(Vector2.zero, 0.85, 6), { fillColor: Color.hex("#3a6fd8"), width: 0.1 });
    hex.addComponent(Spin);

    const note = new GameObject("Note", { position: new Vector3(0, -4.3, 0) });
    note.addComponent(TextMesh, {
      text: "Outlines are capsules drawn by Rust; fills are tessellated when the points change.",
      fontSize: 22,
      color: Color.hex("#6f7f96"),
      maxWidth: 900,
    });
  }
}

register({ Gallery, Spin });
