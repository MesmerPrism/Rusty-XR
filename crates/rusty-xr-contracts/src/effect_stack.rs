use crate::{ImageSize, StereoMediaLayout};

/// Versioned schema id for generic effect-stack descriptors.
pub const EFFECT_STACK_DESCRIPTOR_SCHEMA: &str = "rusty.xr.effect_stack.descriptor.v1";

/// Versioned schema id for generic effect-stack comparison reports.
pub const EFFECT_STACK_COMPARISON_REPORT_SCHEMA: &str =
    "rusty.xr.effect_stack.comparison_report.v1";

/// Public category for a renderer-owned image-processing pass.
///
/// These categories describe scheduling and diagnostics only. They do not
/// define shader code, visual tuning, private color ramps, or app-specific
/// behavior.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectPassKind {
    /// App-provided source frame or stream.
    #[default]
    Source,
    /// Copy or normalize an external renderer/source texture into a normal
    /// renderer-owned working buffer.
    IngestCopy,
    /// Luminance, channel, contrast, threshold, or scalar guide transform.
    LumaTransform,
    /// Blur or smoothing pass.
    Blur,
    /// Generic color-map pass.
    ColorMap,
    /// Edge or feature detector.
    EdgeDetection,
    /// Scalar guide or mask generation pass.
    ScalarMap,
    /// Geometry, UV, or sample-location displacement pass.
    Displacement,
    /// Final composition pass.
    Composite,
    /// Debug tap that exposes an intermediate layer for capture or metrics.
    DiagnosticTap,
}

/// Role of an input consumed by a pass.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectPassInputRole {
    /// Full-color source input.
    #[default]
    SourceColor,
    /// External source texture or media surface before the first working copy.
    SourceExternal,
    /// Luma or scalar source input.
    SourceLuma,
    /// Intermediate guide texture or scalar field.
    Guide,
    /// Mask or confidence map.
    Mask,
    /// Explicit displacement, offset, or flow map.
    DisplacementMap,
    /// Output of the previous pass in the ordered graph.
    PreviousPass,
}

/// Public image-buffer format descriptor for pass planning.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectBufferFormat {
    #[default]
    Rgba8,
    Rgba16Float,
    Rgba32Float,
    R8,
    R16Float,
    R32Float,
    /// Android/OpenGL external-OES source texture. This is a descriptor label,
    /// not a native texture handle.
    ExternalOes,
    /// Generic external GPU resource when the public contract does not need a
    /// more specific source label.
    ExternalGpu,
}

/// Texture or logical buffer used by an effect stack.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EffectBufferDescriptor {
    pub buffer_id: String,
    pub size: ImageSize,
    pub format: EffectBufferFormat,
    pub stereo_layout: StereoMediaLayout,
    pub persistent: bool,
}

impl EffectBufferDescriptor {
    pub fn new(buffer_id: impl Into<String>, size: ImageSize, format: EffectBufferFormat) -> Self {
        Self {
            buffer_id: buffer_id.into(),
            size,
            format,
            stereo_layout: StereoMediaLayout::Mono,
            persistent: false,
        }
    }

    pub const fn with_stereo_layout(mut self, stereo_layout: StereoMediaLayout) -> Self {
        self.stereo_layout = stereo_layout;
        self
    }

    pub const fn persistent(mut self) -> Self {
        self.persistent = true;
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.buffer_id) && self.size.is_non_empty()
    }
}

/// A named input edge in an ordered effect graph.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectPassInput {
    pub input_id: String,
    pub role: EffectPassInputRole,
}

impl EffectPassInput {
    pub fn new(input_id: impl Into<String>, role: EffectPassInputRole) -> Self {
        Self {
            input_id: input_id.into(),
            role,
        }
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.input_id)
    }
}

/// One renderer-owned pass in a generic effect stack.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EffectPassDescriptor {
    pub pass_id: String,
    pub kind: EffectPassKind,
    pub inputs: Vec<EffectPassInput>,
    pub output_buffer: Option<String>,
    pub enabled_by_default: bool,
    pub offscreen: bool,
    pub separable: bool,
    pub diagnostic_label: Option<String>,
    pub parameter_keys: Vec<String>,
}

impl EffectPassDescriptor {
    pub fn new(pass_id: impl Into<String>, kind: EffectPassKind) -> Self {
        Self {
            pass_id: pass_id.into(),
            kind,
            inputs: Vec::new(),
            output_buffer: None,
            enabled_by_default: true,
            offscreen: false,
            separable: false,
            diagnostic_label: None,
            parameter_keys: Vec::new(),
        }
    }

    pub fn with_input(mut self, input_id: impl Into<String>, role: EffectPassInputRole) -> Self {
        self.inputs.push(EffectPassInput::new(input_id, role));
        self
    }

    pub fn with_output_buffer(mut self, output_buffer: impl Into<String>) -> Self {
        self.output_buffer = Some(output_buffer.into());
        self
    }

    pub const fn disabled_by_default(mut self) -> Self {
        self.enabled_by_default = false;
        self
    }

    pub const fn offscreen(mut self) -> Self {
        self.offscreen = true;
        self
    }

    pub const fn separable(mut self) -> Self {
        self.separable = true;
        self
    }

    pub fn with_diagnostic_label(mut self, label: impl Into<String>) -> Self {
        self.diagnostic_label = Some(label.into());
        self
    }

    pub fn with_parameter_key(mut self, key: impl Into<String>) -> Self {
        self.parameter_keys.push(key.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.pass_id)
            && self.inputs.iter().all(EffectPassInput::is_valid)
            && self
                .output_buffer
                .as_ref()
                .map(|buffer| stable_id(buffer))
                .unwrap_or(true)
            && self
                .diagnostic_label
                .as_ref()
                .map(|label| non_empty(label))
                .unwrap_or(true)
            && self.parameter_keys.iter().all(|key| stable_id(key))
    }
}

/// Intermediate layer exposed for capture, display, or offline comparison.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectDiagnosticLayer {
    pub layer_id: String,
    pub label: String,
    pub pass_id: Option<String>,
    pub buffer_id: Option<String>,
    pub expected_role: EffectPassInputRole,
}

impl EffectDiagnosticLayer {
    pub fn new(layer_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            layer_id: layer_id.into(),
            label: label.into(),
            pass_id: None,
            buffer_id: None,
            expected_role: EffectPassInputRole::Guide,
        }
    }

    pub fn from_pass(mut self, pass_id: impl Into<String>) -> Self {
        self.pass_id = Some(pass_id.into());
        self
    }

    pub fn from_buffer(mut self, buffer_id: impl Into<String>) -> Self {
        self.buffer_id = Some(buffer_id.into());
        self
    }

    pub const fn with_expected_role(mut self, expected_role: EffectPassInputRole) -> Self {
        self.expected_role = expected_role;
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.layer_id)
            && non_empty(&self.label)
            && self
                .pass_id
                .as_ref()
                .map(|pass_id| stable_id(pass_id))
                .unwrap_or(true)
            && self
                .buffer_id
                .as_ref()
                .map(|buffer_id| stable_id(buffer_id))
                .unwrap_or(true)
    }
}

/// Data-only description of a multi-pass visual pipeline.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EffectStackDescriptor {
    pub schema: String,
    pub stack_id: String,
    pub source_size: ImageSize,
    pub source_layout: StereoMediaLayout,
    pub buffers: Vec<EffectBufferDescriptor>,
    pub passes: Vec<EffectPassDescriptor>,
    pub diagnostic_layers: Vec<EffectDiagnosticLayer>,
}

impl EffectStackDescriptor {
    pub fn new(stack_id: impl Into<String>, source_size: ImageSize) -> Self {
        Self {
            schema: EFFECT_STACK_DESCRIPTOR_SCHEMA.to_string(),
            stack_id: stack_id.into(),
            source_size,
            source_layout: StereoMediaLayout::Mono,
            buffers: Vec::new(),
            passes: Vec::new(),
            diagnostic_layers: Vec::new(),
        }
    }

    pub const fn with_source_layout(mut self, source_layout: StereoMediaLayout) -> Self {
        self.source_layout = source_layout;
        self
    }

    pub fn with_buffer(mut self, buffer: EffectBufferDescriptor) -> Self {
        self.buffers.push(buffer);
        self
    }

    pub fn with_pass(mut self, pass: EffectPassDescriptor) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn with_diagnostic_layer(mut self, layer: EffectDiagnosticLayer) -> Self {
        self.diagnostic_layers.push(layer);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == EFFECT_STACK_DESCRIPTOR_SCHEMA
            && stable_id(&self.stack_id)
            && self.source_size.is_non_empty()
            && !self.passes.is_empty()
            && all_unique(self.buffers.iter().map(|buffer| buffer.buffer_id.as_str()))
            && all_unique(self.passes.iter().map(|pass| pass.pass_id.as_str()))
            && all_unique(
                self.passes
                    .iter()
                    .filter_map(|pass| pass.output_buffer.as_deref()),
            )
            && all_unique(
                self.diagnostic_layers
                    .iter()
                    .map(|layer| layer.layer_id.as_str()),
            )
            && self.buffers.iter().all(EffectBufferDescriptor::is_valid)
            && self.passes.iter().all(EffectPassDescriptor::is_valid)
            && self
                .passes
                .iter()
                .enumerate()
                .all(|(pass_index, pass)| self.pass_inputs_are_known(pass_index, pass))
            && self
                .diagnostic_layers
                .iter()
                .all(|layer| self.diagnostic_layer_is_known(layer))
    }

    fn pass_inputs_are_known(&self, pass_index: usize, pass: &EffectPassDescriptor) -> bool {
        pass.inputs
            .iter()
            .all(|input| self.input_id_is_known_before(&input.input_id, pass_index))
    }

    fn input_id_is_known_before(&self, input_id: &str, pass_index: usize) -> bool {
        input_id == "source"
            || self
                .buffers
                .iter()
                .any(|buffer| buffer.buffer_id == input_id)
            || self.passes.iter().take(pass_index).any(|pass| {
                pass.pass_id == input_id || pass.output_buffer.as_deref() == Some(input_id)
            })
    }

    fn diagnostic_layer_is_known(&self, layer: &EffectDiagnosticLayer) -> bool {
        if !layer.is_valid() {
            return false;
        }

        let pass_ok = layer
            .pass_id
            .as_ref()
            .map(|pass_id| self.passes.iter().any(|pass| pass.pass_id == *pass_id))
            .unwrap_or(true);
        let buffer_ok = layer
            .buffer_id
            .as_ref()
            .map(|buffer_id| {
                self.buffers
                    .iter()
                    .any(|buffer| buffer.buffer_id == *buffer_id)
                    || self
                        .passes
                        .iter()
                        .any(|pass| pass.output_buffer.as_deref() == Some(buffer_id.as_str()))
            })
            .unwrap_or(true);

        pass_ok && buffer_ok
    }
}

/// Scalar metrics for one captured diagnostic layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectLayerMetrics {
    pub active_pixel_fraction: f32,
    pub luma_mean: f32,
    pub luma_std: f32,
    pub edge_energy: f32,
    pub high_frequency_energy: f32,
}

impl EffectLayerMetrics {
    pub const fn new(active_pixel_fraction: f32) -> Self {
        Self {
            active_pixel_fraction,
            luma_mean: 0.0,
            luma_std: 0.0,
            edge_energy: 0.0,
            high_frequency_energy: 0.0,
        }
    }

    pub const fn with_luma(mut self, mean: f32, std: f32) -> Self {
        self.luma_mean = mean;
        self.luma_std = std;
        self
    }

    pub const fn with_energy(mut self, edge_energy: f32, high_frequency_energy: f32) -> Self {
        self.edge_energy = edge_energy;
        self.high_frequency_energy = high_frequency_energy;
        self
    }

    pub fn is_valid(self) -> bool {
        unit(self.active_pixel_fraction)
            && unit(self.luma_mean)
            && non_negative(self.luma_std)
            && non_negative(self.edge_energy)
            && non_negative(self.high_frequency_energy)
    }
}

/// Pairwise comparison metrics for one diagnostic layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectLayerComparisonMetrics {
    pub luma_rmse: f32,
    pub luma_bias: f32,
    pub luma_correlation: f32,
    pub edge_ratio_candidate_over_reference: f32,
    pub high_frequency_ratio_candidate_over_reference: f32,
}

impl EffectLayerComparisonMetrics {
    pub const fn new(
        luma_rmse: f32,
        edge_ratio_candidate_over_reference: f32,
        high_frequency_ratio_candidate_over_reference: f32,
    ) -> Self {
        Self {
            luma_rmse,
            luma_bias: 0.0,
            luma_correlation: 0.0,
            edge_ratio_candidate_over_reference,
            high_frequency_ratio_candidate_over_reference,
        }
    }

    pub const fn with_luma_fit(mut self, luma_bias: f32, luma_correlation: f32) -> Self {
        self.luma_bias = luma_bias;
        self.luma_correlation = luma_correlation;
        self
    }

    pub fn is_valid(self) -> bool {
        non_negative(self.luma_rmse)
            && finite(self.luma_bias)
            && finite(self.luma_correlation)
            && (-1.0..=1.0).contains(&self.luma_correlation)
            && non_negative(self.edge_ratio_candidate_over_reference)
            && non_negative(self.high_frequency_ratio_candidate_over_reference)
    }

    pub fn is_blur_ratio_close(self, tolerance: f32) -> bool {
        if !self.is_valid() || !unit(tolerance) {
            return false;
        }

        let min = 1.0 - tolerance;
        let max = 1.0 + tolerance;
        (min..=max).contains(&self.edge_ratio_candidate_over_reference)
            && (min..=max).contains(&self.high_frequency_ratio_candidate_over_reference)
    }
}

/// Result row for one layer in a generic effect-stack comparison.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EffectLayerComparison {
    pub layer_id: String,
    pub reference: Option<EffectLayerMetrics>,
    pub candidate: Option<EffectLayerMetrics>,
    pub pair: Option<EffectLayerComparisonMetrics>,
    pub note: Option<String>,
}

impl EffectLayerComparison {
    pub fn new(layer_id: impl Into<String>) -> Self {
        Self {
            layer_id: layer_id.into(),
            reference: None,
            candidate: None,
            pair: None,
            note: None,
        }
    }

    pub const fn with_reference(mut self, reference: EffectLayerMetrics) -> Self {
        self.reference = Some(reference);
        self
    }

    pub const fn with_candidate(mut self, candidate: EffectLayerMetrics) -> Self {
        self.candidate = Some(candidate);
        self
    }

    pub const fn with_pair(mut self, pair: EffectLayerComparisonMetrics) -> Self {
        self.pair = Some(pair);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.layer_id)
            && self
                .reference
                .map(EffectLayerMetrics::is_valid)
                .unwrap_or(true)
            && self
                .candidate
                .map(EffectLayerMetrics::is_valid)
                .unwrap_or(true)
            && self
                .pair
                .map(EffectLayerComparisonMetrics::is_valid)
                .unwrap_or(true)
            && self
                .note
                .as_ref()
                .map(|note| non_empty(note))
                .unwrap_or(true)
    }
}

/// Public report shape for an offline or headset-captured layer comparison.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EffectStackComparisonReport {
    pub schema: String,
    pub report_id: String,
    pub stack_id: String,
    pub reference_label: String,
    pub candidate_label: String,
    pub layers: Vec<EffectLayerComparison>,
}

impl EffectStackComparisonReport {
    pub fn new(
        report_id: impl Into<String>,
        stack_id: impl Into<String>,
        reference_label: impl Into<String>,
        candidate_label: impl Into<String>,
    ) -> Self {
        Self {
            schema: EFFECT_STACK_COMPARISON_REPORT_SCHEMA.to_string(),
            report_id: report_id.into(),
            stack_id: stack_id.into(),
            reference_label: reference_label.into(),
            candidate_label: candidate_label.into(),
            layers: Vec::new(),
        }
    }

    pub fn with_layer(mut self, layer: EffectLayerComparison) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == EFFECT_STACK_COMPARISON_REPORT_SCHEMA
            && stable_id(&self.report_id)
            && stable_id(&self.stack_id)
            && non_empty(&self.reference_label)
            && non_empty(&self.candidate_label)
            && !self.layers.is_empty()
            && all_unique(self.layers.iter().map(|layer| layer.layer_id.as_str()))
            && self.layers.iter().all(EffectLayerComparison::is_valid)
    }

    pub fn first_layer_outside_blur_tolerance(&self, tolerance: f32) -> Option<&str> {
        self.layers.iter().find_map(|layer| {
            let pair = layer.pair?;
            if pair.is_blur_ratio_close(tolerance) {
                None
            } else {
                Some(layer.layer_id.as_str())
            }
        })
    }
}

fn all_unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values: Vec<&str> = values.collect();
    values
        .iter()
        .enumerate()
        .all(|(index, value)| !values.iter().skip(index + 1).any(|other| other == value))
}

fn stable_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '+'))
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite(value: f32) -> bool {
    value.is_finite()
}

fn non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stack() -> EffectStackDescriptor {
        EffectStackDescriptor::new(
            "diagnostic.gl_oes_edge_mask_stack",
            ImageSize::new(1280, 720),
        )
        .with_source_layout(StereoMediaLayout::Separate)
        .with_buffer(
            EffectBufferDescriptor::new(
                "source.oes",
                ImageSize::new(1280, 720),
                EffectBufferFormat::ExternalOes,
            )
            .with_stereo_layout(StereoMediaLayout::Separate),
        )
        .with_buffer(
            EffectBufferDescriptor::new(
                "source.raw",
                ImageSize::new(1280, 720),
                EffectBufferFormat::Rgba8,
            )
            .with_stereo_layout(StereoMediaLayout::Separate),
        )
        .with_buffer(
            EffectBufferDescriptor::new(
                "guide.luma",
                ImageSize::new(384, 384),
                EffectBufferFormat::R16Float,
            )
            .persistent(),
        )
        .with_buffer(EffectBufferDescriptor::new(
            "guide.mask",
            ImageSize::new(384, 384),
            EffectBufferFormat::R8,
        ))
        .with_pass(
            EffectPassDescriptor::new("source", EffectPassKind::Source)
                .with_output_buffer("source.oes")
                .with_diagnostic_label("External OES source"),
        )
        .with_pass(
            EffectPassDescriptor::new("ingest.copy", EffectPassKind::IngestCopy)
                .with_input("source.oes", EffectPassInputRole::SourceExternal)
                .with_output_buffer("source.raw")
                .offscreen()
                .with_parameter_key("ingest.orientation_policy")
                .with_parameter_key("ingest.transform_matrix"),
        )
        .with_pass(
            EffectPassDescriptor::new("luma", EffectPassKind::LumaTransform)
                .with_input("source.raw", EffectPassInputRole::SourceColor)
                .with_output_buffer("guide.luma")
                .offscreen()
                .with_parameter_key("luma.contrast"),
        )
        .with_pass(
            EffectPassDescriptor::new("blur.horizontal", EffectPassKind::Blur)
                .with_input("guide.luma", EffectPassInputRole::Guide)
                .with_output_buffer("guide.blur_h")
                .offscreen()
                .separable(),
        )
        .with_pass(
            EffectPassDescriptor::new("blur.vertical", EffectPassKind::Blur)
                .with_input("guide.blur_h", EffectPassInputRole::Guide)
                .with_output_buffer("guide.blur")
                .offscreen()
                .separable(),
        )
        .with_pass(
            EffectPassDescriptor::new("edges", EffectPassKind::EdgeDetection)
                .with_input("guide.blur", EffectPassInputRole::Guide)
                .with_output_buffer("guide.edges")
                .offscreen(),
        )
        .with_pass(
            EffectPassDescriptor::new("mask.threshold", EffectPassKind::ScalarMap)
                .with_input("guide.edges", EffectPassInputRole::Guide)
                .with_output_buffer("guide.mask")
                .offscreen()
                .with_parameter_key("mask.threshold"),
        )
        .with_pass(
            EffectPassDescriptor::new("final", EffectPassKind::Composite)
                .with_input("source.raw", EffectPassInputRole::SourceColor)
                .with_input("guide.mask", EffectPassInputRole::Mask)
                .with_output_buffer("final.color"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("raw", "Raw source")
                .from_pass("ingest.copy")
                .from_buffer("source.raw")
                .with_expected_role(EffectPassInputRole::SourceColor),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("luma-guide", "Luma guide")
                .from_pass("luma")
                .from_buffer("guide.luma")
                .with_expected_role(EffectPassInputRole::Guide),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("blurred-guide", "Blurred guide")
                .from_pass("blur.vertical")
                .from_buffer("guide.blur"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("edge-guide", "Edge guide")
                .from_pass("edges")
                .from_buffer("guide.edges"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("mask", "Threshold mask")
                .from_pass("mask.threshold")
                .from_buffer("guide.mask")
                .with_expected_role(EffectPassInputRole::Mask),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("final", "Final composite")
                .from_pass("final")
                .from_buffer("final.color")
                .with_expected_role(EffectPassInputRole::SourceColor),
        )
    }

    #[test]
    fn stack_descriptor_validates_ordered_pass_graph() {
        assert!(sample_stack().is_valid());

        let invalid = EffectStackDescriptor::new("invalid", ImageSize::new(1280, 720)).with_pass(
            EffectPassDescriptor::new("final", EffectPassKind::Composite)
                .with_input("future.output", EffectPassInputRole::Guide),
        );

        assert!(!invalid.is_valid());
    }

    #[test]
    fn duplicate_pass_outputs_are_rejected() {
        let stack = EffectStackDescriptor::new("duplicate.outputs", ImageSize::new(1280, 720))
            .with_pass(
                EffectPassDescriptor::new("a", EffectPassKind::Source).with_output_buffer("shared"),
            )
            .with_pass(
                EffectPassDescriptor::new("b", EffectPassKind::Composite)
                    .with_input("shared", EffectPassInputRole::PreviousPass)
                    .with_output_buffer("shared"),
            );

        assert!(!stack.is_valid());
    }

    #[test]
    fn comparison_report_flags_first_blur_mismatch() {
        let report = EffectStackComparisonReport::new(
            "report-001",
            "diagnostic.color_edge_stack",
            "reference",
            "candidate",
        )
        .with_layer(
            EffectLayerComparison::new("raw")
                .with_pair(EffectLayerComparisonMetrics::new(0.03, 1.01, 0.99)),
        )
        .with_layer(
            EffectLayerComparison::new("blurred-guide")
                .with_pair(EffectLayerComparisonMetrics::new(0.14, 1.42, 1.51)),
        );

        assert!(report.is_valid());
        assert_eq!(
            report.first_layer_outside_blur_tolerance(0.15),
            Some("blurred-guide")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn effect_stack_round_trips_with_serde() {
        let stack = sample_stack();
        let encoded = serde_json::to_string(&stack).expect("stack should serialize");
        let decoded: EffectStackDescriptor =
            serde_json::from_str(&encoded).expect("stack should deserialize");

        assert_eq!(decoded, stack);
    }
}
