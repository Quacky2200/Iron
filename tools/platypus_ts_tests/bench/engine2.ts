// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Compiled code calling the engine API beyond the scene (`platypus_engine_api`): Color with its
// statics and methods, Time and input properties, Gizmos with a camera, Screen and QualitySettings.

export function colors(): number {
  const c = Color.hex("#ff8000");
  const mid = Color.black.lerp(Color.white, 0.25);
  const t = new Color(0.5, 0.25, 1, 0.5).withAlpha(0.75);
  return Math.round(c.r * 10000) * 1000000 + Math.round(mid.r * 1000) * 1000 + Math.round(t.a * 100);
}

export function hexRoundTrip(): string {
  return Color.hex("#336699").toHex();
}

export function timing(): number {
  Time.timeScale = 0.5;
  const scaled = Time.deltaTime;
  const raw = Time.unscaledDeltaTime;
  Time.timeScale = 1;
  return scaled * 1000 + raw * 100 + Time.frameCount;
}

export function keys(): number {
  let n = 0;
  if (input.getKeyDown("Space")) n += 1;
  if (input.getKey("KeyV")) n += 10;
  if (input.getMouseButtonDown(0)) n += 100;
  return n + input.mousePosition.x * 1000;
}

export function display(): number {
  Screen.fullScreen = !Screen.fullScreen;
  QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
  return Screen.width * 10 + (Screen.fullScreen ? 1 : 0) + QualitySettings.vSyncCount * 100000;
}

export function drawCurve(points: number): number {
  const go = GameObject.create();
  const cam = Camera.attach(go);
  cam.orthographic = true;
  cam.orthographicSize = 5;
  cam.backgroundColor = Color.white;
  Gizmos.clear();
  Gizmos.color = Color.red;
  for (let i = 1; i <= points; i++) {
    Gizmos.drawLine(new Vector3(i - 1, 0, 0), new Vector3(i, i, 0));
  }
  Gizmos.present(cam);
  return cam.orthographicSize * (cam.orthographic ? 1 : 0);
}
