// The TypeScript `@iron/engine` shapes the examples use, over the native API: object-valued
// namespace properties and `new GameObject(name, { .. })` option keys. The Rust test
// (tests/all/facade.rs) reads the same state back.

export function skyAndBloom(): string {
  RenderSettings.skybox = { sunDirection: new Vector3(0.5, 0.45, 0.4), zenith: new Color(1, 0, 0, 1), sunSize: 2 };
  const a = `${RenderSettings.skyboxEnabled}|${RenderSettings.skyZenith.r}|${RenderSettings.skySunSize}|${RenderSettings.skyHasSun}`;
  RenderSettings.skybox = { sunDirection: null };
  const b = `${RenderSettings.skyZenith.r === 1}|${RenderSettings.skyHasSun}`;
  RenderSettings.bloom = { threshold: 1.2, intensity: 0.45, scatter: 0.55 };
  const c = `${RenderSettings.bloomEnabled}|${RenderSettings.bloomThreshold}|${RenderSettings.bloomScatter}`;
  RenderSettings.bloom = null;
  RenderSettings.skybox = null;
  return `${a}|${b}|${c}|${RenderSettings.bloomEnabled}|${RenderSettings.skyboxEnabled}`;
}

export function gameObjectOptions(): string {
  const parent = new GameObject("P", { position: new Vector3(1, 2, 3), scale: new Vector3(2, 2, 2) });
  const child = new GameObject("C", { position: new Vector3(0, 1, 0), parent, layer: 3, active: false });
  const top = new GameObject("T", { parent: null });
  const w = child.transform.position;
  return `${child.parent === parent}|${top.parent === null}|${child.layer}|${child.enabled}|${child.transform.localPosition.y}|${w.x}|${w.y}|${w.z}`;
}
