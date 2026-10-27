// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Cornell box (Cornell University Program of Computer Graphics, 1984), built from
// primitives: a 2 m room with a red left wall, a green right wall, white floor, ceiling
// and back, a tall and a short block, and a light in the ceiling.
//
// Platypus has no global illumination or area lights yet, so the light is approximated: an
// emissive panel (glowing through bloom), a point light under it for the walls, a
// soft-shadowed directional light from above so the blocks cast their shadows, and bounce
// lights (soft point lights tinted by the walls and floor) faking the light GI would
// bounce, colour bleeding included. Real GI is in TODO.md (M7).

// The original's measured reflectances (linear).
const WHITE = new Color(0.73, 0.73, 0.73, 1);
const RED = new Color(0.63, 0.065, 0.05, 1);
const GREEN = new Color(0.14, 0.45, 0.091, 1);

function box(name: string, position: Vector3, size: Vector3, color: Color, yaw = 0, castShadows = true): GameObject {
  const go = new GameObject(name, { position, scale: size, rotation: Quaternion.euler(0, yaw, 0) });
  go.addComponent(MeshRenderer, {
    mesh: "cube",
    material: new Material(color, Color.black, 0.05),
    shadowCastingMode: castShadows ? "on" : "off",
  });
  return go;
}

/**
 * A bounce light: a soft point light just off a lit surface, in the surface's colour,
 * standing in for the light it reflects onto everything nearby (what GI would compute).
 */
function bounce(name: string, at: Vector3, surface: Color, light: Color, intensity: number, range: number): void {
  const go = new GameObject(name, { position: at });
  const tint = new Color(surface.r * light.r, surface.g * light.g, surface.b * light.b, 1);
  // Brightest channel to 1: the intensity sets how much it bounces.
  const peak = Math.max(tint.r, tint.g, tint.b, 1e-6);
  go.addComponent(Light, { type: "point", color: new Color(tint.r / peak, tint.g / peak, tint.b / peak, 1), intensity, range });
}

class CornellBox extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera", { position: new Vector3(0, 1, 3.9) });
    cam.addComponent(Camera, { fieldOfView: 39, backgroundColor: Color.black });
    RenderSettings.ambientLight = new Color(0.07, 0.07, 0.07);
    RenderSettings.bloom = { threshold: 1.2, intensity: 0.45, scatter: 0.55 };
    QualitySettings.shadowDistance = 6;

    // The room: thin boxes, 2 m inside, open at the front. Its shell casts no shadows (the
    // ceiling would shade everything from the light above); only the blocks do.
    const t = 0.05;
    box("Floor", new Vector3(0, -t / 2, 0), new Vector3(2 + 2 * t, t, 2), WHITE, 0, false);
    box("Ceiling", new Vector3(0, 2 + t / 2, 0), new Vector3(2 + 2 * t, t, 2), WHITE, 0, false);
    box("Back wall", new Vector3(0, 1, -1 - t / 2), new Vector3(2 + 2 * t, 2 + 2 * t, t), WHITE, 0, false);
    box("Left wall", new Vector3(-1 - t / 2, 1, 0), new Vector3(t, 2, 2), RED, 0, false);
    box("Right wall", new Vector3(1 + t / 2, 1, 0), new Vector3(t, 2, 2), GREEN, 0, false);

    // The blocks, turned about 17 degrees each way as in the original.
    box("Tall block", new Vector3(-0.33, 0.6, -0.28), new Vector3(0.6, 1.2, 0.6), WHITE, 17);
    box("Short block", new Vector3(0.34, 0.3, 0.3), new Vector3(0.6, 0.6, 0.6), WHITE, -17);

    // The light: a glowing panel just under the ceiling, a point light below it, and a
    // shadow-casting light from straight above.
    const panel = new GameObject("Light panel", { position: new Vector3(0, 1.995, 0), scale: new Vector3(0.48, 0.01, 0.38) });
    panel.addComponent(MeshRenderer, {
      mesh: "cube",
      material: new Material(Color.white, new Color(4, 3.6, 2.9, 1), 0, true),
    });
    const bulb = new GameObject("Point light", { position: new Vector3(0, 1.85, 0) });
    bulb.addComponent(Light, { type: "point", color: new Color(1, 0.9, 0.75, 1), intensity: 3.2, range: 4.5 });
    // Tilted a little towards the back and left, so the blocks' shadows show.
    const above = new GameObject("Shadow light", { rotation: Quaternion.euler(-72, 20, 0) });
    above.addComponent(Light, { type: "directional", intensity: 0.4, color: new Color(1, 0.93, 0.82, 1), shadows: "soft", shadowStrength: 0.8 });
    // Bounce lights: the red and green walls tint what faces them (the box's famous
    // colour bleeding), the lit floor lifts the undersides, and a dim fill from the open
    // front stands in for the rest of the bounced light.
    const warm = new Color(1, 0.9, 0.75, 1);
    bounce("Red bounce", new Vector3(-0.75, 0.9, 0.1), RED, warm, 1.6, 2.4);
    bounce("Green bounce", new Vector3(0.75, 0.9, 0.1), GREEN, warm, 1.6, 2.4);
    bounce("Floor bounce", new Vector3(0, 0.12, 0.2), WHITE, warm, 0.6, 2.2);
    const fill = new GameObject("Fill light", { position: new Vector3(0, 1, 2.6) });
    fill.addComponent(Light, { type: "point", color: new Color(1, 0.95, 0.9, 1), intensity: 0.9, range: 6 });
  }
}

register({ CornellBox });

function Caption() {
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, justifyContent: "flex-end", pointerEvents: "none" }}>
      <text style={{ color: "#8a8f99", fontSize: 12 }}>Cornell box — Cornell University Program of Computer Graphics</text>
    </box>
  );
}

createRoot().render(<Caption />);
