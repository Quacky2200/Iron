// Project-wide graphics settings (Unity's `GraphicsSettings`). Iron has one render pipeline,
// as Unity 7 does (its URP, which takes over HDRP's features as HDRP and the Built-in
// pipeline go away); what it runs is set per quality level (`QualitySettings`: light shafts,
// HDR, additional lights and their shadows, auto exposure). What's left here is the
// project's light units, chosen in its `package.json`:
//
//   "iron": { "lightUnits": "physical" }
//
// `physical`: real-world units, as Unity 7 and HDRP use (the sun in lux, lamps in lumens,
// glowing surfaces and the sky in nits; a physical camera's EV100 for the manual exposure).
// `legacy` (the default): Iron's own units, as every example before had them.

/** How light is measured: real-world units, or Iron's own. */
export type LightUnits = "physical" | "legacy";

const info = (): { lightUnits?: unknown } => (globalThis as { __iron_info?: { app?: { lightUnits?: unknown } } }).__iron_info?.app ?? {};

export const GraphicsSettings = {
  /** The project's light units (`iron.lightUnits`). */
  get lightUnits(): LightUnits {
    return info().lightUnits === "physical" ? "physical" : "legacy";
  },
};
