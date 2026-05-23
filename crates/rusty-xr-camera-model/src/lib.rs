//! Camera metadata and projection helpers for Rusty XR.
//!
//! This crate owns camera math that is public and app-neutral: intrinsics
//! scaling, pinhole projection/back-projection, and timestamp matching.
//!
//! Enable the `serde` feature to serialize helper result types; camera metadata
//! serialization is forwarded through `rusty-xr-contracts/serde`.
//!
//! ```
//! use rusty_xr_camera_model::{project_camera_point, CameraIntrinsics, ImageSize, Vec2, Vec3};
//!
//! let intrinsics = CameraIntrinsics::new(
//!     Vec2::new(500.0, 500.0),
//!     Vec2::new(320.0, 240.0),
//!     ImageSize::new(640, 480),
//! );
//! let pixel = project_camera_point(intrinsics, Vec3::new(0.0, 0.0, 1.0)).unwrap();
//! assert_eq!(pixel, Vec2::new(320.0, 240.0));
//! ```

use core::fmt;

pub use rusty_xr_contracts::{
    CameraCompositeTier, CameraExtrinsics, CameraFrameAdoptionMode, CameraFrameMetadata,
    CameraFrameMetadataFlags, CameraFrameTiming, CameraGpuBufferDescriptor, CameraImageRotation,
    CameraIntrinsics, CameraPixelDomain, CameraPixelDomainKind, CameraProjectionState,
    CameraProjectionStatus, CameraSourceId, CameraTextureTransform, ColorRgba, Eye, ImageSize,
    Pose, ProjectionTargetState, Quat, Rect2, StereoCameraFrameMetadata, StereoCameraFramePair,
    StereoMediaLayout, TemporalProjectionEdgeMode, TemporalProjectionMetrics,
    TemporalProjectionMode, TemporalProjectionPolicy, Vec2, Vec3, VisualProjectionState,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Camera model helper error.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraModelError {
    InvalidSourceIntrinsics,
    InvalidSourceSize,
    InvalidTargetSize,
    InvalidPoseTranslation,
    InvalidPoseRotation,
    InvalidTrackingBasis,
    InvalidProjectionSurface,
    PointBehindCamera,
    ZeroDepth,
}

impl fmt::Display for CameraModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceIntrinsics => f.write_str("invalid source intrinsics"),
            Self::InvalidSourceSize => f.write_str("invalid source image size"),
            Self::InvalidTargetSize => f.write_str("invalid target image size"),
            Self::InvalidPoseTranslation => f.write_str("invalid pose translation"),
            Self::InvalidPoseRotation => f.write_str("invalid pose rotation"),
            Self::InvalidTrackingBasis => f.write_str("invalid tracking basis"),
            Self::InvalidProjectionSurface => f.write_str("invalid projection surface"),
            Self::PointBehindCamera => f.write_str("point is behind the camera"),
            Self::ZeroDepth => f.write_str("depth must be non-zero"),
        }
    }
}

impl std::error::Error for CameraModelError {}

/// Public Quest camera-source preference used by the example shell.
pub const QUEST_CAMERA_PREFERRED_SQUARE_SIZE: u32 = 1280;

/// Public Quest camera-source cap used before preferring larger formats.
pub const QUEST_CAMERA_MAX_DIMENSION: u32 = 1920;

/// Source-size and source-kind policy for camera adapters.
///
/// The policy is intentionally metadata-only: platform adapters still own the
/// actual API calls, permission prompts, and camera-source handles.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraSourceSelectionPolicy {
    pub preferred_square_size: u32,
    pub max_dimension: u32,
    pub prefer_square: bool,
}

impl CameraSourceSelectionPolicy {
    pub const QUEST_RAW_CAMERA: Self = Self {
        preferred_square_size: QUEST_CAMERA_PREFERRED_SQUARE_SIZE,
        max_dimension: QUEST_CAMERA_MAX_DIMENSION,
        prefer_square: true,
    };
}

impl Default for CameraSourceSelectionPolicy {
    fn default() -> Self {
        Self::QUEST_RAW_CAMERA
    }
}

/// Public input used to rank candidate camera streams.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraSourceCandidate {
    pub size: ImageSize,
    pub is_stereo: bool,
    pub preferred_device_rank: u8,
    pub frame_rate_millihz: Option<u32>,
}

impl CameraSourceCandidate {
    pub const fn new(size: ImageSize) -> Self {
        Self {
            size,
            is_stereo: false,
            preferred_device_rank: 0,
            frame_rate_millihz: None,
        }
    }

    pub const fn with_stereo(mut self, is_stereo: bool) -> Self {
        self.is_stereo = is_stereo;
        self
    }

    pub const fn with_preferred_device_rank(mut self, rank: u8) -> Self {
        self.preferred_device_rank = rank;
        self
    }

    pub const fn with_frame_rate_millihz(mut self, frame_rate_millihz: u32) -> Self {
        self.frame_rate_millihz = Some(frame_rate_millihz);
        self
    }
}

/// Score a camera stream candidate. Higher scores are preferred.
///
/// Ordering priorities are stereo source, preferred device rank, exact
/// preferred square size, formats within the cap, square formats, pixel count,
/// and frame rate. Adapters can use this score directly or mirror it in
/// platform-native code.
pub fn score_camera_source_candidate(
    candidate: CameraSourceCandidate,
    policy: CameraSourceSelectionPolicy,
) -> Option<i64> {
    if !candidate.size.is_non_empty() {
        return None;
    }

    let preferred = policy.preferred_square_size.max(1);
    let cap = policy.max_dimension.max(preferred);
    let width = candidate.size.width;
    let height = candidate.size.height;
    let pixels = width as i64 * height as i64;
    let cap_pixels = cap as i64 * cap as i64;
    let exact_preferred_square = width == preferred && height == preferred;
    let within_cap = width <= cap && height <= cap;

    let mut score = 0_i64;
    if candidate.is_stereo {
        score += 10_000_000_000_000;
    }
    score += candidate.preferred_device_rank as i64 * 1_000_000_000_000;
    if exact_preferred_square {
        score += 100_000_000_000;
    }
    if within_cap {
        score += 10_000_000_000;
    } else {
        score -= 10_000_000_000;
    }
    if policy.prefer_square && width == height {
        score += 1_000_000_000;
    }

    score += pixels.min(cap_pixels) / 16;
    score -= width.abs_diff(preferred) as i64 * 5_000;
    score -= height.abs_diff(preferred) as i64 * 5_000;
    score += candidate.frame_rate_millihz.unwrap_or_default() as i64;
    Some(score)
}

/// Public projection/profile defaults for a Quest camera-driven custom layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraProjectionProfile {
    pub camera_fov_y_degrees: f32,
    pub preview_fov_y_degrees: f32,
    pub projection_scale: f32,
    pub raw_overlay_overscan: f32,
    pub full_view_overlay_overscan: f32,
    pub edge_fade: f32,
    pub center_offset_uv: Vec2,
}

impl CameraProjectionProfile {
    pub const QUEST_RAW_CAMERA_COMPOSITE: Self = Self {
        camera_fov_y_degrees: 92.0,
        preview_fov_y_degrees: 60.0,
        projection_scale: 1.0,
        raw_overlay_overscan: 1.06,
        full_view_overlay_overscan: 2.10,
        edge_fade: 0.12,
        center_offset_uv: Vec2::ZERO,
    };

    pub fn sanitized(self) -> Self {
        Self {
            camera_fov_y_degrees: finite_positive_or(self.camera_fov_y_degrees, 92.0),
            preview_fov_y_degrees: finite_positive_or(self.preview_fov_y_degrees, 60.0),
            projection_scale: finite_positive_or(self.projection_scale, 1.0),
            raw_overlay_overscan: finite_positive_or(self.raw_overlay_overscan, 1.0).max(1.0),
            full_view_overlay_overscan: finite_positive_or(self.full_view_overlay_overscan, 1.0)
                .max(1.0),
            edge_fade: finite_positive_or(self.edge_fade, 0.0).clamp(0.0, 0.5),
            center_offset_uv: if self.center_offset_uv.is_finite() {
                self.center_offset_uv
            } else {
                Vec2::ZERO
            },
        }
    }
}

impl Default for CameraProjectionProfile {
    fn default() -> Self {
        Self::QUEST_RAW_CAMERA_COMPOSITE
    }
}

/// Floating source window used to fill a target image while preserving aspect.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourcePixelWindow {
    pub origin_px: Vec2,
    pub size_px: Vec2,
}

impl SourcePixelWindow {
    pub const fn new(origin_px: Vec2, size_px: Vec2) -> Self {
        Self { origin_px, size_px }
    }

    pub fn is_valid(self) -> bool {
        self.origin_px.is_finite()
            && self.size_px.is_finite()
            && self.size_px.x > 0.0
            && self.size_px.y > 0.0
    }
}

/// Versioned schema id for renderer-neutral video projection geometry logs.
pub const VIDEO_PROJECTION_GEOMETRY_SCHEMA: &str = "rusty.xr.video_projection_geometry.v1";

/// Explicit source-to-surface mapping behavior requested by a video feed.
///
/// This is deliberately separate from provenance. A feed may come from a live
/// camera, a broker, a remote headset, or a synthetic generator, but the
/// geometry layer should only branch on this explicit behavior.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VideoProjectionMapping {
    /// Source UV is supplied by a screen-to-source homography.
    #[default]
    ScreenToSourceHomography,
    /// Source UV is supplied by a surface-to-source homography composed with
    /// the display-eye surface mapping.
    SurfaceToSourceHomography,
    /// The feed fills the chosen surface or canvas area directly.
    FullFrameSurface,
}

impl VideoProjectionMapping {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::ScreenToSourceHomography => "screen-to-source-homography",
            Self::SurfaceToSourceHomography => "surface-to-source-homography",
            Self::FullFrameSurface => "full-frame-surface",
        }
    }
}

/// Informational source provenance that must not control projection behavior.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VideoSourceProvenance {
    pub source_kind: String,
    pub transport_kind: String,
    pub provider_label: String,
}

impl VideoSourceProvenance {
    pub fn new(
        source_kind: impl Into<String>,
        transport_kind: impl Into<String>,
        provider_label: impl Into<String>,
    ) -> Self {
        Self {
            source_kind: source_kind.into(),
            transport_kind: transport_kind.into(),
            provider_label: provider_label.into(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.source_kind.trim().is_empty()
            && !self.transport_kind.trim().is_empty()
            && !self.provider_label.trim().is_empty()
    }
}

/// Concrete projection behavior exposed by any video feed before renderer
/// geometry is built.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoProjectionBehavior {
    pub mapping: VideoProjectionMapping,
    pub source_valid_uv_rect: Rect2,
    pub content_aspect_ratio: f32,
    pub desired_projection_aspect_ratio: f32,
}

impl VideoProjectionBehavior {
    pub const fn new(
        mapping: VideoProjectionMapping,
        source_valid_uv_rect: Rect2,
        content_aspect_ratio: f32,
        desired_projection_aspect_ratio: f32,
    ) -> Self {
        Self {
            mapping,
            source_valid_uv_rect,
            content_aspect_ratio,
            desired_projection_aspect_ratio,
        }
    }

    pub fn full_source(mapping: VideoProjectionMapping, aspect_ratio: f32) -> Self {
        Self::new(mapping, Rect2::UNIT, aspect_ratio, aspect_ratio)
    }

    pub fn is_valid(self) -> bool {
        rect_is_non_empty(self.source_valid_uv_rect)
            && rect_is_inside_unit(self.source_valid_uv_rect)
            && self.content_aspect_ratio.is_finite()
            && self.content_aspect_ratio > 0.0
            && self.desired_projection_aspect_ratio.is_finite()
            && self.desired_projection_aspect_ratio > 0.0
    }
}

/// Renderer-neutral video feed descriptor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct VideoFeedDescriptor {
    pub source_id: String,
    pub delivered_size: ImageSize,
    pub content_size: ImageSize,
    pub provenance: VideoSourceProvenance,
    pub behavior: VideoProjectionBehavior,
}

impl VideoFeedDescriptor {
    pub fn new(
        source_id: impl Into<String>,
        delivered_size: ImageSize,
        content_size: ImageSize,
        provenance: VideoSourceProvenance,
        behavior: VideoProjectionBehavior,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            delivered_size,
            content_size,
            provenance,
            behavior,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.source_id.trim().is_empty()
            && self.delivered_size.is_non_empty()
            && self.content_size.is_non_empty()
            && self.provenance.is_valid()
            && self.behavior.is_valid()
    }
}

/// Coverage allocated by the renderer before the feed is placed inside it.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionSurfaceDescriptor {
    pub surface_uv_rect: Rect2,
    pub depth_meters: f32,
    pub preview_fov_y_degrees: f32,
    pub preview_offset_y_meters: f32,
    pub raw_overlay_overscan: f32,
}

impl ProjectionSurfaceDescriptor {
    pub const fn new(
        surface_uv_rect: Rect2,
        depth_meters: f32,
        preview_fov_y_degrees: f32,
        preview_offset_y_meters: f32,
        raw_overlay_overscan: f32,
    ) -> Self {
        Self {
            surface_uv_rect,
            depth_meters,
            preview_fov_y_degrees,
            preview_offset_y_meters,
            raw_overlay_overscan,
        }
    }

    pub fn is_valid(self) -> bool {
        rect_is_non_empty(self.surface_uv_rect)
            && self.depth_meters.is_finite()
            && self.depth_meters > 0.0
            && self.preview_fov_y_degrees.is_finite()
            && self.preview_fov_y_degrees > 0.0
            && self.preview_offset_y_meters.is_finite()
            && self.raw_overlay_overscan.is_finite()
            && self.raw_overlay_overscan > 0.0
    }
}

/// Feed placement inside a renderer-owned surface or canvas.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedPlacementDescriptor {
    pub surface_uv_rect: Rect2,
    pub screen_uv_rect: Rect2,
    pub opacity: f32,
}

impl FeedPlacementDescriptor {
    pub const fn new(surface_uv_rect: Rect2, screen_uv_rect: Rect2, opacity: f32) -> Self {
        Self {
            surface_uv_rect,
            screen_uv_rect,
            opacity,
        }
    }

    pub fn is_valid(self) -> bool {
        rect_is_non_empty(self.surface_uv_rect)
            && rect_is_non_empty(self.screen_uv_rect)
            && self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
    }
}

/// Fill policy for the non-feed region of a surface/canvas.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionBorderFillPolicy {
    #[default]
    SolidColor,
    Transparent,
    PassthroughUnderlay,
}

impl ProjectionBorderFillPolicy {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SolidColor => "solid-color",
            Self::Transparent => "transparent",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }
}

/// Border/backdrop policy independent from the video feed.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionBorderDescriptor {
    pub fill_policy: ProjectionBorderFillPolicy,
    pub color: ColorRgba,
    pub opacity: f32,
}

impl ProjectionBorderDescriptor {
    pub const fn new(
        fill_policy: ProjectionBorderFillPolicy,
        color: ColorRgba,
        opacity: f32,
    ) -> Self {
        Self {
            fill_policy,
            color,
            opacity,
        }
    }

    pub const fn transparent() -> Self {
        Self::new(
            ProjectionBorderFillPolicy::Transparent,
            ColorRgba::new(0.0, 0.0, 0.0, 0.0),
            0.0,
        )
    }

    pub fn is_valid(self) -> bool {
        self.color.is_finite() && self.opacity.is_finite() && (0.0..=1.0).contains(&self.opacity)
    }
}

impl Default for ProjectionBorderDescriptor {
    fn default() -> Self {
        Self::new(
            ProjectionBorderFillPolicy::SolidColor,
            ColorRgba::WHITE,
            1.0,
        )
    }
}

/// Four screen-space segments that make up `surface - feed`.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionBorderLayout {
    pub outer_screen_uv_rect: Rect2,
    pub feed_screen_uv_rect: Rect2,
    pub top: Rect2,
    pub right: Rect2,
    pub bottom: Rect2,
    pub left: Rect2,
    pub descriptor: ProjectionBorderDescriptor,
}

impl ProjectionBorderLayout {
    pub fn segments(self) -> [Rect2; 4] {
        [self.top, self.right, self.bottom, self.left]
    }

    pub fn is_valid(self) -> bool {
        rect_is_non_empty(self.outer_screen_uv_rect)
            && self.feed_screen_uv_rect.is_valid()
            && self.segments().into_iter().all(Rect2::is_valid)
            && self.descriptor.is_valid()
    }
}

/// Shared source-valid footprint sampled in display-eye screen UV.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SourceValidScreenUvFootprint {
    pub active_fraction: f32,
    pub bbox_screen_uv_rect: Rect2,
    pub row_spans: Vec<SourceValidScreenUvRowSpan>,
}

impl SourceValidScreenUvFootprint {
    pub fn bbox_xywh(&self) -> [f32; 4] {
        rect_xywh(self.bbox_screen_uv_rect)
    }

    pub fn bbox_ltrb(&self) -> [f32; 4] {
        rect_ltrb(self.bbox_screen_uv_rect)
    }
}

/// Valid span on one display-eye screen row.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceValidScreenUvRowSpan {
    pub row_y: f32,
    pub active_fraction: f32,
    pub span: Option<(f32, f32)>,
}

/// Per-eye renderer-neutral projection plan.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PerEyeVideoProjectionPlan {
    pub eye: Eye,
    pub mapping: VideoProjectionMapping,
    pub surface_to_screen_uv: [[f32; 3]; 3],
    pub screen_to_surface_uv: [[f32; 3]; 3],
    pub surface_to_source_uv: [[f32; 3]; 3],
    pub screen_to_source_uv: [[f32; 3]; 3],
    pub surface_coverage_screen_uv_rect: Rect2,
    pub feed_placement: FeedPlacementDescriptor,
    pub border: ProjectionBorderLayout,
    pub source_valid_uv_rect: Rect2,
    pub source_valid_screen_uv_footprint: SourceValidScreenUvFootprint,
}

impl PerEyeVideoProjectionPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn from_homographies(
        eye: Eye,
        mapping: VideoProjectionMapping,
        surface_to_screen_uv: [[f32; 3]; 3],
        screen_to_surface_uv: [[f32; 3]; 3],
        surface_to_source_uv: [[f32; 3]; 3],
        screen_to_source_uv: [[f32; 3]; 3],
        feed_placement: FeedPlacementDescriptor,
        source_valid_uv_rect: Rect2,
        border_descriptor: ProjectionBorderDescriptor,
        footprint_grid: usize,
    ) -> Option<Self> {
        if !feed_placement.is_valid()
            || !rect_is_non_empty(source_valid_uv_rect)
            || !rect_is_inside_unit(source_valid_uv_rect)
            || !border_descriptor.is_valid()
        {
            return None;
        }
        let surface_coverage_screen_uv_rect =
            homography_unit_square_bounding_rect(surface_to_screen_uv).unwrap_or(Rect2::UNIT);
        let border = projection_border_layout(
            surface_coverage_screen_uv_rect,
            feed_placement.screen_uv_rect,
            border_descriptor,
        )?;
        let source_valid_screen_uv_footprint =
            if mapping == VideoProjectionMapping::FullFrameSurface {
                SourceValidScreenUvFootprint {
                    active_fraction: 1.0,
                    bbox_screen_uv_rect: Rect2::UNIT,
                    row_spans: vec![
                        SourceValidScreenUvRowSpan {
                            row_y: 0.0,
                            active_fraction: 1.0,
                            span: Some((0.0, 1.0)),
                        },
                        SourceValidScreenUvRowSpan {
                            row_y: 0.5,
                            active_fraction: 1.0,
                            span: Some((0.0, 1.0)),
                        },
                        SourceValidScreenUvRowSpan {
                            row_y: 1.0,
                            active_fraction: 1.0,
                            span: Some((0.0, 1.0)),
                        },
                    ],
                }
            } else {
                source_valid_screen_uv_footprint(
                    screen_to_source_uv,
                    source_valid_uv_rect,
                    footprint_grid,
                )
            };
        Some(Self {
            eye,
            mapping,
            surface_to_screen_uv,
            screen_to_surface_uv,
            surface_to_source_uv,
            screen_to_source_uv,
            surface_coverage_screen_uv_rect,
            feed_placement,
            border,
            source_valid_uv_rect,
            source_valid_screen_uv_footprint,
        })
    }

    pub fn marker_fields(&self, prefix: &str) -> String {
        let prefix = prefix.trim();
        let source_valid = rect_xywh(self.source_valid_uv_rect);
        let surface = rect_xywh(self.surface_coverage_screen_uv_rect);
        let feed = rect_xywh(self.feed_placement.screen_uv_rect);
        let expected = rect_xywh(self.source_valid_screen_uv_footprint.bbox_screen_uv_rect);
        let outer = rect_xywh(self.border.outer_screen_uv_rect);
        let inner = rect_xywh(self.border.feed_screen_uv_rect);
        format!(
            "{prefix}ProjectionGeometrySchema={} {prefix}ProjectionMapping={} {prefix}SurfaceCoverageScreenUvRect={} {prefix}FeedPlacementScreenUvRect={} {prefix}BorderOuterScreenUvRect={} {prefix}BorderInnerScreenUvRect={} {prefix}BorderRegionSemantics=surface_minus_feed {prefix}BorderFillPolicy={} {prefix}SourceValidUvRect={} {prefix}ExpectedSourceValidScreenUvRect={} {prefix}SourceValidActiveFraction={:.6}",
            VIDEO_PROJECTION_GEOMETRY_SCHEMA,
            self.mapping.stable_id(),
            uv_rect_token(surface),
            uv_rect_token(feed),
            uv_rect_token(outer),
            uv_rect_token(inner),
            self.border.descriptor.fill_policy.stable_id(),
            uv_rect_token(source_valid),
            uv_rect_token(expected),
            self.source_valid_screen_uv_footprint.active_fraction,
        )
    }
}

/// Apply a homography to a normalized UV coordinate.
pub fn apply_homography_uv(rows: [[f32; 3]; 3], uv: Vec2) -> Option<Vec2> {
    let w = rows[2][0] * uv.x + rows[2][1] * uv.y + rows[2][2];
    if !w.is_finite() || w.abs() <= 1.0e-6 {
        return None;
    }
    let u = (rows[0][0] * uv.x + rows[0][1] * uv.y + rows[0][2]) / w;
    let v = (rows[1][0] * uv.x + rows[1][1] * uv.y + rows[1][2]) / w;
    (u.is_finite() && v.is_finite()).then_some(Vec2::new(u, v))
}

/// Return the screen-space bounding rect of a homography-projected unit square.
pub fn homography_unit_square_bounding_rect(rows: [[f32; 3]; 3]) -> Option<Rect2> {
    let points = [
        apply_homography_uv(rows, Vec2::new(0.0, 0.0))?,
        apply_homography_uv(rows, Vec2::new(1.0, 0.0))?,
        apply_homography_uv(rows, Vec2::new(1.0, 1.0))?,
        apply_homography_uv(rows, Vec2::new(0.0, 1.0))?,
    ];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let rect = Rect2::new(
        Vec2::new(min_x, min_y),
        Vec2::new((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
    );
    rect_is_non_empty(rect).then_some(rect)
}

/// Sample the part of display-eye screen UV that maps into a source-valid UV
/// rectangle.
pub fn source_valid_screen_uv_footprint(
    screen_to_source_uv: [[f32; 3]; 3],
    source_valid_uv_rect: Rect2,
    grid: usize,
) -> SourceValidScreenUvFootprint {
    let grid = grid.max(2);
    let mut valid_count = 0_usize;
    let mut min_x = 1.0_f32;
    let mut min_y = 1.0_f32;
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    let step = 1.0 / grid as f32;
    for iy in 0..grid {
        for ix in 0..grid {
            let x = (ix as f32 + 0.5) * step;
            let y = (iy as f32 + 0.5) * step;
            if screen_uv_maps_to_source_uv(
                screen_to_source_uv,
                source_valid_uv_rect,
                Vec2::new(x, y),
            ) {
                valid_count += 1;
                min_x = min_x.min((x - step * 0.5).clamp(0.0, 1.0));
                min_y = min_y.min((y - step * 0.5).clamp(0.0, 1.0));
                max_x = max_x.max((x + step * 0.5).clamp(0.0, 1.0));
                max_y = max_y.max((y + step * 0.5).clamp(0.0, 1.0));
            }
        }
    }
    let bbox_screen_uv_rect = if valid_count == 0 {
        Rect2::new(Vec2::ZERO, Vec2::ZERO)
    } else {
        Rect2::new(
            Vec2::new(min_x, min_y),
            Vec2::new((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
        )
    };
    let row_spans = [0.0_f32, 0.5, 1.0]
        .into_iter()
        .map(|row_y| {
            source_valid_screen_uv_row_span(screen_to_source_uv, source_valid_uv_rect, row_y, 256)
        })
        .collect();
    SourceValidScreenUvFootprint {
        active_fraction: valid_count as f32 / (grid * grid) as f32,
        bbox_screen_uv_rect,
        row_spans,
    }
}

/// Sample one display-eye screen row for the source-valid span.
pub fn source_valid_screen_uv_row_span(
    screen_to_source_uv: [[f32; 3]; 3],
    source_valid_uv_rect: Rect2,
    row_y: f32,
    sample_count: usize,
) -> SourceValidScreenUvRowSpan {
    let sample_count = sample_count.max(2);
    let mut min_x = 1.0_f32;
    let mut max_x = 0.0_f32;
    let mut valid_count = 0_usize;
    for index in 0..sample_count {
        let x = index as f32 / (sample_count - 1) as f32;
        if screen_uv_maps_to_source_uv(
            screen_to_source_uv,
            source_valid_uv_rect,
            Vec2::new(x, row_y),
        ) {
            valid_count += 1;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
    }
    SourceValidScreenUvRowSpan {
        row_y,
        active_fraction: valid_count as f32 / sample_count as f32,
        span: (valid_count > 0).then_some((min_x, max_x)),
    }
}

/// Build four border rectangles for the screen-space region outside the feed.
pub fn projection_border_layout(
    outer_screen_uv_rect: Rect2,
    feed_screen_uv_rect: Rect2,
    descriptor: ProjectionBorderDescriptor,
) -> Option<ProjectionBorderLayout> {
    if !rect_is_non_empty(outer_screen_uv_rect)
        || !feed_screen_uv_rect.is_valid()
        || !descriptor.is_valid()
    {
        return None;
    }
    let feed = rect_intersection(outer_screen_uv_rect, feed_screen_uv_rect)
        .unwrap_or_else(|| Rect2::new(outer_screen_uv_rect.center(), Vec2::ZERO));
    let outer_max = outer_screen_uv_rect.max();
    let feed_max = feed.max();
    let top = Rect2::new(
        outer_screen_uv_rect.origin,
        Vec2::new(
            outer_screen_uv_rect.size.x,
            (feed.origin.y - outer_screen_uv_rect.origin.y).max(0.0),
        ),
    );
    let bottom = Rect2::new(
        Vec2::new(outer_screen_uv_rect.origin.x, feed_max.y),
        Vec2::new(
            outer_screen_uv_rect.size.x,
            (outer_max.y - feed_max.y).max(0.0),
        ),
    );
    let middle_height = feed.size.y.max(0.0);
    let left = Rect2::new(
        Vec2::new(outer_screen_uv_rect.origin.x, feed.origin.y),
        Vec2::new(
            (feed.origin.x - outer_screen_uv_rect.origin.x).max(0.0),
            middle_height,
        ),
    );
    let right = Rect2::new(
        Vec2::new(feed_max.x, feed.origin.y),
        Vec2::new((outer_max.x - feed_max.x).max(0.0), middle_height),
    );
    Some(ProjectionBorderLayout {
        outer_screen_uv_rect,
        feed_screen_uv_rect: feed,
        top,
        right,
        bottom,
        left,
        descriptor,
    })
}

pub fn rect_xywh(rect: Rect2) -> [f32; 4] {
    [rect.origin.x, rect.origin.y, rect.size.x, rect.size.y]
}

pub fn rect_ltrb(rect: Rect2) -> [f32; 4] {
    let max = rect.max();
    [rect.origin.x, rect.origin.y, max.x, max.y]
}

pub fn uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

/// Return the per-eye pixel domain for a delivered mono/stereo stream.
pub fn delivered_eye_stream_size(
    delivered_size: ImageSize,
    layout: StereoMediaLayout,
) -> Option<ImageSize> {
    if !delivered_size.is_non_empty() {
        return None;
    }

    let size = match layout {
        StereoMediaLayout::Mono | StereoMediaLayout::Separate => delivered_size,
        StereoMediaLayout::SideBySide { .. } => {
            ImageSize::new(delivered_size.width / 2, delivered_size.height)
        }
        StereoMediaLayout::TopBottom { .. } => {
            ImageSize::new(delivered_size.width, delivered_size.height / 2)
        }
    };

    size.is_non_empty().then_some(size)
}

/// Compute a centered crop window that fills `target_size` from `source_size`.
///
/// `overscan` values above `1.0` zoom into the source by shrinking the crop
/// window. This is useful when an app renders a full-view camera projection and
/// wants to avoid underscan or hard source edges.
pub fn centered_fill_source_window(
    source_size: ImageSize,
    target_size: ImageSize,
    overscan: f32,
) -> Option<SourcePixelWindow> {
    if !source_size.is_non_empty() || !target_size.is_non_empty() {
        return None;
    }

    let source_width = source_size.width as f32;
    let source_height = source_size.height as f32;
    let source_aspect = source_width / source_height;
    let target_aspect = target_size.width as f32 / target_size.height as f32;
    let overscan = finite_positive_or(overscan, 1.0).max(1.0);

    let (mut width, mut height) = if source_aspect > target_aspect {
        (source_height * target_aspect, source_height)
    } else {
        (source_width, source_width / target_aspect)
    };
    width /= overscan;
    height /= overscan;

    let origin = Vec2::new((source_width - width) * 0.5, (source_height - height) * 0.5);
    let window = SourcePixelWindow::new(origin, Vec2::new(width, height));
    window.is_valid().then_some(window)
}

/// Compute a simple alpha fade from normalized UV edges.
pub fn edge_fade_alpha(uv: Vec2, edge_fade: f32) -> f32 {
    let edge_fade = finite_positive_or(edge_fade, 0.0).clamp(0.0, 0.5);
    if edge_fade <= f32::EPSILON {
        return 1.0;
    }

    let edge_distance = uv.x.min(1.0 - uv.x).min(uv.y).min(1.0 - uv.y);
    (edge_distance / edge_fade).clamp(0.0, 1.0)
}

fn finite_positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn rect_is_non_empty(rect: Rect2) -> bool {
    rect.is_valid() && rect.size.x > 0.0 && rect.size.y > 0.0
}

fn rect_is_inside_unit(rect: Rect2) -> bool {
    let max = rect.max();
    rect.is_valid() && rect.origin.x >= 0.0 && rect.origin.y >= 0.0 && max.x <= 1.0 && max.y <= 1.0
}

fn rect_contains_uv(rect: Rect2, uv: Vec2) -> bool {
    let max = rect.max();
    uv.x >= rect.origin.x && uv.x <= max.x && uv.y >= rect.origin.y && uv.y <= max.y
}

fn rect_intersection(a: Rect2, b: Rect2) -> Option<Rect2> {
    if !a.is_valid() || !b.is_valid() {
        return None;
    }
    let a_max = a.max();
    let b_max = b.max();
    let min_x = a.origin.x.max(b.origin.x);
    let min_y = a.origin.y.max(b.origin.y);
    let max_x = a_max.x.min(b_max.x);
    let max_y = a_max.y.min(b_max.y);
    if max_x < min_x || max_y < min_y {
        return None;
    }
    Some(Rect2::new(
        Vec2::new(min_x, min_y),
        Vec2::new((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
    ))
}

fn screen_uv_maps_to_source_uv(
    screen_to_source_uv: [[f32; 3]; 3],
    source_valid_uv_rect: Rect2,
    screen_uv: Vec2,
) -> bool {
    apply_homography_uv(screen_to_source_uv, screen_uv)
        .map(|source_uv| rect_contains_uv(source_valid_uv_rect, source_uv))
        .unwrap_or(false)
}

/// Scale camera intrinsics from one pixel domain to another.
///
/// This is the core active-array-to-preview-stream operation used by camera
/// adapters before handing projection data to app-neutral consumers.
pub fn scale_intrinsics_to_image(
    intrinsics: CameraIntrinsics,
    source_size: ImageSize,
    target_size: ImageSize,
) -> Result<CameraIntrinsics, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if !source_size.is_non_empty() {
        return Err(CameraModelError::InvalidSourceSize);
    }
    if !target_size.is_non_empty() {
        return Err(CameraModelError::InvalidTargetSize);
    }

    let scale = Vec2::new(
        target_size.width as f32 / source_size.width as f32,
        target_size.height as f32 / source_size.height as f32,
    );

    Ok(CameraIntrinsics::new(
        Vec2::new(
            intrinsics.focal_length_px.x * scale.x,
            intrinsics.focal_length_px.y * scale.y,
        ),
        Vec2::new(
            intrinsics.principal_point_px.x * scale.x,
            intrinsics.principal_point_px.y * scale.y,
        ),
        target_size,
    )
    .with_skew_px(intrinsics.skew_px * scale.x))
}

/// Convert Android Camera2 lens pose arrays into public extrinsics.
///
/// Camera2 stores `LENS_POSE_TRANSLATION` as meters in the
/// `LENS_POSE_REFERENCE` coordinate frame and `LENS_POSE_ROTATION` as an
/// `[x, y, z, w]` quaternion that maps the Android sensor/reference frame into
/// the camera-aligned frame. The returned pose preserves that public reference
/// frame as `world_from_camera`, so the quaternion is normalized and inverted
/// before it is stored.
pub fn camera2_lens_pose_to_extrinsics(
    translation_m: [f32; 3],
    rotation_xyzw: [f32; 4],
) -> Result<CameraExtrinsics, CameraModelError> {
    if !translation_m.iter().all(|value| value.is_finite()) {
        return Err(CameraModelError::InvalidPoseTranslation);
    }
    if !rotation_xyzw.iter().all(|value| value.is_finite()) {
        return Err(CameraModelError::InvalidPoseRotation);
    }

    let rotation = Quat::new(
        rotation_xyzw[0],
        rotation_xyzw[1],
        rotation_xyzw[2],
        rotation_xyzw[3],
    );
    let norm_sq = rotation.length_squared();
    if norm_sq <= 1.0e-12 || !norm_sq.is_finite() {
        return Err(CameraModelError::InvalidPoseRotation);
    }

    let reference_from_camera = rotation.normalized_or(Quat::IDENTITY).conjugate();
    let extrinsics = CameraExtrinsics::new(Pose::new(
        Vec3::new(translation_m[0], translation_m[1], translation_m[2]),
        reference_from_camera,
    ));
    if extrinsics.is_valid() {
        Ok(extrinsics)
    } else {
        Err(CameraModelError::InvalidPoseRotation)
    }
}

/// A head/tracking basis used to resolve Android Camera2 pose-reference data
/// into the same space as the OpenXR preview surface.
///
/// `forward` is the current head-forward vector in the renderer's tracking
/// space. For OpenXR view poses this is commonly the view orientation applied
/// to negative Z.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackingBasis {
    pub origin: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl TrackingBasis {
    pub fn new(origin: Vec3, right: Vec3, up: Vec3, forward: Vec3) -> Option<Self> {
        let basis = Self {
            origin,
            right: right.normalized_or(Vec3::ZERO),
            up: up.normalized_or(Vec3::ZERO),
            forward: forward.normalized_or(Vec3::ZERO),
        };
        basis.is_valid().then_some(basis)
    }

    pub fn is_valid(self) -> bool {
        self.origin.is_finite()
            && basis_axis_is_valid(self.right)
            && basis_axis_is_valid(self.up)
            && basis_axis_is_valid(self.forward)
            && self.right.dot(self.up).abs() < 0.05
            && self.right.dot(self.forward).abs() < 0.05
            && self.up.dot(self.forward).abs() < 0.05
    }
}

/// Camera basis in the renderer's tracking space.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBasis {
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl CameraBasis {
    pub fn new(position: Vec3, right: Vec3, up: Vec3, forward: Vec3) -> Option<Self> {
        let basis = Self {
            position,
            right: right.normalized_or(Vec3::ZERO),
            up: up.normalized_or(Vec3::ZERO),
            forward: forward.normalized_or(Vec3::ZERO),
        };
        basis.is_valid().then_some(basis)
    }

    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && basis_axis_is_valid(self.right)
            && basis_axis_is_valid(self.up)
            && basis_axis_is_valid(self.forward)
            && self.right.dot(self.up).abs() < 0.05
            && self.right.dot(self.forward).abs() < 0.05
            && self.up.dot(self.forward).abs() < 0.05
    }
}

fn basis_axis_is_valid(axis: Vec3) -> bool {
    axis.is_finite() && (axis.length() - 1.0).abs() < 0.05
}

/// Convert a vector expressed in Android Camera2 sensor/reference coordinates
/// into the current renderer tracking basis.
///
/// Android Camera2 pose data is not automatically OpenXR view/world space. The
/// public renderer resolves it by assigning sensor +X to current head right,
/// sensor +Y to current head up, and sensor +Z opposite current head forward.
pub fn android_sensor_vector_to_tracking(basis: TrackingBasis, sensor_vector: Vec3) -> Vec3 {
    (basis.right * sensor_vector.x) + (basis.up * sensor_vector.y)
        - (basis.forward * sensor_vector.z)
}

/// Resolve Camera2 `LENS_POSE_*` extrinsics into a camera basis in the current
/// tracking frame.
///
/// The input extrinsics must be produced by `camera2_lens_pose_to_extrinsics`.
/// Its pose orientation is the Camera2 reference-from-camera rotation. The
/// returned basis can be used directly with a head-anchored preview surface.
pub fn camera_basis_from_camera2_reference_pose(
    tracking: TrackingBasis,
    extrinsics: CameraExtrinsics,
) -> Result<CameraBasis, CameraModelError> {
    camera_basis_from_camera2_reference_pose_relative_to_center(tracking, extrinsics, Vec3::ZERO)
}

/// Resolve Camera2 `LENS_POSE_*` extrinsics after subtracting a shared rig
/// center in the Camera2 reference frame.
///
/// Stereo headset camera poses are often reported relative to a gyroscope or
/// primary-camera reference rather than the head origin used by the renderer.
/// Subtracting the average left/right pose before mapping into tracking space
/// preserves the stereo baseline without making the whole projection surface
/// orbit around an unrelated platform pose origin.
pub fn camera_basis_from_camera2_reference_pose_relative_to_center(
    tracking: TrackingBasis,
    extrinsics: CameraExtrinsics,
    reference_center: Vec3,
) -> Result<CameraBasis, CameraModelError> {
    if !tracking.is_valid() {
        return Err(CameraModelError::InvalidTrackingBasis);
    }
    if !extrinsics.is_valid() || !extrinsics.world_from_camera.position.is_finite() {
        return Err(CameraModelError::InvalidPoseTranslation);
    }
    if !reference_center.is_finite() {
        return Err(CameraModelError::InvalidPoseTranslation);
    }

    let reference_from_camera = extrinsics
        .world_from_camera
        .orientation
        .normalized_or(Quat::IDENTITY);
    if !reference_from_camera.is_finite() {
        return Err(CameraModelError::InvalidPoseRotation);
    }

    let centered_position = extrinsics.world_from_camera.position - reference_center;
    let position = tracking.origin + android_sensor_vector_to_tracking(tracking, centered_position);
    let right =
        android_sensor_vector_to_tracking(tracking, reference_from_camera.rotate_vec3(Vec3::RIGHT));
    let up =
        android_sensor_vector_to_tracking(tracking, reference_from_camera.rotate_vec3(Vec3::UP));
    let forward = android_sensor_vector_to_tracking(
        tracking,
        reference_from_camera.rotate_vec3(Vec3::new(0.0, 0.0, 1.0)),
    );

    CameraBasis::new(position, right, up, forward).ok_or(CameraModelError::InvalidPoseRotation)
}

/// Return the four corners of a head-anchored preview-FOV camera surface in
/// tracking space. The order is top-left, top-right, bottom-right, bottom-left.
pub fn head_anchored_preview_surface_corners(
    tracking: TrackingBasis,
    preview_fov_y_degrees: f32,
    depth_meters: f32,
    aspect: f32,
    overscan: f32,
) -> Result<[Vec3; 4], CameraModelError> {
    if !tracking.is_valid() {
        return Err(CameraModelError::InvalidTrackingBasis);
    }
    if !preview_fov_y_degrees.is_finite()
        || !depth_meters.is_finite()
        || !aspect.is_finite()
        || !overscan.is_finite()
        || depth_meters <= 0.0
        || aspect <= 0.0
    {
        return Err(CameraModelError::InvalidProjectionSurface);
    }

    let half_height = (preview_fov_y_degrees.clamp(1.0, 175.0).to_radians() * 0.5).tan()
        * depth_meters
        * overscan.max(1.0);
    let half_width = half_height * aspect.clamp(0.1, 10.0);
    let center = tracking.origin + tracking.forward * depth_meters;
    Ok([
        center - tracking.right * half_width + tracking.up * half_height,
        center + tracking.right * half_width + tracking.up * half_height,
        center + tracking.right * half_width - tracking.up * half_height,
        center - tracking.right * half_width - tracking.up * half_height,
    ])
}

/// Return the UV scale from the visible full-view surface into the projected
/// camera-content surface.
///
/// Full-lens overlays often render a larger surface than the camera-content
/// projection itself. Public renderers can use this value in the fragment
/// shader as:
///
/// `content_uv = (surface_uv - 0.5) * scale + 0.5`
///
/// and build the camera homography over the smaller raw-overlay/content
/// surface. This mirrors the geometry contract without depending on any
/// downstream effect stack.
pub fn full_view_content_uv_scale(
    full_view_overlay_overscan: f32,
    raw_overlay_overscan: f32,
) -> Result<f32, CameraModelError> {
    if !full_view_overlay_overscan.is_finite()
        || !raw_overlay_overscan.is_finite()
        || full_view_overlay_overscan <= 0.0
        || raw_overlay_overscan <= 0.0
    {
        return Err(CameraModelError::InvalidProjectionSurface);
    }

    Ok((full_view_overlay_overscan.max(1.0) / raw_overlay_overscan.max(1.0)).max(1.0))
}

/// Project a tracking-space point through a source camera basis into normalized
/// camera UV.
pub fn project_tracking_point_to_camera_uv(
    camera: CameraBasis,
    intrinsics: CameraIntrinsics,
    tracking_point: Vec3,
) -> Result<Vec2, CameraModelError> {
    if !camera.is_valid() {
        return Err(CameraModelError::InvalidPoseRotation);
    }
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }

    let local = tracking_point - camera.position;
    let camera_point = Vec3::new(
        local.dot(camera.right),
        local.dot(camera.up),
        local.dot(camera.forward),
    );
    let pixel = project_camera_point(intrinsics, camera_point)?;
    Ok(Vec2::new(
        pixel.x / intrinsics.image_size.width as f32,
        pixel.y / intrinsics.image_size.height as f32,
    ))
}

/// Project a tracking-space point through an OpenXR display-eye view into
/// fullscreen shader UV.
///
/// The returned UV matches the Quest composite example's fullscreen triangle:
/// `x=0` is screen left and `y=0` is screen top. This is distinct from raw
/// OpenXR tangent space, where positive Y is up.
pub fn project_tracking_point_to_eye_screen_uv(
    eye: CameraBasis,
    tan_left: f32,
    tan_right: f32,
    tan_down: f32,
    tan_up: f32,
    tracking_point: Vec3,
) -> Result<Vec2, CameraModelError> {
    if !eye.is_valid() {
        return Err(CameraModelError::InvalidPoseRotation);
    }
    if !tan_left.is_finite()
        || !tan_right.is_finite()
        || !tan_down.is_finite()
        || !tan_up.is_finite()
        || tan_right <= tan_left
        || tan_up <= tan_down
    {
        return Err(CameraModelError::InvalidProjectionSurface);
    }

    let local = tracking_point - eye.position;
    let z = local.dot(eye.forward);
    if z <= 0.0 {
        return Err(CameraModelError::PointBehindCamera);
    }

    let tan_x = local.dot(eye.right) / z;
    let tan_y = local.dot(eye.up) / z;
    Ok(Vec2::new(
        (tan_x - tan_left) / (tan_right - tan_left),
        (tan_up - tan_y) / (tan_up - tan_down),
    ))
}

/// Compute homography rows from a unit preview surface to source-camera UV.
pub fn surface_to_camera_uv_homography(
    surface_corners: [Vec3; 4],
    camera: CameraBasis,
    intrinsics: CameraIntrinsics,
) -> Result<[[f32; 3]; 3], CameraModelError> {
    if !surface_corners.iter().all(|corner| corner.is_finite()) {
        return Err(CameraModelError::InvalidProjectionSurface);
    }

    let mut projected = [Vec2::ZERO; 4];
    for (index, corner) in surface_corners.iter().copied().enumerate() {
        projected[index] = project_tracking_point_to_camera_uv(camera, intrinsics, corner)?;
    }

    homography_from_unit_square(projected).ok_or(CameraModelError::InvalidProjectionSurface)
}

/// Compute homography rows from a unit preview surface to fullscreen display
/// UV for one OpenXR display eye.
pub fn surface_to_eye_screen_uv_homography(
    surface_corners: [Vec3; 4],
    eye: CameraBasis,
    tan_left: f32,
    tan_right: f32,
    tan_down: f32,
    tan_up: f32,
) -> Result<[[f32; 3]; 3], CameraModelError> {
    if !surface_corners.iter().all(|corner| corner.is_finite()) {
        return Err(CameraModelError::InvalidProjectionSurface);
    }

    let mut projected = [Vec2::ZERO; 4];
    for (index, corner) in surface_corners.iter().copied().enumerate() {
        projected[index] = project_tracking_point_to_eye_screen_uv(
            eye, tan_left, tan_right, tan_down, tan_up, corner,
        )?;
    }

    homography_from_unit_square(projected).ok_or(CameraModelError::InvalidProjectionSurface)
}

/// Compose a display-screen UV to source-camera UV homography from the shared
/// head-anchored content surface mappings.
pub fn screen_to_camera_uv_homography(
    surface_to_screen_uv: [[f32; 3]; 3],
    surface_to_camera_uv: [[f32; 3]; 3],
) -> Result<[[f32; 3]; 3], CameraModelError> {
    let screen_to_surface_uv = invert_homography(surface_to_screen_uv)
        .ok_or(CameraModelError::InvalidProjectionSurface)?;
    multiply_homographies(surface_to_camera_uv, screen_to_surface_uv)
        .ok_or(CameraModelError::InvalidProjectionSurface)
}

/// Build a 3x3 homography from unit-square UV to a projected quadrilateral.
pub fn homography_from_unit_square(points: [Vec2; 4]) -> Option<[[f32; 3]; 3]> {
    if !points.iter().all(|point| point.is_finite()) {
        return None;
    }

    let p0 = points[0];
    let p1 = points[1];
    let p2 = points[2];
    let p3 = points[3];
    let sx = p0.x - p1.x + p2.x - p3.x;
    let sy = p0.y - p1.y + p2.y - p3.y;
    let (g, h) = if sx.abs() <= 1.0e-6 && sy.abs() <= 1.0e-6 {
        (0.0, 0.0)
    } else {
        let dx1 = p1.x - p2.x;
        let dy1 = p1.y - p2.y;
        let dx2 = p3.x - p2.x;
        let dy2 = p3.y - p2.y;
        let denominator = dx1 * dy2 - dx2 * dy1;
        if denominator.abs() <= 1.0e-6 || !denominator.is_finite() {
            return None;
        }
        (
            (sx * dy2 - dx2 * sy) / denominator,
            (dx1 * sy - sx * dy1) / denominator,
        )
    };

    let rows = [
        [p1.x - p0.x + g * p1.x, p3.x - p0.x + h * p3.x, p0.x],
        [p1.y - p0.y + g * p1.y, p3.y - p0.y + h * p3.y, p0.y],
        [g, h, 1.0],
    ];
    rows.iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(rows)
}

/// Invert a 3x3 homography matrix.
pub fn invert_homography(rows: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    if !rows.iter().flatten().all(|value| value.is_finite()) {
        return None;
    }

    let a = rows[0][0];
    let b = rows[0][1];
    let c = rows[0][2];
    let d = rows[1][0];
    let e = rows[1][1];
    let f = rows[1][2];
    let g = rows[2][0];
    let h = rows[2][1];
    let i = rows[2][2];
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() <= 1.0e-7 || !det.is_finite() {
        return None;
    }
    let inv_det = 1.0 / det;
    let inverse = [
        [
            (e * i - f * h) * inv_det,
            (c * h - b * i) * inv_det,
            (b * f - c * e) * inv_det,
        ],
        [
            (f * g - d * i) * inv_det,
            (a * i - c * g) * inv_det,
            (c * d - a * f) * inv_det,
        ],
        [
            (d * h - e * g) * inv_det,
            (b * g - a * h) * inv_det,
            (a * e - b * d) * inv_det,
        ],
    ];
    inverse
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(inverse)
}

/// Multiply two 3x3 homographies as `left * right`.
pub fn multiply_homographies(left: [[f32; 3]; 3], right: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    if !left.iter().flatten().all(|value| value.is_finite())
        || !right.iter().flatten().all(|value| value.is_finite())
    {
        return None;
    }

    let mut out = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            out[row][col] = left[row][0] * right[0][col]
                + left[row][1] * right[1][col]
                + left[row][2] * right[2][col];
        }
    }
    out.iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(out)
}

/// Screen-to-camera homographies for a stereo custom camera projection.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StereoHomographyProjection {
    pub left_screen_to_camera: [[f32; 3]; 3],
    pub right_screen_to_camera: [[f32; 3]; 3],
}

impl StereoHomographyProjection {
    pub const fn new(
        left_screen_to_camera: [[f32; 3]; 3],
        right_screen_to_camera: [[f32; 3]; 3],
    ) -> Self {
        Self {
            left_screen_to_camera,
            right_screen_to_camera,
        }
    }

    pub fn is_valid(self) -> bool {
        homography_rows_are_finite(self.left_screen_to_camera)
            && homography_rows_are_finite(self.right_screen_to_camera)
    }
}

/// Motion and coverage metrics derived from stereo projection homographies.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StereoHomographyProjectionMetrics {
    pub average_motion_px: f32,
    pub p95_motion_px: f32,
    pub sample_count: u32,
    pub invalid_uv_sample_count: u32,
    pub invalid_uv_percent: f32,
}

/// Result of clamping a target stereo homography toward a previous visible
/// homography by screen-space p95 motion.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenMotionClampResult {
    pub projection: StereoHomographyProjection,
    pub alpha: f32,
    pub clamped: bool,
    pub target_motion: StereoHomographyProjectionMetrics,
    pub applied_motion: StereoHomographyProjectionMetrics,
    pub residual_motion: StereoHomographyProjectionMetrics,
}

/// Result of clamping a target stereo homography using a shared pose-delta
/// coefficient before measuring the resulting screen-space motion.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseDeltaClampResult {
    pub projection: StereoHomographyProjection,
    pub visual_pose: Pose,
    pub alpha: f32,
    pub clamped: bool,
    pub angular_delta_deg: f32,
    pub linear_delta_m: f32,
    pub target_motion: StereoHomographyProjectionMetrics,
    pub applied_motion: StereoHomographyProjectionMetrics,
    pub residual_motion: StereoHomographyProjectionMetrics,
}

/// Default sample grid for temporal projection motion estimates.
pub const TEMPORAL_PROJECTION_SAMPLE_GRID_5X5: [Vec2; 25] = [
    Vec2::new(0.1, 0.1),
    Vec2::new(0.3, 0.1),
    Vec2::new(0.5, 0.1),
    Vec2::new(0.7, 0.1),
    Vec2::new(0.9, 0.1),
    Vec2::new(0.1, 0.3),
    Vec2::new(0.3, 0.3),
    Vec2::new(0.5, 0.3),
    Vec2::new(0.7, 0.3),
    Vec2::new(0.9, 0.3),
    Vec2::new(0.1, 0.5),
    Vec2::new(0.3, 0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(0.7, 0.5),
    Vec2::new(0.9, 0.5),
    Vec2::new(0.1, 0.7),
    Vec2::new(0.3, 0.7),
    Vec2::new(0.5, 0.7),
    Vec2::new(0.7, 0.7),
    Vec2::new(0.9, 0.7),
    Vec2::new(0.1, 0.9),
    Vec2::new(0.3, 0.9),
    Vec2::new(0.5, 0.9),
    Vec2::new(0.7, 0.9),
    Vec2::new(0.9, 0.9),
];

/// Apply a 3x3 UV homography to a normalized UV coordinate.
pub fn apply_uv_homography(rows: [[f32; 3]; 3], uv: Vec2) -> Option<Vec2> {
    if !homography_rows_are_finite(rows) || !uv.is_finite() {
        return None;
    }

    let w = rows[2][0] * uv.x + rows[2][1] * uv.y + rows[2][2];
    if !w.is_finite() || w.abs() < 1.0e-6 {
        return None;
    }
    let projected = Vec2::new(
        (rows[0][0] * uv.x + rows[0][1] * uv.y + rows[0][2]) / w,
        (rows[1][0] * uv.x + rows[1][1] * uv.y + rows[1][2]) / w,
    );
    projected.is_finite().then_some(projected)
}

/// Estimate temporal projection motion from consecutive stereo homographies.
///
/// When `previous` is `None`, motion is reported as zero but invalid current
/// UV coverage is still measured. This matches the first-frame behavior needed
/// by metrics-only renderers.
pub fn stereo_homography_projection_metrics(
    previous: Option<StereoHomographyProjection>,
    current: StereoHomographyProjection,
    resolution: ImageSize,
) -> StereoHomographyProjectionMetrics {
    stereo_homography_projection_metrics_with_samples(
        previous,
        current,
        resolution,
        &TEMPORAL_PROJECTION_SAMPLE_GRID_5X5,
    )
}

/// Clamp target stereo homography motion to a p95 pixel budget.
///
/// The blend factor is shared between eyes so stereo lockstep is preserved.
/// Invalid previous projections fall back to the target projection, matching
/// first-frame behavior where there is nothing to smooth against.
pub fn clamp_stereo_homography_screen_motion(
    previous_visible: Option<StereoHomographyProjection>,
    target: StereoHomographyProjection,
    resolution: ImageSize,
    max_motion_px_per_frame: f32,
) -> ScreenMotionClampResult {
    let visual_start = previous_visible
        .filter(|projection| projection.is_valid())
        .unwrap_or(target);
    let target_motion =
        stereo_homography_projection_metrics(Some(visual_start), target, resolution);
    let max_motion_px = if max_motion_px_per_frame.is_finite() {
        max_motion_px_per_frame.max(1.0)
    } else {
        1.0
    };
    let target_motion_px = target_motion.p95_motion_px.max(0.0);
    let alpha = if target_motion_px > max_motion_px {
        (max_motion_px / target_motion_px).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let projection = blend_stereo_homography_projection(visual_start, target, alpha);
    let applied_motion =
        stereo_homography_projection_metrics(Some(visual_start), projection, resolution);
    let residual_motion =
        stereo_homography_projection_metrics(Some(projection), target, resolution);

    ScreenMotionClampResult {
        projection,
        alpha,
        clamped: alpha < 0.999,
        target_motion,
        applied_motion,
        residual_motion,
    }
}

/// Clamp target stereo homography advancement according to angular and linear
/// pose deltas.
///
/// The pose-derived blend factor is shared between eyes. Screen-space motion
/// metrics are still reported so scorecards can prove residual projection lag
/// without needing renderer-private pose state.
pub fn clamp_stereo_homography_pose_delta(
    previous_visible: Option<StereoHomographyProjection>,
    target: StereoHomographyProjection,
    previous_visual_pose: Option<Pose>,
    target_pose: Pose,
    resolution: ImageSize,
    max_angular_deg_per_frame: f32,
    max_linear_m_per_frame: f32,
) -> PoseDeltaClampResult {
    let visual_start = previous_visible
        .filter(|projection| projection.is_valid())
        .unwrap_or(target);
    let pose_start = previous_visual_pose
        .filter(|pose| pose.is_finite())
        .unwrap_or(target_pose);

    let angular_delta_deg = pose_angular_delta_degrees(pose_start, target_pose);
    let linear_delta_m = pose_linear_delta_meters(pose_start, target_pose);
    let angular_alpha =
        clamp_alpha_for_delta(angular_delta_deg, max_angular_deg_per_frame.max(0.0));
    let linear_alpha = clamp_alpha_for_delta(linear_delta_m, max_linear_m_per_frame.max(0.0));
    let alpha = angular_alpha.min(linear_alpha).clamp(0.0, 1.0);

    let projection = blend_stereo_homography_projection(visual_start, target, alpha);
    let visual_pose = blend_pose(pose_start, target_pose, alpha);
    let target_motion =
        stereo_homography_projection_metrics(Some(visual_start), target, resolution);
    let applied_motion =
        stereo_homography_projection_metrics(Some(visual_start), projection, resolution);
    let residual_motion =
        stereo_homography_projection_metrics(Some(projection), target, resolution);

    PoseDeltaClampResult {
        projection,
        visual_pose,
        alpha,
        clamped: alpha < 0.999,
        angular_delta_deg,
        linear_delta_m,
        target_motion,
        applied_motion,
        residual_motion,
    }
}

/// Linearly blend screen-to-camera homography rows, preserving the target when
/// interpolation produces non-finite rows.
pub fn blend_stereo_homography_projection(
    from: StereoHomographyProjection,
    to: StereoHomographyProjection,
    alpha: f32,
) -> StereoHomographyProjection {
    let alpha = alpha.clamp(0.0, 1.0);
    StereoHomographyProjection::new(
        blend_homography_rows(from.left_screen_to_camera, to.left_screen_to_camera, alpha),
        blend_homography_rows(
            from.right_screen_to_camera,
            to.right_screen_to_camera,
            alpha,
        ),
    )
}

fn clamp_alpha_for_delta(delta: f32, max_delta: f32) -> f32 {
    if !delta.is_finite() || delta <= 0.0 || !max_delta.is_finite() || max_delta <= 0.0 {
        return 1.0;
    }
    if delta > max_delta {
        (max_delta / delta).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn blend_pose(from: Pose, to: Pose, alpha: f32) -> Pose {
    let alpha = alpha.clamp(0.0, 1.0);
    Pose::new(
        from.position + ((to.position - from.position) * alpha),
        blend_quaternion(from.orientation, to.orientation, alpha),
    )
}

fn blend_quaternion(from: Quat, to: Quat, alpha: f32) -> Quat {
    let from = from.normalized_or(Quat::IDENTITY);
    let mut to = to.normalized_or(Quat::IDENTITY);
    if quat_dot(from, to) < 0.0 {
        to = Quat::new(-to.x, -to.y, -to.z, -to.w);
    }
    Quat::new(
        from.x + ((to.x - from.x) * alpha),
        from.y + ((to.y - from.y) * alpha),
        from.z + ((to.z - from.z) * alpha),
        from.w + ((to.w - from.w) * alpha),
    )
    .normalized_or(Quat::IDENTITY)
}

fn pose_angular_delta_degrees(from: Pose, to: Pose) -> f32 {
    let dot = quat_dot(
        from.orientation.normalized_or(Quat::IDENTITY),
        to.orientation.normalized_or(Quat::IDENTITY),
    )
    .abs()
    .clamp(0.0, 1.0);
    (2.0 * dot.acos()).to_degrees()
}

fn pose_linear_delta_meters(from: Pose, to: Pose) -> f32 {
    let delta = to.position - from.position;
    ((delta.x * delta.x) + (delta.y * delta.y) + (delta.z * delta.z)).sqrt()
}

fn quat_dot(left: Quat, right: Quat) -> f32 {
    (left.x * right.x) + (left.y * right.y) + (left.z * right.z) + (left.w * right.w)
}

fn blend_homography_rows(from: [[f32; 3]; 3], to: [[f32; 3]; 3], alpha: f32) -> [[f32; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            out[row][column] = from[row][column] + (to[row][column] - from[row][column]) * alpha;
        }
    }
    if homography_rows_are_finite(out) {
        out
    } else {
        to
    }
}

/// Estimate temporal projection motion with a caller-supplied sample grid.
pub fn stereo_homography_projection_metrics_with_samples(
    previous: Option<StereoHomographyProjection>,
    current: StereoHomographyProjection,
    resolution: ImageSize,
    samples: &[Vec2],
) -> StereoHomographyProjectionMetrics {
    let sample_count = (samples.len() * 2) as u32;
    if !current.is_valid() || !resolution.is_non_empty() || samples.is_empty() {
        return StereoHomographyProjectionMetrics {
            sample_count,
            invalid_uv_sample_count: sample_count,
            invalid_uv_percent: if sample_count == 0 { 0.0 } else { 100.0 },
            ..StereoHomographyProjectionMetrics::default()
        };
    }

    let invalid_uv_sample_count = count_invalid_uv_samples(current, samples);
    let invalid_uv_percent = if sample_count == 0 {
        0.0
    } else {
        invalid_uv_sample_count as f32 * 100.0 / sample_count as f32
    };
    let Some(previous) = previous.filter(|previous| previous.is_valid()) else {
        return StereoHomographyProjectionMetrics {
            sample_count,
            invalid_uv_sample_count,
            invalid_uv_percent,
            ..StereoHomographyProjectionMetrics::default()
        };
    };

    let mut deltas = Vec::with_capacity(sample_count as usize);
    for (previous_rows, current_rows) in [
        (
            previous.left_screen_to_camera,
            current.left_screen_to_camera,
        ),
        (
            previous.right_screen_to_camera,
            current.right_screen_to_camera,
        ),
    ] {
        for &sample in samples {
            let Some(previous_uv) = apply_uv_homography(previous_rows, sample) else {
                continue;
            };
            let Some(current_uv) = apply_uv_homography(current_rows, sample) else {
                continue;
            };
            let dx = (current_uv.x - previous_uv.x) * resolution.width as f32;
            let dy = (current_uv.y - previous_uv.y) * resolution.height as f32;
            let delta = (dx * dx + dy * dy).sqrt();
            if delta.is_finite() {
                deltas.push(delta);
            }
        }
    }

    StereoHomographyProjectionMetrics {
        average_motion_px: finite_average(&deltas),
        p95_motion_px: percentile_95(deltas),
        sample_count,
        invalid_uv_sample_count,
        invalid_uv_percent,
    }
}

fn homography_rows_are_finite(rows: [[f32; 3]; 3]) -> bool {
    rows.iter().flatten().all(|value| value.is_finite())
}

fn count_invalid_uv_samples(projection: StereoHomographyProjection, samples: &[Vec2]) -> u32 {
    let mut invalid = 0_u32;
    for rows in [
        projection.left_screen_to_camera,
        projection.right_screen_to_camera,
    ] {
        for &sample in samples {
            match apply_uv_homography(rows, sample) {
                Some(uv) if (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y) => {}
                _ => invalid += 1,
            }
        }
    }
    invalid
}

fn finite_average(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

fn percentile_95(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    let index = ((values.len() as f32 * 0.95).ceil() as usize).saturating_sub(1);
    values[index.min(values.len() - 1)]
}

/// Project a camera-space point into pixel coordinates.
pub fn project_camera_point(
    intrinsics: CameraIntrinsics,
    camera_point: Vec3,
) -> Result<Vec2, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if camera_point.z <= 0.0 {
        return Err(CameraModelError::PointBehindCamera);
    }

    Ok(Vec2::new(
        (camera_point.x * intrinsics.focal_length_px.x / camera_point.z)
            + intrinsics.principal_point_px.x,
        (camera_point.y * intrinsics.focal_length_px.y / camera_point.z)
            + intrinsics.principal_point_px.y,
    ))
}

/// Back-project a pixel and metric depth into camera-space coordinates.
pub fn back_project_pixel(
    intrinsics: CameraIntrinsics,
    pixel: Vec2,
    depth_meters: f32,
) -> Result<Vec3, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if !depth_meters.is_finite() || depth_meters.abs() <= f32::EPSILON {
        return Err(CameraModelError::ZeroDepth);
    }

    Ok(Vec3::new(
        (pixel.x - intrinsics.principal_point_px.x) * depth_meters / intrinsics.focal_length_px.x,
        (pixel.y - intrinsics.principal_point_px.y) * depth_meters / intrinsics.focal_length_px.y,
        depth_meters,
    ))
}

/// Result of nearest timestamp matching.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampMatch {
    pub candidate_index: usize,
    pub delta_ns: i128,
}

impl TimestampMatch {
    pub const fn absolute_delta_ns(self) -> u128 {
        self.delta_ns.unsigned_abs()
    }
}

/// Find the nearest candidate timestamp to a target timestamp.
pub fn match_nearest_timestamp(
    target_timestamp_ns: u64,
    candidate_timestamps_ns: &[u64],
    max_delta_ns: Option<u64>,
) -> Option<TimestampMatch> {
    let best = candidate_timestamps_ns
        .iter()
        .copied()
        .enumerate()
        .map(|(candidate_index, candidate)| TimestampMatch {
            candidate_index,
            delta_ns: candidate as i128 - target_timestamp_ns as i128,
        })
        .min_by_key(|candidate| candidate.absolute_delta_ns())?;

    if max_delta_ns
        .map(|max_delta_ns| best.absolute_delta_ns() <= max_delta_ns as u128)
        .unwrap_or(true)
    {
        Some(best)
    } else {
        None
    }
}

/// Result of matching one left camera timestamp with one right camera
/// timestamp.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StereoTimestampPair {
    pub left_index: usize,
    pub right_index: usize,
    pub delta_ns: u64,
    pub midpoint_timestamp_ns: u64,
}

/// Find the closest left/right timestamp pair within a maximum delta.
pub fn match_stereo_timestamps(
    left_timestamps_ns: &[u64],
    right_timestamps_ns: &[u64],
    max_delta_ns: u64,
) -> Option<StereoTimestampPair> {
    left_timestamps_ns
        .iter()
        .copied()
        .enumerate()
        .flat_map(|(left_index, left)| {
            right_timestamps_ns
                .iter()
                .copied()
                .enumerate()
                .map(move |(right_index, right)| {
                    let delta_ns = left.abs_diff(right);
                    StereoTimestampPair {
                        left_index,
                        right_index,
                        delta_ns,
                        midpoint_timestamp_ns: left / 2 + right / 2 + ((left % 2 + right % 2) / 2),
                    }
                })
        })
        .filter(|pair| pair.delta_ns <= max_delta_ns)
        .min_by_key(|pair| pair.delta_ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_intrinsics() -> CameraIntrinsics {
        CameraIntrinsics::new(
            Vec2::new(1000.0, 900.0),
            Vec2::new(500.0, 400.0),
            ImageSize::new(1000, 800),
        )
    }

    fn centered_square_intrinsics() -> CameraIntrinsics {
        CameraIntrinsics::new(
            Vec2::new(900.0, 900.0),
            Vec2::new(640.0, 640.0),
            ImageSize::new(1280, 1280),
        )
    }

    fn fov_matched_square_intrinsics() -> CameraIntrinsics {
        CameraIntrinsics::new(
            Vec2::new(640.0, 640.0),
            Vec2::new(640.0, 640.0),
            ImageSize::new(1280, 1280),
        )
    }

    fn tracking_basis() -> TrackingBasis {
        TrackingBasis::new(Vec3::ZERO, Vec3::RIGHT, Vec3::UP, Vec3::FORWARD_NEG_Z)
            .expect("test tracking basis is valid")
    }

    fn camera_for_tracking_basis(tracking: TrackingBasis, right_offset: f32) -> CameraBasis {
        CameraBasis::new(
            tracking.origin + tracking.right * right_offset,
            tracking.right,
            tracking.up,
            tracking.forward,
        )
        .expect("test camera basis is valid")
    }

    fn apply_homography(rows: [[f32; 3]; 3], uv: Vec2) -> Vec2 {
        let w = rows[2][0] * uv.x + rows[2][1] * uv.y + rows[2][2];
        Vec2::new(
            (rows[0][0] * uv.x + rows[0][1] * uv.y + rows[0][2]) / w,
            (rows[1][0] * uv.x + rows[1][1] * uv.y + rows[1][2]) / w,
        )
    }

    fn translated_uv_homography(dx: f32, dy: f32) -> [[f32; 3]; 3] {
        [[1.0, 0.0, dx], [0.0, 1.0, dy], [0.0, 0.0, 1.0]]
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn applies_uv_homography_and_rejects_singular_rows() {
        let translated = translated_uv_homography(0.125, -0.25);
        let projected =
            apply_uv_homography(translated, Vec2::new(0.5, 0.75)).expect("uv should project");
        assert_eq!(projected, Vec2::new(0.625, 0.5));

        assert_eq!(
            apply_uv_homography(
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
                Vec2::ONE
            ),
            None
        );
    }

    #[test]
    fn stereo_homography_metrics_reports_motion_and_invalid_uvs() {
        let previous = StereoHomographyProjection::new(
            translated_uv_homography(0.0, 0.0),
            translated_uv_homography(0.0, 0.0),
        );
        let current = StereoHomographyProjection::new(
            translated_uv_homography(0.01, 0.0),
            translated_uv_homography(0.01, 0.0),
        );
        let metrics = stereo_homography_projection_metrics(
            Some(previous),
            current,
            ImageSize::new(1000, 500),
        );

        assert_eq!(metrics.sample_count, 50);
        assert_eq!(metrics.invalid_uv_sample_count, 0);
        assert!((metrics.average_motion_px - 10.0).abs() < 0.001);
        assert!((metrics.p95_motion_px - 10.0).abs() < 0.001);

        let edge_shifted = StereoHomographyProjection::new(
            translated_uv_homography(0.2, 0.0),
            translated_uv_homography(0.2, 0.0),
        );
        let first_frame_metrics =
            stereo_homography_projection_metrics(None, edge_shifted, ImageSize::new(1000, 500));
        assert_eq!(first_frame_metrics.average_motion_px, 0.0);
        assert_eq!(first_frame_metrics.p95_motion_px, 0.0);
        assert_eq!(first_frame_metrics.invalid_uv_sample_count, 10);
        assert!((first_frame_metrics.invalid_uv_percent - 20.0).abs() < 0.001);
    }

    #[test]
    fn screen_motion_clamp_limits_applied_projection_motion() {
        let previous = StereoHomographyProjection::new(
            translated_uv_homography(0.0, 0.0),
            translated_uv_homography(0.0, 0.0),
        );
        let target = StereoHomographyProjection::new(
            translated_uv_homography(0.20, 0.0),
            translated_uv_homography(0.20, 0.0),
        );

        let clamped = clamp_stereo_homography_screen_motion(
            Some(previous),
            target,
            ImageSize::new(1000, 500),
            25.0,
        );

        assert!(clamped.clamped);
        assert!((clamped.target_motion.p95_motion_px - 200.0).abs() < 0.001);
        assert!(clamped.applied_motion.p95_motion_px <= 25.001);
        assert!((clamped.projection.left_screen_to_camera[0][2] - 0.025).abs() < 0.001);
        assert!((clamped.projection.right_screen_to_camera[0][2] - 0.025).abs() < 0.001);
        assert!(clamped.residual_motion.p95_motion_px > 170.0);

        let unclamped = clamp_stereo_homography_screen_motion(
            Some(previous),
            target,
            ImageSize::new(1000, 500),
            250.0,
        );

        assert!(!unclamped.clamped);
        assert_eq!(unclamped.projection, target);
        assert_eq!(unclamped.residual_motion.p95_motion_px, 0.0);
    }

    #[test]
    fn pose_delta_clamp_uses_shared_stereo_coefficient() {
        let previous = StereoHomographyProjection::new(
            translated_uv_homography(0.0, 0.0),
            translated_uv_homography(0.0, 0.0),
        );
        let target = StereoHomographyProjection::new(
            translated_uv_homography(0.20, 0.0),
            translated_uv_homography(0.10, 0.0),
        );
        let previous_pose = Pose::IDENTITY;
        let target_pose = Pose::new(
            Vec3::new(0.0, 0.0, 0.20),
            Quat::from_axis_angle(Vec3::UP, 10.0_f32.to_radians()),
        );

        let clamped = clamp_stereo_homography_pose_delta(
            Some(previous),
            target,
            Some(previous_pose),
            target_pose,
            ImageSize::new(1000, 500),
            2.5,
            0.20,
        );

        assert!(clamped.clamped);
        assert!((clamped.alpha - 0.25).abs() < 0.001);
        assert!((clamped.angular_delta_deg - 10.0).abs() < 0.001);
        assert!((clamped.projection.left_screen_to_camera[0][2] - 0.05).abs() < 0.001);
        assert!((clamped.projection.right_screen_to_camera[0][2] - 0.025).abs() < 0.001);
        assert!(clamped.residual_motion.p95_motion_px > 70.0);

        let unclamped = clamp_stereo_homography_pose_delta(
            Some(previous),
            target,
            Some(previous_pose),
            target_pose,
            ImageSize::new(1000, 500),
            12.0,
            0.30,
        );

        assert!(!unclamped.clamped);
        assert_eq!(unclamped.projection, target);
        assert_eq!(unclamped.residual_motion.p95_motion_px, 0.0);
    }

    #[test]
    fn scales_intrinsics_between_pixel_domains() {
        let scaled = scale_intrinsics_to_image(
            test_intrinsics(),
            ImageSize::new(1000, 800),
            ImageSize::new(500, 400),
        )
        .expect("intrinsics should scale");

        assert_eq!(scaled.focal_length_px, Vec2::new(500.0, 450.0));
        assert_eq!(scaled.principal_point_px, Vec2::new(250.0, 200.0));
        assert_eq!(scaled.image_size, ImageSize::new(500, 400));
    }

    #[test]
    fn scales_intrinsics_from_active_array_to_delivered_image() {
        let source = CameraIntrinsics::new(
            Vec2::new(2200.0, 2000.0),
            Vec2::new(1100.0, 1000.0),
            ImageSize::new(2200, 2000),
        )
        .with_skew_px(4.0);

        let scaled = scale_intrinsics_to_image(
            source,
            ImageSize::new(2200, 2000),
            ImageSize::new(1100, 1000),
        )
        .expect("active-array intrinsics should scale to delivered image");

        assert_eq!(scaled.focal_length_px, Vec2::new(1100.0, 1000.0));
        assert_eq!(scaled.principal_point_px, Vec2::new(550.0, 500.0));
        assert_eq!(scaled.skew_px, 2.0);
        assert_eq!(scaled.image_size, ImageSize::new(1100, 1000));
    }

    #[test]
    fn scores_preferred_quest_camera_candidate() {
        let policy = CameraSourceSelectionPolicy::default();
        let preferred = CameraSourceCandidate::new(ImageSize::new(1280, 1280))
            .with_stereo(true)
            .with_preferred_device_rank(2)
            .with_frame_rate_millihz(60_000);
        let off_square = CameraSourceCandidate::new(ImageSize::new(1920, 1080))
            .with_preferred_device_rank(2)
            .with_frame_rate_millihz(90_000);

        assert!(
            score_camera_source_candidate(preferred, policy)
                > score_camera_source_candidate(off_square, policy)
        );
        assert_eq!(
            score_camera_source_candidate(
                CameraSourceCandidate::new(ImageSize::new(0, 1280)),
                policy
            ),
            None
        );
    }

    #[test]
    fn computes_delivered_eye_stream_size() {
        assert_eq!(
            delivered_eye_stream_size(ImageSize::new(1280, 720), StereoMediaLayout::Mono),
            Some(ImageSize::new(1280, 720))
        );
        assert_eq!(
            delivered_eye_stream_size(ImageSize::new(1280, 720), StereoMediaLayout::Separate),
            Some(ImageSize::new(1280, 720))
        );
        assert_eq!(
            delivered_eye_stream_size(
                ImageSize::new(2560, 1280),
                StereoMediaLayout::SIDE_BY_SIDE_LEFT_FIRST
            ),
            Some(ImageSize::new(1280, 1280))
        );
        assert_eq!(
            delivered_eye_stream_size(
                ImageSize::new(1280, 960),
                StereoMediaLayout::TOP_BOTTOM_LEFT_FIRST
            ),
            Some(ImageSize::new(1280, 480))
        );
        assert_eq!(
            delivered_eye_stream_size(
                ImageSize::new(1, 1280),
                StereoMediaLayout::SideBySide { left_first: true }
            ),
            None
        );
    }

    #[test]
    fn centered_fill_window_preserves_target_aspect_and_overscan() {
        let window =
            centered_fill_source_window(ImageSize::new(1280, 1280), ImageSize::new(16, 9), 1.0)
                .expect("window should fit");

        assert_eq!(window.origin_px, Vec2::new(0.0, 280.0));
        assert_eq!(window.size_px, Vec2::new(1280.0, 720.0));

        let overscanned =
            centered_fill_source_window(ImageSize::new(1280, 1280), ImageSize::new(16, 9), 2.0)
                .expect("overscanned window should fit");

        assert_eq!(overscanned.origin_px, Vec2::new(320.0, 460.0));
        assert_eq!(overscanned.size_px, Vec2::new(640.0, 360.0));
    }

    #[test]
    fn video_projection_behavior_is_explicit_not_provenance_driven() {
        let behavior = VideoProjectionBehavior::full_source(
            VideoProjectionMapping::ScreenToSourceHomography,
            1.0,
        );
        let direct = VideoFeedDescriptor::new(
            "left",
            ImageSize::new(1280, 1280),
            ImageSize::new(1280, 1280),
            VideoSourceProvenance::new("live-camera", "direct-api", "platform-adapter"),
            behavior,
        );
        let broker = VideoFeedDescriptor::new(
            "left",
            ImageSize::new(1280, 1280),
            ImageSize::new(1280, 1280),
            VideoSourceProvenance::new("live-camera", "broker-stream", "remote-adapter"),
            behavior,
        );

        assert!(direct.is_valid());
        assert!(broker.is_valid());
        assert_eq!(direct.behavior, broker.behavior);
        assert_ne!(
            direct.provenance.transport_kind,
            broker.provenance.transport_kind
        );
    }

    #[test]
    fn source_valid_footprint_uses_explicit_source_rect() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let full = source_valid_screen_uv_footprint(identity, Rect2::UNIT, 8);
        let inset = source_valid_screen_uv_footprint(
            identity,
            Rect2::new(Vec2::new(0.25, 0.25), Vec2::new(0.5, 0.5)),
            8,
        );

        assert_eq!(full.bbox_xywh(), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(inset.bbox_xywh(), [0.25, 0.25, 0.5, 0.5]);
        assert!(full.active_fraction > inset.active_fraction);
    }

    #[test]
    fn projection_border_layout_is_surface_minus_feed() {
        let layout = projection_border_layout(
            Rect2::UNIT,
            Rect2::new(Vec2::new(0.2, 0.3), Vec2::new(0.6, 0.4)),
            ProjectionBorderDescriptor::new(
                ProjectionBorderFillPolicy::PassthroughUnderlay,
                ColorRgba::new(0.0, 0.0, 0.0, 0.0),
                0.0,
            ),
        )
        .expect("border should layout");

        assert!(layout.is_valid());
        assert_eq!(layout.top.size.y, 0.3);
        assert!((layout.bottom.origin.y - 0.7).abs() < 1.0e-6);
        assert_eq!(layout.left.size.x, 0.2);
        assert_eq!(layout.right.origin.x, 0.8);
        assert_eq!(
            layout.descriptor.fill_policy,
            ProjectionBorderFillPolicy::PassthroughUnderlay
        );
    }

    #[test]
    fn edge_fade_reaches_full_alpha_after_configured_width() {
        assert_eq!(edge_fade_alpha(Vec2::new(0.0, 0.5), 0.06), 0.0);
        assert_eq!(edge_fade_alpha(Vec2::new(0.06, 0.5), 0.06), 1.0);
        assert_eq!(edge_fade_alpha(Vec2::new(0.5, 0.5), 0.0), 1.0);
    }

    #[test]
    fn projection_round_trips_camera_point() {
        let intrinsics = test_intrinsics();
        let camera_point = Vec3::new(0.1, -0.2, 2.0);
        let pixel = project_camera_point(intrinsics, camera_point).expect("point should project");
        let round_trip =
            back_project_pixel(intrinsics, pixel, camera_point.z).expect("pixel should unproject");

        assert!((round_trip.x - camera_point.x).abs() < 1.0e-5);
        assert!((round_trip.y - camera_point.y).abs() < 1.0e-5);
        assert!((round_trip.z - camera_point.z).abs() < 1.0e-5);
    }

    #[test]
    fn camera2_lens_pose_conversion_normalizes_quaternion() {
        let extrinsics = camera2_lens_pose_to_extrinsics([0.03, -0.01, 0.02], [0.0, 0.0, 0.0, 2.0])
            .expect("finite Camera2 pose should convert");

        assert_eq!(
            extrinsics.world_from_camera.position,
            Vec3::new(0.03, -0.01, 0.02)
        );
        assert!((extrinsics.world_from_camera.orientation.w - 1.0).abs() < 1.0e-6);
        assert!(extrinsics.is_valid());
    }

    #[test]
    fn camera2_lens_pose_conversion_stores_world_from_camera_orientation() {
        let s = core::f32::consts::FRAC_1_SQRT_2;
        let extrinsics = camera2_lens_pose_to_extrinsics([0.0, 0.0, 0.0], [0.0, 0.0, s, s])
            .expect("finite Camera2 pose should convert");

        assert!((extrinsics.world_from_camera.orientation.z + s).abs() < 1.0e-6);
        assert!((extrinsics.world_from_camera.orientation.w - s).abs() < 1.0e-6);
    }

    #[test]
    fn camera2_lens_pose_conversion_rejects_missing_or_invalid_pose_values() {
        assert_eq!(
            camera2_lens_pose_to_extrinsics([f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
            Err(CameraModelError::InvalidPoseTranslation)
        );
        assert_eq!(
            camera2_lens_pose_to_extrinsics([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]),
            Err(CameraModelError::InvalidPoseRotation)
        );
    }

    #[test]
    fn android_sensor_vectors_resolve_through_head_basis() {
        let tracking = tracking_basis();

        assert_eq!(
            android_sensor_vector_to_tracking(tracking, Vec3::RIGHT),
            Vec3::RIGHT
        );
        assert_eq!(
            android_sensor_vector_to_tracking(tracking, Vec3::UP),
            Vec3::UP
        );
        assert_eq!(
            android_sensor_vector_to_tracking(tracking, Vec3::new(0.0, 0.0, 1.0)),
            Vec3::new(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn camera2_reference_pose_builds_tracking_camera_basis() {
        let tracking = tracking_basis();
        let extrinsics = camera2_lens_pose_to_extrinsics([0.03, 0.01, -0.02], [0.0, 0.0, 0.0, 1.0])
            .expect("pose should convert");
        let basis = camera_basis_from_camera2_reference_pose(tracking, extrinsics)
            .expect("basis should resolve");

        assert!((basis.position.x - 0.03).abs() < 1.0e-6);
        assert!((basis.position.y - 0.01).abs() < 1.0e-6);
        assert!((basis.position.z + 0.02).abs() < 1.0e-6);
        assert_eq!(basis.right, Vec3::RIGHT);
        assert_eq!(basis.up, Vec3::UP);
        assert_eq!(basis.forward, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn camera2_reference_pose_can_be_centered_on_stereo_rig() {
        let tracking = tracking_basis();
        let left = camera2_lens_pose_to_extrinsics([-0.03, -0.01, -0.07], [0.0, 0.0, 0.0, 1.0])
            .expect("left pose should convert");
        let right = camera2_lens_pose_to_extrinsics([0.03, -0.01, -0.07], [0.0, 0.0, 0.0, 1.0])
            .expect("right pose should convert");
        let center = (left.world_from_camera.position + right.world_from_camera.position) * 0.5;

        let left_basis =
            camera_basis_from_camera2_reference_pose_relative_to_center(tracking, left, center)
                .expect("left centered basis should resolve");
        let right_basis =
            camera_basis_from_camera2_reference_pose_relative_to_center(tracking, right, center)
                .expect("right centered basis should resolve");

        assert!((left_basis.position.x + 0.03).abs() < 1.0e-6);
        assert!((right_basis.position.x - 0.03).abs() < 1.0e-6);
        assert!(left_basis.position.y.abs() < 1.0e-6);
        assert!(right_basis.position.y.abs() < 1.0e-6);
        assert!(left_basis.position.z.abs() < 1.0e-6);
        assert!(right_basis.position.z.abs() < 1.0e-6);
    }

    #[test]
    fn stereo_baseline_shifts_projected_uv_in_expected_direction() {
        let tracking = tracking_basis();
        let intrinsics = centered_square_intrinsics();
        let surface = head_anchored_preview_surface_corners(tracking, 60.0, 1.0, 1.0, 1.0)
            .expect("surface should resolve");
        let left_camera = camera_for_tracking_basis(tracking, -0.03);
        let right_camera = camera_for_tracking_basis(tracking, 0.03);
        let left_rows = surface_to_camera_uv_homography(surface, left_camera, intrinsics)
            .expect("left projection should resolve");
        let right_rows = surface_to_camera_uv_homography(surface, right_camera, intrinsics)
            .expect("right projection should resolve");

        let left_center = apply_homography(left_rows, Vec2::new(0.5, 0.5));
        let right_center = apply_homography(right_rows, Vec2::new(0.5, 0.5));

        assert!(left_center.x > 0.5);
        assert!(right_center.x < 0.5);
        assert!(left_center.x > right_center.x);
    }

    #[test]
    fn horizontal_mirror_reverses_stereo_baseline_signal() {
        let tracking = tracking_basis();
        let intrinsics = centered_square_intrinsics();
        let surface = head_anchored_preview_surface_corners(tracking, 60.0, 1.0, 1.0, 1.0)
            .expect("surface should resolve");
        let left_rows = surface_to_camera_uv_homography(
            surface,
            camera_for_tracking_basis(tracking, -0.03),
            intrinsics,
        )
        .expect("left projection should resolve");
        let right_rows = surface_to_camera_uv_homography(
            surface,
            camera_for_tracking_basis(tracking, 0.03),
            intrinsics,
        )
        .expect("right projection should resolve");
        let mirror = CameraTextureTransform::new("test", "mirror check").with_flip_x(true);

        let mirrored_left = mirror.apply_uv(apply_homography(left_rows, Vec2::new(0.5, 0.5)));
        let mirrored_right = mirror.apply_uv(apply_homography(right_rows, Vec2::new(0.5, 0.5)));

        assert!(mirrored_left.x < mirrored_right.x);
    }

    #[test]
    fn rotate_180_does_not_preserve_source_eye_mapping_signal() {
        let tracking = tracking_basis();
        let intrinsics = centered_square_intrinsics();
        let surface = head_anchored_preview_surface_corners(tracking, 60.0, 1.0, 1.0, 1.0)
            .expect("surface should resolve");
        let rows = surface_to_camera_uv_homography(
            surface,
            camera_for_tracking_basis(tracking, -0.03),
            intrinsics,
        )
        .expect("projection should resolve");
        let rotate180 = CameraTextureTransform::new("test", "rotation check")
            .with_rotation(CameraImageRotation::Rotate180);

        let unrotated = apply_homography(rows, Vec2::new(0.25, 0.4));
        let rotated = rotate180.apply_uv(unrotated);

        assert!((rotated.x - (1.0 - unrotated.x)).abs() < 1.0e-6);
        assert!((rotated.y - (1.0 - unrotated.y)).abs() < 1.0e-6);
        assert_ne!(rotated.x > 0.5, unrotated.x > 0.5);
    }

    #[test]
    fn vertical_texture_flip_preserves_stereo_baseline_signal() {
        let tracking = tracking_basis();
        let intrinsics = centered_square_intrinsics();
        let surface = head_anchored_preview_surface_corners(tracking, 60.0, 1.0, 1.0, 1.0)
            .expect("surface should resolve");
        let left_rows = surface_to_camera_uv_homography(
            surface,
            camera_for_tracking_basis(tracking, -0.03),
            intrinsics,
        )
        .expect("left projection should resolve");
        let right_rows = surface_to_camera_uv_homography(
            surface,
            camera_for_tracking_basis(tracking, 0.03),
            intrinsics,
        )
        .expect("right projection should resolve");
        let flip_y =
            CameraTextureTransform::new("test", "display-origin correction").with_flip_y(true);

        let left = flip_y.apply_uv(apply_homography(left_rows, Vec2::new(0.5, 0.5)));
        let right = flip_y.apply_uv(apply_homography(right_rows, Vec2::new(0.5, 0.5)));

        assert!(left.x > 0.5);
        assert!(right.x < 0.5);
        assert!(left.x > right.x);
    }

    #[test]
    fn head_basis_rotation_keeps_relative_projection_stable() {
        let base_tracking = tracking_basis();
        let yaw = Quat::from_axis_angle(Vec3::UP, 0.35);
        let rotated_tracking = TrackingBasis::new(
            Vec3::new(0.2, 1.0, -0.4),
            yaw.rotate_vec3(Vec3::RIGHT),
            yaw.rotate_vec3(Vec3::UP),
            yaw.rotate_vec3(Vec3::FORWARD_NEG_Z),
        )
        .expect("rotated tracking basis should be valid");
        let intrinsics = centered_square_intrinsics();

        let base_surface =
            head_anchored_preview_surface_corners(base_tracking, 60.0, 1.0, 1.0, 1.0)
                .expect("base surface should resolve");
        let rotated_surface =
            head_anchored_preview_surface_corners(rotated_tracking, 60.0, 1.0, 1.0, 1.0)
                .expect("rotated surface should resolve");
        let base_rows = surface_to_camera_uv_homography(
            base_surface,
            camera_for_tracking_basis(base_tracking, -0.03),
            intrinsics,
        )
        .expect("base projection should resolve");
        let rotated_rows = surface_to_camera_uv_homography(
            rotated_surface,
            camera_for_tracking_basis(rotated_tracking, -0.03),
            intrinsics,
        )
        .expect("rotated projection should resolve");

        let uv = Vec2::new(0.62, 0.37);
        let base = apply_homography(base_rows, uv);
        let rotated = apply_homography(rotated_rows, uv);

        assert!((base.x - rotated.x).abs() < 1.0e-5);
        assert!((base.y - rotated.y).abs() < 1.0e-5);
    }

    #[test]
    fn full_view_uv_scale_maps_visible_surface_back_to_content_surface() {
        let scale = full_view_content_uv_scale(2.10, 1.06).expect("valid overscans");
        assert!((scale - 1.9811321).abs() < 0.0001);

        let visible_edge_uv = Vec2::new(0.0, 0.5);
        let content_uv = (visible_edge_uv - Vec2::new(0.5, 0.5)) * scale + Vec2::new(0.5, 0.5);
        let visible_local_x = (content_uv.x - 0.5) / scale;

        assert!((visible_local_x + 0.5).abs() < 0.0001);
    }

    #[test]
    fn openxr_eye_projection_uses_fullscreen_shader_uv_origin() {
        let tracking = tracking_basis();
        let eye = camera_for_tracking_basis(tracking, 0.0);
        let center = project_tracking_point_to_eye_screen_uv(
            eye,
            -1.0,
            1.0,
            -1.0,
            1.0,
            tracking.origin + tracking.forward,
        )
        .expect("center point should project");
        let top = project_tracking_point_to_eye_screen_uv(
            eye,
            -1.0,
            1.0,
            -1.0,
            1.0,
            tracking.origin + tracking.forward + tracking.up,
        )
        .expect("top point should project");

        assert!((center.x - 0.5).abs() < 1.0e-6);
        assert!((center.y - 0.5).abs() < 1.0e-6);
        assert!((top.x - 0.5).abs() < 1.0e-6);
        assert!(top.y.abs() < 1.0e-6);
    }

    #[test]
    fn screen_to_camera_homography_matches_eye_projection_for_centered_camera() {
        let tracking = tracking_basis();
        let eye = camera_for_tracking_basis(tracking, 0.0);
        let surface = head_anchored_preview_surface_corners(tracking, 90.0, 1.0, 1.0, 1.0)
            .expect("surface should resolve");
        let surface_to_screen =
            surface_to_eye_screen_uv_homography(surface, eye, -1.0, 1.0, -1.0, 1.0)
                .expect("screen homography should resolve");
        let surface_to_camera =
            surface_to_camera_uv_homography(surface, eye, fov_matched_square_intrinsics())
                .expect("camera homography should resolve");
        let screen_to_camera = screen_to_camera_uv_homography(surface_to_screen, surface_to_camera)
            .expect("screen-to-camera homography should resolve");

        for uv in [
            Vec2::new(0.25, 0.25),
            Vec2::new(0.5, 0.5),
            Vec2::new(0.75, 0.8),
        ] {
            let projected = apply_homography(screen_to_camera, uv);
            assert!((projected.x - uv.x).abs() < 1.0e-5);
            assert!((projected.y - (1.0 - uv.y)).abs() < 1.0e-5);
        }
    }

    #[test]
    fn scaled_projection_round_trips_camera_point() {
        let active_array_intrinsics = test_intrinsics();
        let delivered = scale_intrinsics_to_image(
            active_array_intrinsics,
            ImageSize::new(1000, 800),
            ImageSize::new(500, 400),
        )
        .expect("intrinsics should scale");
        let camera_point = Vec3::new(-0.15, 0.25, 3.0);

        let pixel = project_camera_point(delivered, camera_point).expect("point should project");
        let round_trip =
            back_project_pixel(delivered, pixel, camera_point.z).expect("pixel should unproject");

        assert!((round_trip.x - camera_point.x).abs() < 1.0e-5);
        assert!((round_trip.y - camera_point.y).abs() < 1.0e-5);
        assert!((round_trip.z - camera_point.z).abs() < 1.0e-5);
    }

    #[test]
    fn timestamp_matching_respects_max_delta() {
        let timestamps = [100, 150, 210];
        let matched = match_nearest_timestamp(160, &timestamps, Some(20))
            .expect("nearest timestamp should match");

        assert_eq!(
            matched,
            TimestampMatch {
                candidate_index: 1,
                delta_ns: -10,
            }
        );
        assert_eq!(match_nearest_timestamp(160, &timestamps, Some(5)), None);
    }

    #[test]
    fn stereo_timestamp_pairing_uses_closest_within_delta() {
        let left = [1_000_u64, 2_000, 3_000];
        let right = [980_u64, 2_040, 3_400];

        let pair =
            match_stereo_timestamps(&left, &right, 50).expect("nearest stereo pair should match");

        assert_eq!(pair.left_index, 0);
        assert_eq!(pair.right_index, 0);
        assert_eq!(pair.delta_ns, 20);
        assert_eq!(pair.midpoint_timestamp_ns, 990);
        assert_eq!(match_stereo_timestamps(&left, &right, 10), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn timestamp_match_round_trips_with_serde() {
        let value = TimestampMatch {
            candidate_index: 1,
            delta_ns: -10,
        };

        let encoded = serde_json::to_string(&value).expect("match should serialize");
        let decoded: TimestampMatch =
            serde_json::from_str(&encoded).expect("match should deserialize");

        assert_eq!(decoded, value);
    }
}
