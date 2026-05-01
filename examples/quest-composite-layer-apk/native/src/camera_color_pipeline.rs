#[cfg(target_os = "android")]
use ash::vk;

pub(crate) const CAMERA_SHADER_FLAG_RAW_FEED: u32 = 1 << 13;
pub(crate) const CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST: u32 = 1 << 14;
pub(crate) const CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA: u32 = 1 << 15;

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
    RawProjectionUnderlay,
}

impl CameraProjectionEffectMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "border-composite" | "border" | "composite" | "default" => Some(Self::BorderComposite),
            "raw-projection-fast" | "direct-raw-projection" | "raw-direct" | "fast-raw" => {
                Some(Self::RawProjectionFast)
            }
            "raw-projection-underlay"
            | "raw-projection-alpha-underlay"
            | "direct-raw-projection-underlay"
            | "fast-raw-underlay" => Some(Self::RawProjectionUnderlay),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::BorderComposite => "border-composite",
            Self::RawProjectionFast => "raw-projection-fast",
            Self::RawProjectionUnderlay => "raw-projection-underlay",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::BorderComposite => 0,
            Self::RawProjectionFast => CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST,
            Self::RawProjectionUnderlay => {
                CAMERA_SHADER_FLAG_RAW_PROJECTION_FAST
                    | CAMERA_SHADER_FLAG_PASSTHROUGH_UNDERLAY_ALPHA
            }
        }
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
