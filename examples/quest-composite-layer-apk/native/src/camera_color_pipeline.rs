#[cfg(target_os = "android")]
use ash::vk;

pub(crate) const CAMERA_SHADER_FLAG_RAW_FEED: u32 = 1 << 13;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST: u32 = 1 << 14;
pub(crate) const CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA: u32 = 1 << 15;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_INVALID_FILL: u32 = 1 << 16;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_PERIMETER_FILL: u32 = 1 << 17;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_SOFT_BORDER: u32 = 1 << 18;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_STRONG_BORDER: u32 = 1 << 19;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_DYNAMIC_BORDER: u32 = 1 << 20;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_WARM_BORDER: u32 = 1 << 21;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_CYCLING_BORDER: u32 = 1 << 22;
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
    RawProjectionFast,
    RawProjectionSolidRed,
    RawProjectionInvalidFill,
    RawProjectionPerimeterFill,
    RawProjectionSoftBorder,
    RawProjectionStrongBorder,
    RawProjectionDynamicBorder,
    RawProjectionWarmBorder,
    RawProjectionCyclingBorder,
    RawProjectionUnderlay,
    RawProjectionCameraFootprintUnderlay,
    ProjectionAreaDiagnostic,
    DisplayEyeUvFiducial,
    ProjectionContentUvFiducial,
    SourceSamplingWitness,
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
            "" | "raw" | "none" | "off" | "passthrough" => Some(Self::Raw),
            "blur" | "blur-diagnostic" | "diagnostic-blur" => Some(Self::Blur),
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

impl CameraProjectionEffectMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "border-composite" | "border" | "composite" | "default" => Some(Self::BorderComposite),
            "raw-projection-fast" | "direct-raw-projection" | "raw-direct" | "fast-raw" => {
                Some(Self::RawProjectionFast)
            }
            "raw-projection-solid-red"
            | "raw-projection-red-border"
            | "direct-raw-projection-solid-red"
            | "fast-raw-solid-red" => Some(Self::RawProjectionSolidRed),
            "raw-projection-invalid-fill"
            | "raw-projection-invalid-only-fill"
            | "direct-raw-projection-invalid-fill"
            | "fast-raw-invalid-fill"
            | "raw-projection-fill"
            | "raw-projection-coverage-fill"
            | "raw-projection-fast-fill"
            | "direct-raw-projection-fill"
            | "fast-raw-fill" => Some(Self::RawProjectionInvalidFill),
            "raw-projection-perimeter-fill"
            | "raw-projection-rim-fill"
            | "direct-raw-projection-perimeter-fill"
            | "fast-raw-perimeter-fill" => Some(Self::RawProjectionPerimeterFill),
            "raw-projection-soft-border"
            | "raw-projection-cheap-border"
            | "direct-raw-projection-soft-border"
            | "fast-raw-soft-border" => Some(Self::RawProjectionSoftBorder),
            "raw-projection-strong-border"
            | "raw-projection-strong-cheap-border"
            | "direct-raw-projection-strong-border"
            | "fast-raw-strong-border" => Some(Self::RawProjectionStrongBorder),
            "raw-projection-dynamic-border"
            | "raw-projection-feedback-border"
            | "direct-raw-projection-dynamic-border"
            | "fast-raw-dynamic-border" => Some(Self::RawProjectionDynamicBorder),
            "raw-projection-warm-border"
            | "raw-projection-warm-feedback-border"
            | "direct-raw-projection-warm-border"
            | "fast-raw-warm-border" => Some(Self::RawProjectionWarmBorder),
            "raw-projection-cycling-border"
            | "raw-projection-cycle-border"
            | "raw-projection-spectral-border"
            | "direct-raw-projection-cycling-border"
            | "fast-raw-cycling-border" => Some(Self::RawProjectionCyclingBorder),
            "raw-projection-underlay"
            | "raw-projection-alpha-underlay"
            | "direct-raw-projection-underlay"
            | "fast-raw-underlay" => Some(Self::RawProjectionUnderlay),
            "raw-projection-camera-footprint-underlay"
            | "raw-projection-projection-area-bounded-underlay"
            | "raw-projection-bounded-footprint-underlay"
            | "camera-footprint-underlay"
            | "projection-area-bounded-underlay" => {
                Some(Self::RawProjectionCameraFootprintUnderlay)
            }
            "projection-area-diagnostic"
            | "camera-projection-area-diagnostic"
            | "raw-projection-area-diagnostic"
            | "fast-projection-area-diagnostic" => Some(Self::ProjectionAreaDiagnostic),
            "display-eye-uv-fiducial"
            | "display-eye-screen-uv-fiducial"
            | "display-eye-uv-map"
            | "mirror-mapping-fiducial" => Some(Self::DisplayEyeUvFiducial),
            "projection-content-uv-fiducial"
            | "post-offset-content-uv-fiducial"
            | "content-uv-map"
            | "projection-area-content-uv-map" => Some(Self::ProjectionContentUvFiducial),
            "source-sampling-witness"
            | "source-uv-sampling-witness"
            | "source-sampling-overlay"
            | "source-uv-map" => Some(Self::SourceSamplingWitness),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::BorderComposite => "border-composite",
            Self::RawProjectionFast => "raw-projection-fast",
            Self::RawProjectionSolidRed => "raw-projection-solid-red",
            Self::RawProjectionInvalidFill => "raw-projection-invalid-fill",
            Self::RawProjectionPerimeterFill => "raw-projection-perimeter-fill",
            Self::RawProjectionSoftBorder => "raw-projection-soft-border",
            Self::RawProjectionStrongBorder => "raw-projection-strong-border",
            Self::RawProjectionDynamicBorder => "raw-projection-dynamic-border",
            Self::RawProjectionWarmBorder => "raw-projection-warm-border",
            Self::RawProjectionCyclingBorder => "raw-projection-cycling-border",
            Self::RawProjectionUnderlay => "raw-projection-underlay",
            Self::RawProjectionCameraFootprintUnderlay => {
                "raw-projection-camera-footprint-underlay"
            }
            Self::ProjectionAreaDiagnostic => "projection-area-diagnostic",
            Self::DisplayEyeUvFiducial => "display-eye-uv-fiducial",
            Self::ProjectionContentUvFiducial => "projection-content-uv-fiducial",
            Self::SourceSamplingWitness => "source-sampling-witness",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::BorderComposite => 0,
            Self::RawProjectionFast => CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST,
            Self::RawProjectionSolidRed => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_INVALID_FILL
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_PERIMETER_FILL
            }
            Self::RawProjectionInvalidFill => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_INVALID_FILL
            }
            Self::RawProjectionPerimeterFill => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_PERIMETER_FILL
            }
            Self::RawProjectionSoftBorder => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_SOFT_BORDER
            }
            Self::RawProjectionStrongBorder => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_STRONG_BORDER
            }
            Self::RawProjectionDynamicBorder => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_DYNAMIC_BORDER
            }
            Self::RawProjectionWarmBorder => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_WARM_BORDER
            }
            Self::RawProjectionCyclingBorder => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_RAW_PROJECTION_CYCLING_BORDER
            }
            Self::RawProjectionUnderlay => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA
            }
            Self::RawProjectionCameraFootprintUnderlay => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA
            }
            Self::ProjectionAreaDiagnostic => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
            }
            Self::DisplayEyeUvFiducial => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
            }
            Self::ProjectionContentUvFiducial => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
                    | CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
            }
            Self::SourceSamplingWitness => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PROJECTION_AREA_DIAGNOSTIC
                    | CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
            }
        }
    }

    pub(crate) const fn diagnostic_shader_code(self) -> f32 {
        match self {
            Self::DisplayEyeUvFiducial => 1.0,
            Self::ProjectionContentUvFiducial => 2.0,
            Self::SourceSamplingWitness => 3.0,
            Self::RawProjectionCameraFootprintUnderlay => 4.0,
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

    pub(crate) const fn uses_fast_projection_pipeline(self) -> bool {
        matches!(
            self,
            Self::RawProjectionFast
                | Self::ProjectionAreaDiagnostic
                | Self::DisplayEyeUvFiducial
                | Self::ProjectionContentUvFiducial
                | Self::SourceSamplingWitness
        )
    }

    pub(crate) const fn uses_passthrough_underlay_alpha(self) -> bool {
        matches!(
            self,
            Self::RawProjectionUnderlay | Self::RawProjectionCameraFootprintUnderlay
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
