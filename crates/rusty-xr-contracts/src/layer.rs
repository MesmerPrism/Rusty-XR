use crate::{ColorRgba, Eye, ImageSize, Pose, Vec2};

/// Two-dimensional rectangle in caller-defined coordinates.
///
/// For source sampling this is normally normalized UV space. For display
/// layout it can be meters, pixels, or normalized surface space.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect2 {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect2 {
    pub const UNIT: Self = Self::new(Vec2::ZERO, Vec2::ONE);

    pub const fn new(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }

    pub fn max(self) -> Vec2 {
        self.origin + self.size
    }

    pub fn center(self) -> Vec2 {
        self.origin + (self.size * 0.5)
    }

    pub fn aspect(self) -> Option<f32> {
        if self.size.x > 0.0 && self.size.y > 0.0 && self.size.is_finite() {
            Some(self.size.x / self.size.y)
        } else {
            None
        }
    }

    pub fn is_valid(self) -> bool {
        self.origin.is_finite() && self.size.is_finite() && self.size.x >= 0.0 && self.size.y >= 0.0
    }

    pub fn inset(self, amount: f32) -> Option<Self> {
        if !self.is_valid() || !amount.is_finite() || amount < 0.0 {
            return None;
        }

        let max_inset = (self.size.x.min(self.size.y) * 0.5).max(0.0);
        let inset = amount.min(max_inset);
        Some(Self::new(
            Vec2::new(self.origin.x + inset, self.origin.y + inset),
            Vec2::new(
                (self.size.x - (inset * 2.0)).max(0.0),
                (self.size.y - (inset * 2.0)).max(0.0),
            ),
        ))
    }

    pub fn aspect_fit(self, content_aspect: f32) -> Option<Self> {
        self.aspect_layout(content_aspect, StereoLayerContentMode::Fit)
    }

    pub fn aspect_fill(self, content_aspect: f32) -> Option<Self> {
        self.aspect_layout(content_aspect, StereoLayerContentMode::Fill)
    }

    fn aspect_layout(self, content_aspect: f32, mode: StereoLayerContentMode) -> Option<Self> {
        if !self.is_valid()
            || self.size.x <= 0.0
            || self.size.y <= 0.0
            || !content_aspect.is_finite()
            || content_aspect <= 0.0
        {
            return None;
        }

        if matches!(mode, StereoLayerContentMode::Stretch) {
            return Some(self);
        }

        let container_aspect = self.size.x / self.size.y;
        let fit_by_width = match mode {
            StereoLayerContentMode::Fit => container_aspect <= content_aspect,
            StereoLayerContentMode::Fill => container_aspect > content_aspect,
            StereoLayerContentMode::Stretch => unreachable!(),
        };

        let size = if fit_by_width {
            Vec2::new(self.size.x, self.size.x / content_aspect)
        } else {
            Vec2::new(self.size.y * content_aspect, self.size.y)
        };
        let origin = Vec2::new(
            self.origin.x + ((self.size.x - size.x) * 0.5),
            self.origin.y + ((self.size.y - size.y) * 0.5),
        );

        Some(Self::new(origin, size))
    }
}

/// How a media source stores left/right eye images.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoMediaLayout {
    /// One image is used for both eyes.
    #[default]
    Mono,
    /// Left and right eyes are packed horizontally in one image.
    SideBySide { left_first: bool },
    /// Left and right eyes are packed vertically in one image.
    TopBottom { left_first: bool },
    /// Each eye has its own texture or stream, so each eye samples the full UV range.
    Separate,
}

impl StereoMediaLayout {
    pub const SIDE_BY_SIDE_LEFT_FIRST: Self = Self::SideBySide { left_first: true };
    pub const TOP_BOTTOM_LEFT_FIRST: Self = Self::TopBottom { left_first: true };

    pub fn eye_uv_rect(self, eye: Eye) -> Rect2 {
        match self {
            Self::Mono | Self::Separate => Rect2::UNIT,
            Self::SideBySide { left_first } => {
                let left_is_low_half = left_first;
                let use_low_half = matches!(eye, Eye::Left) == left_is_low_half;
                let x = if use_low_half { 0.0 } else { 0.5 };
                Rect2::new(Vec2::new(x, 0.0), Vec2::new(0.5, 1.0))
            }
            Self::TopBottom { left_first } => {
                let left_is_low_half = left_first;
                let use_low_half = matches!(eye, Eye::Left) == left_is_low_half;
                let y = if use_low_half { 0.0 } else { 0.5 };
                Rect2::new(Vec2::new(0.0, y), Vec2::new(1.0, 0.5))
            }
        }
    }

    pub fn eye_source_size(self, source_size: ImageSize) -> Option<Vec2> {
        if !source_size.is_non_empty() {
            return None;
        }

        let width = source_size.width as f32;
        let height = source_size.height as f32;
        let size = match self {
            Self::Mono | Self::Separate => Vec2::new(width, height),
            Self::SideBySide { .. } => Vec2::new(width * 0.5, height),
            Self::TopBottom { .. } => Vec2::new(width, height * 0.5),
        };

        if size.x > 0.0 && size.y > 0.0 {
            Some(size)
        } else {
            None
        }
    }

    pub fn eye_aspect(self, source_size: ImageSize) -> Option<f32> {
        let size = self.eye_source_size(source_size)?;
        Some(size.x / size.y)
    }
}

/// Display policy for fitting a media source into a projected surface.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoLayerContentMode {
    #[default]
    Fit,
    Fill,
    Stretch,
}

/// Public plain media layer descriptor for mono, stereo, or feedback sources.
///
/// This describes the app-owned projected layer shape. It is not the privileged
/// compositor passthrough layer and does not include downstream visual effects.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlainStereoLayer {
    pub source_size: ImageSize,
    pub source_layout: StereoMediaLayout,
    pub surface_size: Vec2,
    pub content_mode: StereoLayerContentMode,
    pub pose: Pose,
    pub opacity: f32,
    pub border: Option<VisualFeedbackBorder>,
    pub border_tuning: Option<FeedbackBorderTuning>,
    pub visual_feedback_tuning: Option<VisualFeedbackLayerTuning>,
    pub performance_hints: StereoLayerPerformanceHints,
}

impl PlainStereoLayer {
    pub const fn new(source_size: ImageSize, surface_size: Vec2) -> Self {
        Self {
            source_size,
            source_layout: StereoMediaLayout::Mono,
            surface_size,
            content_mode: StereoLayerContentMode::Fit,
            pose: Pose::IDENTITY,
            opacity: 1.0,
            border: None,
            border_tuning: None,
            visual_feedback_tuning: None,
            performance_hints: StereoLayerPerformanceHints::QUEST_STEREO_CAMERA_BASELINE,
        }
    }

    pub const fn with_source_layout(mut self, source_layout: StereoMediaLayout) -> Self {
        self.source_layout = source_layout;
        self
    }

    pub const fn with_content_mode(mut self, content_mode: StereoLayerContentMode) -> Self {
        self.content_mode = content_mode;
        self
    }

    pub const fn with_pose(mut self, pose: Pose) -> Self {
        self.pose = pose;
        self
    }

    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub const fn with_border(mut self, border: VisualFeedbackBorder) -> Self {
        self.border = Some(border);
        self
    }

    pub const fn with_border_tuning(mut self, border_tuning: FeedbackBorderTuning) -> Self {
        self.border_tuning = Some(border_tuning);
        self
    }

    pub const fn with_visual_feedback_tuning(
        mut self,
        visual_feedback_tuning: VisualFeedbackLayerTuning,
    ) -> Self {
        self.visual_feedback_tuning = Some(visual_feedback_tuning);
        self
    }

    pub const fn with_performance_hints(
        mut self,
        performance_hints: StereoLayerPerformanceHints,
    ) -> Self {
        self.performance_hints = performance_hints;
        self
    }

    pub fn is_valid(self) -> bool {
        self.source_size.is_non_empty()
            && self.surface_size.is_finite()
            && self.surface_size.x > 0.0
            && self.surface_size.y > 0.0
            && self.pose.is_finite()
            && self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
            && self
                .border
                .map(VisualFeedbackBorder::is_valid)
                .unwrap_or(true)
            && self
                .border_tuning
                .map(FeedbackBorderTuning::is_valid)
                .unwrap_or(true)
            && self
                .visual_feedback_tuning
                .map(VisualFeedbackLayerTuning::is_valid)
                .unwrap_or(true)
            && self.performance_hints.is_valid()
    }

    pub fn surface_rect(self) -> Rect2 {
        Rect2::new(Vec2::ZERO, self.surface_size)
    }

    pub fn eye_uv_rect(self, eye: Eye) -> Rect2 {
        self.source_layout.eye_uv_rect(eye)
    }

    pub fn eye_source_aspect(self) -> Option<f32> {
        self.source_layout.eye_aspect(self.source_size)
    }

    pub fn content_rect(self) -> Option<Rect2> {
        let surface = self.surface_rect();
        let aspect = self.eye_source_aspect()?;
        match self.content_mode {
            StereoLayerContentMode::Fit => surface.aspect_fit(aspect),
            StereoLayerContentMode::Fill => surface.aspect_fill(aspect),
            StereoLayerContentMode::Stretch => Some(surface),
        }
    }

    pub fn border_layout(self) -> Option<VisualFeedbackBorderLayout> {
        let border = self.border?;
        border.layout(self.content_rect()?)
    }
}

impl Default for PlainStereoLayer {
    fn default() -> Self {
        Self::new(ImageSize::new(1, 1), Vec2::ONE)
    }
}

/// Style for a simple visual feedback/content border.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualFeedbackBorder {
    pub thickness: f32,
    pub color: ColorRgba,
    pub opacity: f32,
}

impl VisualFeedbackBorder {
    pub const fn new(thickness: f32, color: ColorRgba) -> Self {
        Self {
            thickness,
            color,
            opacity: 1.0,
        }
    }

    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn is_valid(self) -> bool {
        self.thickness.is_finite()
            && self.thickness >= 0.0
            && self.color.is_finite()
            && self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
    }

    pub fn layout(self, content_rect: Rect2) -> Option<VisualFeedbackBorderLayout> {
        if !self.is_valid() || !content_rect.is_valid() {
            return None;
        }

        let thickness = self
            .thickness
            .min(content_rect.size.x * 0.5)
            .min(content_rect.size.y * 0.5)
            .max(0.0);
        let inner_height = (content_rect.size.y - (thickness * 2.0)).max(0.0);

        Some(VisualFeedbackBorderLayout {
            content_rect,
            top: Rect2::new(
                content_rect.origin,
                Vec2::new(content_rect.size.x, thickness),
            ),
            right: Rect2::new(
                Vec2::new(
                    content_rect.origin.x + content_rect.size.x - thickness,
                    content_rect.origin.y + thickness,
                ),
                Vec2::new(thickness, inner_height),
            ),
            bottom: Rect2::new(
                Vec2::new(
                    content_rect.origin.x,
                    content_rect.origin.y + content_rect.size.y - thickness,
                ),
                Vec2::new(content_rect.size.x, thickness),
            ),
            left: Rect2::new(
                Vec2::new(content_rect.origin.x, content_rect.origin.y + thickness),
                Vec2::new(thickness, inner_height),
            ),
            color: self.color,
            opacity: self.opacity,
        })
    }
}

impl Default for VisualFeedbackBorder {
    fn default() -> Self {
        Self::new(0.0, ColorRgba::WHITE)
    }
}

/// Public tuning knobs for a full-view camera feedback border.
///
/// These values describe the border mask and feedback response. They do not
/// implement downstream image-processing passes, effect maps, or
/// geometric-effect logic.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedbackBorderTuning {
    pub inner_coverage: f32,
    pub outer_coverage: f32,
    pub feedback_mix: f32,
    pub pullback: f32,
    pub swirl_strength: f32,
    pub zoom: f32,
    pub edge_boost: f32,
    pub rounded_radius: Vec2,
    pub rounded_feather: f32,
    pub corner_radius: f32,
    pub dark_edge_bleed_inset: f32,
    pub dark_edge_cutoff: f32,
    pub dark_edge_feather: f32,
}

impl FeedbackBorderTuning {
    /// Border-only baseline from custom Quest camera work, sanitized for public
    /// adapters. Downstream guide/effect stacks are intentionally not included.
    pub const FULL_VIEW_CAMERA_BORDER: Self = Self {
        inner_coverage: 0.18,
        outer_coverage: 0.82,
        feedback_mix: 1.0,
        pullback: 0.14,
        swirl_strength: 0.58,
        zoom: 0.22,
        edge_boost: 0.46,
        rounded_radius: Vec2::new(0.50, 0.39),
        rounded_feather: 0.12,
        corner_radius: 0.08,
        dark_edge_bleed_inset: 0.25,
        dark_edge_cutoff: 0.22,
        dark_edge_feather: 0.16,
    };

    pub fn is_valid(self) -> bool {
        unit(self.inner_coverage)
            && unit(self.outer_coverage)
            && self.inner_coverage < self.outer_coverage
            && unit(self.feedback_mix)
            && non_negative(self.pullback)
            && non_negative(self.swirl_strength)
            && non_negative(self.zoom)
            && non_negative(self.edge_boost)
            && self.rounded_radius.is_finite()
            && self.rounded_radius.x > 0.0
            && self.rounded_radius.y > 0.0
            && non_negative(self.rounded_feather)
            && non_negative(self.corner_radius)
            && unit(self.dark_edge_bleed_inset)
            && unit(self.dark_edge_cutoff)
            && unit(self.dark_edge_feather)
    }

    pub fn coverage_band(self) -> (f32, f32) {
        (
            self.inner_coverage.min(self.outer_coverage),
            self.inner_coverage.max(self.outer_coverage),
        )
    }

    pub fn feedback_mix_at_coverage(self, coverage: f32) -> Option<f32> {
        if !self.is_valid() || !coverage.is_finite() {
            return None;
        }

        let (inner, outer) = self.coverage_band();
        Some((1.0 - smoothstep(inner, outer, coverage)) * self.feedback_mix.clamp(0.0, 1.0))
    }
}

impl Default for FeedbackBorderTuning {
    fn default() -> Self {
        Self::FULL_VIEW_CAMERA_BORDER
    }
}

/// Public tuning knobs for a MediaProjection/composite feedback surface.
///
/// These are scalar adapter hints for recursive visual feedback around a border
/// or inset. The public core does not implement a feedback shader, UV warp,
/// depth modulation, or any downstream scene behavior.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualFeedbackLayerTuning {
    pub feedback_intensity: f32,
    pub border_gain: f32,
    pub fill_gain: f32,
    pub feedback_floor: f32,
    pub feedback_gain: f32,
    pub feedback_warp: f32,
    pub feedback_zoom: f32,
    pub organic_noise_scale: f32,
    pub organic_noise_amount: f32,
    pub low_confidence_threshold: f32,
    pub low_confidence_softness: f32,
}

impl VisualFeedbackLayerTuning {
    /// Public baseline for the visual feedback layer used with the border
    /// surface. Keep this separate from raw camera and native passthrough.
    pub const MEDIA_PROJECTION_BORDER_FEEDBACK: Self = Self {
        feedback_intensity: 1.10,
        border_gain: 1.25,
        fill_gain: 0.72,
        feedback_floor: 0.0,
        feedback_gain: 1.45,
        feedback_warp: 0.085,
        feedback_zoom: 0.095,
        organic_noise_scale: 5.0,
        organic_noise_amount: 0.16,
        low_confidence_threshold: 0.25,
        low_confidence_softness: 0.12,
    };

    pub fn is_valid(self) -> bool {
        non_negative(self.feedback_intensity)
            && non_negative(self.border_gain)
            && non_negative(self.fill_gain)
            && unit(self.feedback_floor)
            && non_negative(self.feedback_gain)
            && non_negative(self.feedback_warp)
            && non_negative(self.feedback_zoom)
            && non_negative(self.organic_noise_scale)
            && unit(self.organic_noise_amount)
            && unit(self.low_confidence_threshold)
            && unit(self.low_confidence_softness)
    }
}

impl Default for VisualFeedbackLayerTuning {
    fn default() -> Self {
        Self::MEDIA_PROJECTION_BORDER_FEEDBACK
    }
}

/// Preferred frame source for a projected stereo layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoLayerCameraPath {
    /// Opaque GPU-sampled camera buffers, such as Android hardware buffers.
    #[default]
    ExternalGpu,
    /// CPU-readable YUV frame path, useful for capture/debug but not the default.
    CpuYuv,
    /// RGBA display-composite source such as MediaProjection.
    CompositeRgba,
}

/// How much real environment-depth work a layer should request.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoLayerDepthPolicy {
    #[default]
    Off,
    RuntimeStateOnly,
    VisualDebug,
    ReadbackOrMeshing,
}

/// Public performance hints for custom stereo and feedback layers.
///
/// These are not platform calls. They document the knobs an Android/OpenXR/
/// Vulkan adapter should expose to keep the Quest path measurable and debuggable.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StereoLayerPerformanceHints {
    pub preferred_camera_path: StereoLayerCameraPath,
    pub allow_cpu_visible_path: bool,
    pub depth_policy: StereoLayerDepthPolicy,
    pub environment_cube: bool,
    pub physics_worker: bool,
    pub preserve_eye_packing_in_guides: bool,
    pub sample_full_feedback_uv_per_eye: bool,
    pub offscreen_pass_count: u8,
    pub guide_texture_size_px: u32,
    pub target_vertex_attributes: u16,
    pub target_descriptors: u16,
    pub reuse_descriptor_pools: bool,
    pub coalesce_frame_ready_events: bool,
}

impl StereoLayerPerformanceHints {
    /// Quest custom-stereo baseline: GPU camera path, one compact XR layer,
    /// depth/env-cube/physics off unless an explicit debug scene needs them.
    pub const QUEST_STEREO_CAMERA_BASELINE: Self = Self {
        preferred_camera_path: StereoLayerCameraPath::ExternalGpu,
        allow_cpu_visible_path: false,
        depth_policy: StereoLayerDepthPolicy::Off,
        environment_cube: false,
        physics_worker: false,
        preserve_eye_packing_in_guides: true,
        sample_full_feedback_uv_per_eye: true,
        offscreen_pass_count: 0,
        guide_texture_size_px: 0,
        target_vertex_attributes: 11,
        target_descriptors: 7,
        reuse_descriptor_pools: true,
        coalesce_frame_ready_events: true,
    };

    /// MediaProjection/composite feedback baseline: monoscopic RGBA source,
    /// full UV sampling for both eyes, and no real environment-depth work by
    /// default.
    pub const MEDIA_PROJECTION_FEEDBACK_BASELINE: Self = Self {
        preferred_camera_path: StereoLayerCameraPath::CompositeRgba,
        allow_cpu_visible_path: false,
        depth_policy: StereoLayerDepthPolicy::Off,
        environment_cube: false,
        physics_worker: false,
        preserve_eye_packing_in_guides: false,
        sample_full_feedback_uv_per_eye: true,
        offscreen_pass_count: 0,
        guide_texture_size_px: 512,
        target_vertex_attributes: 11,
        target_descriptors: 7,
        reuse_descriptor_pools: true,
        coalesce_frame_ready_events: true,
    };

    pub fn is_valid(self) -> bool {
        if self.allow_cpu_visible_path
            && matches!(
                self.preferred_camera_path,
                StereoLayerCameraPath::ExternalGpu
            )
        {
            return false;
        }

        let guide_size_valid = self.offscreen_pass_count == 0 || self.guide_texture_size_px >= 128;

        guide_size_valid && self.target_vertex_attributes > 0 && self.target_descriptors > 0
    }
}

impl Default for StereoLayerPerformanceHints {
    fn default() -> Self {
        Self::QUEST_STEREO_CAMERA_BASELINE
    }
}

/// Four rectangular border segments around a fitted content rectangle.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualFeedbackBorderLayout {
    pub content_rect: Rect2,
    pub top: Rect2,
    pub right: Rect2,
    pub bottom: Rect2,
    pub left: Rect2,
    pub color: ColorRgba,
    pub opacity: f32,
}

impl VisualFeedbackBorderLayout {
    pub fn segments(self) -> [Rect2; 4] {
        [self.top, self.right, self.bottom, self.left]
    }

    pub fn is_valid(self) -> bool {
        self.content_rect.is_valid()
            && self.segments().into_iter().all(Rect2::is_valid)
            && self.color.is_finite()
            && self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
    }
}

fn non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - (2.0 * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn side_by_side_layout_selects_eye_uv_halves() {
        let layout = StereoMediaLayout::SIDE_BY_SIDE_LEFT_FIRST;

        assert_eq!(
            layout.eye_uv_rect(Eye::Left),
            Rect2::new(Vec2::new(0.0, 0.0), Vec2::new(0.5, 1.0))
        );
        assert_eq!(
            layout.eye_uv_rect(Eye::Right),
            Rect2::new(Vec2::new(0.5, 0.0), Vec2::new(0.5, 1.0))
        );
    }

    #[test]
    fn aspect_fit_preserves_mono_media_aspect() {
        let layer = PlainStereoLayer::new(ImageSize::new(512, 288), Vec2::ONE);
        let rect = layer.content_rect().expect("content should fit");

        near(rect.origin.x, 0.0);
        near(rect.origin.y, 0.21875);
        near(rect.size.x, 1.0);
        near(rect.size.y, 0.5625);
    }

    #[test]
    fn stereo_layer_uses_eye_source_aspect() {
        let layer = PlainStereoLayer::new(ImageSize::new(2560, 1280), Vec2::ONE)
            .with_source_layout(StereoMediaLayout::SIDE_BY_SIDE_LEFT_FIRST);
        let rect = layer.content_rect().expect("content should fit");

        assert_eq!(rect, Rect2::UNIT);
    }

    #[test]
    fn visual_feedback_border_lays_out_inside_content_rect() {
        let layer = PlainStereoLayer::new(ImageSize::new(512, 288), Vec2::ONE).with_border(
            VisualFeedbackBorder::new(0.02, ColorRgba::new(0.0, 1.0, 1.0, 1.0)).with_opacity(0.75),
        );

        let border = layer.border_layout().expect("border should fit");

        near(border.content_rect.origin.y, 0.21875);
        near(border.top.size.y, 0.02);
        near(border.bottom.size.y, 0.02);
        near(border.left.size.x, 0.02);
        near(border.right.size.x, 0.02);
        near(border.left.size.y, 0.5225);
        near(border.opacity, 0.75);
        assert!(border.is_valid());
    }

    #[test]
    fn invalid_border_opacity_is_rejected() {
        let border = VisualFeedbackBorder::new(0.01, ColorRgba::WHITE).with_opacity(1.5);

        assert!(!border.is_valid());
        assert!(border.layout(Rect2::UNIT).is_none());
    }

    #[test]
    fn border_tuning_reports_public_mix_curve() {
        let tuning = FeedbackBorderTuning::FULL_VIEW_CAMERA_BORDER;

        near(tuning.feedback_mix_at_coverage(0.0).unwrap(), 1.0);
        near(tuning.feedback_mix_at_coverage(0.18).unwrap(), 1.0);
        near(tuning.feedback_mix_at_coverage(0.82).unwrap(), 0.0);
        near(tuning.feedback_mix_at_coverage(1.0).unwrap(), 0.0);
        assert!(tuning.is_valid());
    }

    #[test]
    fn visual_feedback_tuning_accepts_public_baseline() {
        let tuning = VisualFeedbackLayerTuning::MEDIA_PROJECTION_BORDER_FEEDBACK;

        assert!(tuning.is_valid());
        assert!(tuning.feedback_gain > tuning.border_gain);
    }

    #[test]
    fn performance_hints_reject_ambiguous_cpu_visible_path() {
        let valid = StereoLayerPerformanceHints::QUEST_STEREO_CAMERA_BASELINE;
        let invalid = StereoLayerPerformanceHints {
            allow_cpu_visible_path: true,
            ..valid
        };

        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn stereo_layer_round_trips_with_serde() {
        let layer = PlainStereoLayer::new(ImageSize::new(1920, 1080), Vec2::new(1.6, 0.9))
            .with_source_layout(StereoMediaLayout::SIDE_BY_SIDE_LEFT_FIRST)
            .with_content_mode(StereoLayerContentMode::Fit)
            .with_border(VisualFeedbackBorder::new(0.02, ColorRgba::WHITE));

        let encoded = serde_json::to_string(&layer).expect("layer should serialize");
        let decoded: PlainStereoLayer =
            serde_json::from_str(&encoded).expect("layer should deserialize");

        assert_eq!(decoded, layer);
    }
}
