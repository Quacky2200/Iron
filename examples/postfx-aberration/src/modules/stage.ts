// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

// The scene the post-processing examples share (each example keeps its own copy): a sunlit
// colonnade seen down its avenue, with three coloured balls to focus on, bright lamps far away
// (bokeh when blurred) and thin rails (fine detail for outlines and blur).

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material) {
  const go = new GameObject(name, { position: at, scale });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}

/** Sandstone flags: a 2 x 2 checker of two warm greys, so the floor has edges to blur and ink. */
function flags(): Texture {
  const n = 64;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const dark = (x < n / 2) !== (y < n / 2);
      const edge = x % (n / 2) < 2 || y % (n / 2) < 2;
      px.push(edge ? new Color(0.3, 0.27, 0.23, 1) : dark ? new Color(0.78, 0.7, 0.58, 1) : new Color(0.88, 0.8, 0.68, 1));
    }
  }
  return Texture.create(n, n, px, { filterMode: "trilinear", anisoLevel: 16 });
}

export interface Stage {
  camera: Camera;
  /** Near, middle and far ball. */
  balls: GameObject[];
}

export function buildStage(): Stage {
  QualitySettings.shadowDistance = 60;
  RenderSettings.ambientMode = "color";
  RenderSettings.ambientLight = new Color(0.34, 0.36, 0.42, 1);
  RenderSettings.skybox = {};
  RenderSettings.tonemapping = "aces";
  RenderSettings.postExposure = 0.2;

  const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.45, -0.55, -0.7).normalized) });
  sun.addComponent(Light, { type: "directional", color: new Color(1, 0.9, 0.74, 1), intensity: 1.6, shadows: "soft" });

  const floor = new GameObject("Floor", { position: new Vector3(0, 0, -30), scale: new Vector3(2, 1, 12) });
  floor.addComponent(MeshRenderer, {
    mesh: "plane",
    material: Material.create({ color: Color.white, mainTexture: flags(), mainTextureScale: new Vector2(10, 60), smoothness: 0.25 }),
  });

  const stone = Material.create({ color: new Color(0.86, 0.8, 0.7, 1), smoothness: 0.2 });
  const dark = Material.create({ color: new Color(0.45, 0.4, 0.34, 1), smoothness: 0.15 });
  for (let i = 0; i < 9; i++) {
    const z = 3 - i * 5.5;
    for (const x of [-4.2, 4.2]) {
      shape("Base", "cube", new Vector3(x, 0.2, z), new Vector3(1.3, 0.4, 1.3), dark);
      shape("Column", "cylinder", new Vector3(x, 2.6, z), new Vector3(0.55, 2.2, 0.55), stone);
      shape("Capital", "cube", new Vector3(x, 4.95, z), new Vector3(1.2, 0.3, 1.2), dark);
    }
    shape("Lintel", "cube", new Vector3(0, 5.35, z), new Vector3(9.8, 0.4, 0.9), stone);
  }
  const iron = Material.create({ color: new Color(0.08, 0.08, 0.09, 1), metallic: 0.8, smoothness: 0.5 });
  for (const x of [-3.2, 3.2]) {
    shape("Rail", "cube", new Vector3(x, 1.0, -8), new Vector3(0.06, 0.06, 16), iron);
    for (let k = 0; k < 14; k++) shape("Baluster", "cylinder", new Vector3(x, 0.5, 2 - k * 1.2), new Vector3(0.05, 0.5, 0.05), iron);
  }

  const colors = [new Color(0.95, 0.2, 0.12, 1), new Color(0.15, 0.7, 0.3, 1), new Color(0.2, 0.4, 0.95, 1)];
  const at = [new Vector3(-1.0, 0.7, 4.0), new Vector3(0.6, 0.7, -2.5), new Vector3(-0.8, 0.7, -12.0)];
  const balls: GameObject[] = [];
  for (let i = 0; i < 3; i++) {
    balls.push(shape(`Ball ${i}`, "sphere", at[i]!, Vector3.one.scale(1.4), Material.create({ color: colors[i]!, metallic: 0.2, smoothness: 0.85 })));
  }
  const glow = Material.create({ color: Color.black, emission: new Color(6, 4.6, 2.8, 1), unlit: true });
  for (let i = 0; i < 6; i++) {
    for (const x of [-2.4, 2.4]) {
      const lamp = shape("Lamp", "sphere", new Vector3(x, 2.3 + (i % 2) * 0.6, -16 - i * 4), Vector3.one.scale(0.22), glow);
      lamp.getComponent(MeshRenderer)!.shadowCastingMode = "off";
    }
  }

  const eye = new GameObject("Main Camera", { position: new Vector3(0, 1.7, 9) });
  const camera = eye.addComponent(Camera, { fieldOfView: 50, farClipPlane: 200, clearFlags: "skybox" });
  return { camera, balls };
}

/** A slow sway down the avenue. */
export function sway(camera: Camera, t: number): void {
  const cam = camera.transform;
  cam.position = new Vector3(Math.sin(t * 0.2) * 1.2, 1.7 + Math.sin(t * 0.13) * 0.15, 9);
  cam.rotation = Quaternion.lookRotation(new Vector3(0, 1.6, -20).sub(cam.position));
}
