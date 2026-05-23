#[cfg(target_os = "android")]
use ash::vk;

pub(crate) const CAMERA_SHADER_FLAG_RAW_FEED: u32 = 1 << 13;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION: u32 = 1 << 14;
pub(crate) const CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA: u32 = 1 << 15;
pub(crate) const CAMERA_SHADER_FLAG_PROJECTION_BORDER_SOLID_RED: u32 = 1 << 16;
pub(crate) const CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC: u32 = 1 << 23;
pub(crate) const CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING: u32 = 1 << 24;
const CAMERA_SHADER_EFFECT_RAW_PROJECTION_BLUR: f32 = 5.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraFeedPipelineMode {
    #[default]
    ProjectedFeed,
    RawFeed,
}

impl CameraFeedPipelineMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "projected-feed" | "projected" | "projection" | "adjusted" | "default" => {
                Some(Self::ProjectedFeed)
            }
            "raw-feed" | "raw" | "camera-raw" | "feed-raw" => Some(Self::RawFeed),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::ProjectedFeed => "projected-feed",
            Self::RawFeed => "raw-feed",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::ProjectedFeed => 0,
            Self::RawFeed => CAMERA_SHADER_FLAG_RAW_FEED,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraProjectionEffectMode {
    #[default]
    BorderComposite,
    RawProjection,
    ProjectionAreaDiagnostic,
    DisplayEyeUvFiducial,
    ProjectionContentUvFiducial,
    SourceSamplingWitness,
    FullFrameStimulusSurfaceMapping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraProcessingLayer {
    #[default]
    Raw,
    Blur,
}

impl CameraProcessingLayer {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "blur" => Some(Self::Blur),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Blur => "blur",
        }
    }

    pub(crate) const fn diagnostic_shader_code(self) -> f32 {
        match self {
            Self::Raw => 0.0,
            Self::Blur => CAMERA_SHADER_EFFECT_RAW_PROJECTION_BLUR,
        }
    }

    pub(crate) const fn requires_full_projection_pipeline(self) -> bool {
        matches!(self, Self::Blur)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraProjectionBorderPolicy {
    #[default]
    SolidRed,
    PassthroughUnderlay,
}

impl CameraProjectionBorderPolicy {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "solid-red" => Some(Self::SolidRed),
            "passthrough-underlay" => Some(Self::PassthroughUnderlay),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-red",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    pub(crate) const fn shared_fill_policy_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-color",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::SolidRed => CAMERA_SHADER_FLAG_PROJECTION_BORDER_SOLID_RED,
            Self::PassthroughUnderlay => CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA,
        }
    }

    pub(crate) const fn requires_full_projection_pipeline(self) -> bool {
        true
    }

    pub(crate) const fn uses_passthrough_underlay_alpha(self) -> bool {
        matches!(self, Self::PassthroughUnderlay)
    }
}

impl CameraProjectionEffectMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "border-composite" => Some(Self::BorderComposite),
            "raw-projection" => Some(Self::RawProjection),
            "projection-area-diagnostic" => Some(Self::ProjectionAreaDiagnostic),
            "display-eye-uv-fiducial" => Some(Self::DisplayEyeUvFiducial),
            "projection-content-uv-fiducial" => Some(Self::ProjectionContentUvFiducial),
            "source-sampling-witness" => Some(Self::SourceSamplingWitness),
            "full-frame-stimulus-surface-mapping" => Some(Self::FullFrameStimulusSurfaceMapping),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::BorderComposite => "border-composite",
            Self::RawProjection => "raw-projection",
            Self::ProjectionAreaDiagnostic => "projection-area-diagnostic",
            Self::DisplayEyeUvFiducial => "display-eye-uv-fiducial",
            Self::ProjectionContentUvFiducial => "projection-content-uv-fiducial",
            Self::SourceSamplingWitness => "source-sampling-witness",
            Self::FullFrameStimulusSurfaceMapping => "full-frame-stimulus-surface-mapping",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::BorderComposite => 0,
            Self::RawProjection => CAMERA_SHADER_FLAG_RAW_PROJECTION,
            Self::ProjectionAreaDiagnostic => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
            }
            Self::DisplayEyeUvFiducial => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
            }
            Self::ProjectionContentUvFiducial => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
                    | CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
            }
            Self::SourceSamplingWitness => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
                    | CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
            }
            Self::FullFrameStimulusSurfaceMapping => CAMERA_SHADER_FLAG_RAW_PROJECTION,
        }
    }

    pub(crate) const fn diagnostic_shader_code(self) -> f32 {
        match self {
            Self::DisplayEyeUvFiducial => 1.0,
            Self::ProjectionContentUvFiducial => 2.0,
            Self::SourceSamplingWitness => 3.0,
            Self::FullFrameStimulusSurfaceMapping => 4.0,
            _ => 0.0,
        }
    }

    pub(crate) const fn is_uv_fiducial(self) -> bool {
        matches!(
            self,
            Self::DisplayEyeUvFiducial
                | Self::ProjectionContentUvFiducial
                | Self::SourceSamplingWitness
        )
    }

    pub(crate) const fn uses_raw_projection_pipeline(self) -> bool {
        matches!(
            self,
            Self::RawProjection
                | Self::ProjectionAreaDiagnostic
                | Self::DisplayEyeUvFiducial
                | Self::ProjectionContentUvFiducial
                | Self::SourceSamplingWitness
                | Self::FullFrameStimulusSurfaceMapping
        )
    }

    pub(crate) const fn uses_projection_border_policy(self) -> bool {
        matches!(
            self,
            Self::RawProjection | Self::FullFrameStimulusSurfaceMapping
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OpenXrColorFormatMode {
    #[default]
    Rgba8Srgb,
    Rgba8Unorm,
}

impl OpenXrColorFormatMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "rgba8-srgb" | "r8g8b8a8-srgb" | "srgb" | "default" => Some(Self::Rgba8Srgb),
            "rgba8-unorm" | "r8g8b8a8-unorm" | "unorm" => Some(Self::Rgba8Unorm),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Rgba8Srgb => "rgba8-srgb",
            Self::Rgba8Unorm => "rgba8-unorm",
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) const fn vk_format(self) -> vk::Format {
        match self {
            Self::Rgba8Srgb => vk::Format::R8G8B8A8_SRGB,
            Self::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
        }
    }
}
