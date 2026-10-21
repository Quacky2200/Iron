// The project's render pipeline (Unity's `GraphicsSettings.currentRenderPipeline`), chosen
// in its `package.json`:
//
//   "iron": { "renderPipeline": "universal" }
//
// `builtin` (the default) runs everything Iron has, in its own light units. `universal`
// (URP) is for phones and tablets as well as desktops: forward lighting, the main light and
// up to 32 others, no light shafts and no auto exposure. `hd` (HDRP) runs everything, and is
// where real-world light units go.

/** The render pipelines, as Unity names them: Built-in, Universal (URP), High Definition (HDRP). */
export type RenderPipelineName = "builtin" | "universal" | "hd";

const info = (): { renderPipeline?: unknown } =>
  (globalThis as { __iron_info?: { app?: { renderPipeline?: unknown } } }).__iron_info?.app ?? {};

export const GraphicsSettings = {
  /** The pipeline the project renders with. */
  get renderPipeline(): RenderPipelineName {
    const p = info().renderPipeline;
    return p === "universal" || p === "hd" ? p : "builtin";
  },
  /** Whether light shafts and god rays run (`RenderSettings.volumetricFog`): not in URP. */
  get supportsVolumetricFog(): boolean {
    return this.renderPipeline !== "universal";
  },
  /** Whether auto exposure runs (`RenderSettings.autoExposure`): not in URP. */
  get supportsAutoExposure(): boolean {
    return this.renderPipeline !== "universal";
  },
  /** How many lights besides the main one a camera draws (the nearest): 32 in URP. */
  get maxAdditionalLights(): number {
    return this.renderPipeline === "universal" ? 32 : Number.POSITIVE_INFINITY;
  },
};
