//! Render pipelines, after Unity's: a project picks one (`iron.renderPipeline` in its
//! manifest), and it decides which features the renderer runs.
//!
//! - **Built-in** (the default): everything Iron has, in its own (arbitrary) light units.
//! - **Universal** (URP): for phones and tablets as well as desktops. Forward lighting with a
//!   cap on lights, no light-shaft volume or per-pixel god rays, no auto exposure (Unity's
//!   URP has neither), the rest of the post chain kept.
//! - **High Definition** (HDRP): everything, for high-end desktops and consoles; it's where
//!   real-world light units go.

/// Which pipeline a project renders with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderPipeline {
    #[default]
    BuiltIn,
    Universal,
    HighDefinition,
}

/// What a pipeline does and doesn't run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineFeatures {
    /// The light-shaft volume, per-pixel shafts and god rays (`VolumetricFog`).
    pub volumetric_fog: bool,
    /// `Grading::auto_exposure` (the manual exposure always works).
    pub auto_exposure: bool,
    /// How many lights besides the main (brightest directional) one a view gets: the nearest.
    pub max_additional_lights: usize,
}

impl RenderPipeline {
    /// From the manifest's name for it (Unity's short names too); None if it's not one.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "builtin" | "built-in" => Some(Self::BuiltIn),
            "universal" | "urp" => Some(Self::Universal),
            "hd" | "hdrp" | "high-definition" | "highdefinition" => Some(Self::HighDefinition),
            _ => None,
        }
    }

    /// The name scripts see (`GraphicsSettings.renderPipeline`).
    pub fn name(self) -> &'static str {
        match self {
            Self::BuiltIn => "builtin",
            Self::Universal => "universal",
            Self::HighDefinition => "hd",
        }
    }

    pub fn features(self) -> PipelineFeatures {
        match self {
            Self::BuiltIn | Self::HighDefinition => PipelineFeatures {
                volumetric_fog: true,
                auto_exposure: true,
                max_additional_lights: usize::MAX,
            },
            // URP's mobile default: 32 visible additional lights.
            Self::Universal => PipelineFeatures {
                volumetric_fog: false,
                auto_exposure: false,
                max_additional_lights: 32,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_as_unitys_do() {
        assert_eq!(
            RenderPipeline::parse("URP"),
            Some(RenderPipeline::Universal)
        );
        assert_eq!(
            RenderPipeline::parse("universal"),
            Some(RenderPipeline::Universal)
        );
        assert_eq!(
            RenderPipeline::parse("HDRP"),
            Some(RenderPipeline::HighDefinition)
        );
        assert_eq!(
            RenderPipeline::parse("hd"),
            Some(RenderPipeline::HighDefinition)
        );
        assert_eq!(
            RenderPipeline::parse("Built-in"),
            Some(RenderPipeline::BuiltIn)
        );
        assert_eq!(RenderPipeline::parse("lwrp"), None);
        assert!(!RenderPipeline::Universal.features().volumetric_fog);
        assert!(RenderPipeline::BuiltIn.features().auto_exposure);
    }
}
