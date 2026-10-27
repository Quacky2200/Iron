// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The render pipeline's settings, after Unity 7: one pipeline (Unity's URP, which takes
//! over HDRP's features as HDRP and the Built-in pipeline go away), scaled by settings that
//! change with the quality level (Unity's URP asset per quality level). A phone's level drops
//! what a desktop's keeps; real-world light units are a project-wide choice of their own
//! (`WorldRenderer::physical_units`), not a pipeline.

/// What the renderer runs (Unity's URP asset). Everything is on by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineAsset {
    /// The light-shaft volume, per-pixel shafts and god rays (`VolumetricFog`).
    pub volumetric_fog: bool,
    /// How many lights besides the main (brightest directional) one a view gets: the nearest.
    pub max_additional_lights: usize,
    /// Whether spot and point lights cast shadows (the main light always may).
    pub additional_light_shadows: bool,
    /// Rendering in HDR: light past white is kept for bloom, exposure and tonemapping. Off,
    /// colours clip at white and those are skipped (a low-end level's choice).
    pub hdr: bool,
}

impl Default for PipelineAsset {
    fn default() -> Self {
        Self {
            volumetric_fog: true,
            max_additional_lights: usize::MAX,
            additional_light_shadows: true,
            hdr: true,
        }
    }
}
