use serde_json::Value;

pub(crate) const SURFACE_SEQUENCE_SCHEMA_ID: &str =
    "rusty.matter.tools.glb_mesh_surface_sequence.v1";
const DEFAULT_SEQUENCE_JSON: &str =
    include_str!("../fixtures/mesh-replay/synthetic-hand-mesh-sequence.json");
const REPLAY_SCHEMA_ID: &str = "rusty.quest.makepad.mesh_replay.v1";
const REPLAY_MARKER_PREFIX: &str = "RUSTY_QUEST_MAKEPAD_MESH_REPLAY";
const REPLAY_ADAPTER: &str = "shader-panel-edge-overlay";
const MAX_FRAME_MARKERS: usize = 8;
const SELECTED_SEGMENT_COUNT: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct MeshReplayConfig {
    pub enabled: bool,
    pub source: String,
    pub speed: f32,
    pub opacity: f32,
}

impl MeshReplayConfig {
    pub fn normalized(enabled: bool, source: String, speed: f32, opacity: f32) -> Self {
        Self {
            enabled,
            source,
            speed: speed.clamp(0.0, 8.0),
            opacity: opacity.clamp(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeshReplayUniforms {
    pub enabled: f32,
    pub phase: f32,
    pub frame01: f32,
    pub opacity: f32,
    pub segments: [[f32; 4]; SELECTED_SEGMENT_COUNT],
}

impl MeshReplayUniforms {
    pub fn disabled() -> Self {
        Self {
            enabled: 0.0,
            phase: 0.0,
            frame01: 0.0,
            opacity: 0.0,
            segments: [[0.0; 4]; SELECTED_SEGMENT_COUNT],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeshReplayStep {
    pub enabled: bool,
    pub changed_frame: bool,
    pub frame_index: usize,
    pub frame_count: usize,
    pub phase: f32,
}

#[derive(Debug)]
pub(crate) struct MeshReplayRuntime {
    sequence: MeshReplaySequence,
    config: MeshReplayConfig,
    playback_seconds: f64,
    last_update_seconds: Option<f64>,
    current_frame_index: usize,
    config_marker_emitted: bool,
    frame_markers_emitted: usize,
}

impl Default for MeshReplayRuntime {
    fn default() -> Self {
        Self {
            sequence: MeshReplaySequence::from_json_str(DEFAULT_SEQUENCE_JSON)
                .expect("bundled public mesh replay fixture should parse"),
            config: MeshReplayConfig::normalized(
                false,
                "public-synthetic-hand-sequence".to_string(),
                1.0,
                0.84,
            ),
            playback_seconds: 0.0,
            last_update_seconds: None,
            current_frame_index: 0,
            config_marker_emitted: false,
            frame_markers_emitted: 0,
        }
    }
}

impl MeshReplayRuntime {
    pub fn configure(&mut self, config: MeshReplayConfig) -> bool {
        let changed = self.config.enabled != config.enabled
            || self.config.source != config.source
            || (self.config.speed - config.speed).abs() > f32::EPSILON
            || (self.config.opacity - config.opacity).abs() > f32::EPSILON;
        if changed {
            self.config = config;
            self.last_update_seconds = None;
            self.config_marker_emitted = false;
            self.frame_markers_emitted = 0;
        }
        changed
    }

    pub fn step(&mut self, now_seconds: f64) -> MeshReplayStep {
        if !self.config.enabled {
            self.last_update_seconds = Some(now_seconds.max(0.0));
            return MeshReplayStep {
                enabled: false,
                changed_frame: false,
                frame_index: self.current_frame_index,
                frame_count: self.sequence.frame_count(),
                phase: self.phase(),
            };
        }

        let now_seconds = now_seconds.max(0.0);
        let delta_seconds = self
            .last_update_seconds
            .map(|last| (now_seconds - last).max(0.0))
            .unwrap_or(0.0);
        self.last_update_seconds = Some(now_seconds);

        let duration = self.sequence.duration_seconds.max(0.001) as f64;
        self.playback_seconds =
            (self.playback_seconds + delta_seconds * self.config.speed as f64) % duration;

        let previous_frame = self.current_frame_index;
        self.current_frame_index = self.sequence.frame_index_at(self.playback_seconds as f32);

        MeshReplayStep {
            enabled: true,
            changed_frame: previous_frame != self.current_frame_index,
            frame_index: self.current_frame_index,
            frame_count: self.sequence.frame_count(),
            phase: self.phase(),
        }
    }

    pub fn uniforms(&self) -> MeshReplayUniforms {
        if !self.config.enabled {
            return MeshReplayUniforms::disabled();
        }
        MeshReplayUniforms {
            enabled: 1.0,
            phase: self.phase(),
            frame01: self.frame01(),
            opacity: self.config.opacity,
            segments: self.sequence.projected_segments(self.current_frame_index),
        }
    }

    pub fn should_emit_config_marker(&mut self) -> bool {
        if self.config_marker_emitted {
            false
        } else {
            self.config_marker_emitted = true;
            true
        }
    }

    pub fn should_emit_frame_marker(&mut self) -> bool {
        if !self.config.enabled || self.frame_markers_emitted >= MAX_FRAME_MARKERS {
            false
        } else {
            self.frame_markers_emitted += 1;
            true
        }
    }

    pub fn config_marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} enabled={} source={} sourceSchema={} sequenceId={} meshName={} animationName={} frameCount={} vertexCount={} triangleCount={} durationSeconds={:.3} speed={:.3} opacity={:.3} replayAdapter={} matterAuthority=true opticsBrowserParitySchema=true externalCaptureCommitted=false",
            REPLAY_MARKER_PREFIX,
            REPLAY_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.config.enabled { "ready" } else { "disabled" },
            self.config.enabled,
            sanitize_marker_value(&self.config.source),
            SURFACE_SEQUENCE_SCHEMA_ID,
            sanitize_marker_value(&self.sequence.sequence_id),
            sanitize_marker_value(&self.sequence.mesh_name),
            sanitize_marker_value(&self.sequence.animation_name),
            self.sequence.frame_count(),
            self.sequence.vertex_count,
            self.sequence.triangles.len(),
            self.sequence.duration_seconds,
            self.config.speed,
            self.config.opacity,
            REPLAY_ADAPTER,
        )
    }

    pub fn frame_marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status=playing sequenceId={} frameIndex={} frameCount={} playbackSeconds={:.3} playbackPhase={:.4} speed={:.3} visibleOverlay=true replayAdapter={} panelUniforms=edgeSegments4",
            REPLAY_MARKER_PREFIX,
            REPLAY_SCHEMA_ID,
            sanitize_marker_value(phase),
            sanitize_marker_value(&self.sequence.sequence_id),
            self.current_frame_index,
            self.sequence.frame_count(),
            self.playback_seconds,
            self.phase(),
            self.config.speed,
            REPLAY_ADAPTER,
        )
    }

    fn phase(&self) -> f32 {
        (self.playback_seconds as f32 / self.sequence.duration_seconds.max(0.001)).fract()
    }

    fn frame01(&self) -> f32 {
        if self.sequence.frame_count() <= 1 {
            0.0
        } else {
            self.current_frame_index as f32 / (self.sequence.frame_count() - 1) as f32
        }
    }
}

#[derive(Debug)]
struct MeshReplaySequence {
    sequence_id: String,
    mesh_name: String,
    animation_name: String,
    duration_seconds: f32,
    vertex_count: usize,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    triangles: Vec<[usize; 3]>,
    selected_edges: [[usize; 2]; SELECTED_SEGMENT_COUNT],
    frames: Vec<MeshReplayFrame>,
}

impl MeshReplaySequence {
    fn from_json_str(json: &str) -> Result<Self, MeshReplayError> {
        let value: Value =
            serde_json::from_str(json).map_err(|_| MeshReplayError::MalformedJson)?;
        let schema_id = value
            .get("schema_id")
            .and_then(Value::as_str)
            .ok_or(MeshReplayError::MissingField("schema_id"))?;
        if schema_id != SURFACE_SEQUENCE_SCHEMA_ID {
            return Err(MeshReplayError::UnexpectedSchema);
        }
        let sequence_id = required_text(&value, "sequence_id")?;
        let mesh_name = optional_text(&value, "mesh_name", "mesh");
        let animation_name = optional_text(&value, "animation_name", "animation");
        let duration_seconds = value
            .get("duration_seconds")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| value as f32)
            .ok_or(MeshReplayError::InvalidValue("duration_seconds"))?;
        let vertex_count = value
            .get("vertex_count")
            .and_then(Value::as_u64)
            .ok_or(MeshReplayError::MissingField("vertex_count"))?
            as usize;
        if vertex_count == 0 {
            return Err(MeshReplayError::InvalidValue("vertex_count"));
        }
        let bounds_min = required_vec3(&value, "bounds_min")?;
        let bounds_max = required_vec3(&value, "bounds_max")?;
        validate_bounds(bounds_min, bounds_max)?;

        let triangles_value = value
            .get("triangles")
            .and_then(Value::as_array)
            .ok_or(MeshReplayError::MissingField("triangles"))?;
        let mut triangles = Vec::with_capacity(triangles_value.len());
        for triangle in triangles_value {
            let indices = triangle
                .as_array()
                .filter(|items| items.len() == 3)
                .ok_or(MeshReplayError::InvalidValue("triangle"))?;
            let parsed = [
                parse_index(&indices[0], vertex_count)?,
                parse_index(&indices[1], vertex_count)?,
                parse_index(&indices[2], vertex_count)?,
            ];
            triangles.push(parsed);
        }
        if triangles.is_empty() {
            return Err(MeshReplayError::InvalidValue("triangles"));
        }

        let frames_value = value
            .get("frames")
            .and_then(Value::as_array)
            .ok_or(MeshReplayError::MissingField("frames"))?;
        let mut frames = Vec::with_capacity(frames_value.len());
        for frame_value in frames_value {
            frames.push(MeshReplayFrame::from_value(frame_value, vertex_count)?);
        }
        if frames.is_empty() {
            return Err(MeshReplayError::InvalidValue("frames"));
        }

        let edges = unique_edges(&triangles);
        let selected_edges = select_representative_edges(&edges, &frames);

        Ok(Self {
            sequence_id,
            mesh_name,
            animation_name,
            duration_seconds,
            vertex_count,
            bounds_min,
            bounds_max,
            triangles,
            selected_edges,
            frames,
        })
    }

    fn frame_count(&self) -> usize {
        self.frames.len()
    }

    fn frame_index_at(&self, playback_seconds: f32) -> usize {
        if self.frames.len() <= 1 {
            return 0;
        }
        let duration = self.duration_seconds.max(0.001);
        let phase = (playback_seconds / duration).fract();
        ((phase * self.frames.len() as f32).floor() as usize).min(self.frames.len() - 1)
    }

    fn projected_segments(&self, frame_index: usize) -> [[f32; 4]; SELECTED_SEGMENT_COUNT] {
        let mut segments = [[0.0; 4]; SELECTED_SEGMENT_COUNT];
        let Some(frame) = self.frames.get(frame_index) else {
            return segments;
        };
        for (slot, segment) in segments.iter_mut().enumerate() {
            let [start_index, end_index] = self.selected_edges[slot];
            let start = self.project_position(frame.positions[start_index]);
            let end = self.project_position(frame.positions[end_index]);
            *segment = [start[0], start[1], end[0], end[1]];
        }
        segments
    }

    fn project_position(&self, position: [f32; 3]) -> [f32; 2] {
        let extent_x = (self.bounds_max[0] - self.bounds_min[0]).max(0.0001);
        let extent_y = (self.bounds_max[1] - self.bounds_min[1]).max(0.0001);
        let normalized_x = (position[0] - self.bounds_min[0]) / extent_x;
        let normalized_y = (position[1] - self.bounds_min[1]) / extent_y;
        [
            ((normalized_x - 0.5) * 0.72 + 0.5).clamp(0.04, 0.96),
            ((1.0 - normalized_y - 0.5) * 0.72 + 0.5).clamp(0.04, 0.96),
        ]
    }
}

#[derive(Debug)]
struct MeshReplayFrame {
    positions: Vec<[f32; 3]>,
}

impl MeshReplayFrame {
    fn from_value(value: &Value, vertex_count: usize) -> Result<Self, MeshReplayError> {
        let positions_value = value
            .get("positions")
            .and_then(Value::as_array)
            .ok_or(MeshReplayError::MissingField("positions"))?;
        if positions_value.len() != vertex_count {
            return Err(MeshReplayError::InvalidValue("positions"));
        }
        let mut positions = Vec::with_capacity(vertex_count);
        for position in positions_value {
            positions.push(parse_vec3_value(position)?);
        }
        Ok(Self { positions })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MeshReplayError {
    MalformedJson,
    MissingField(&'static str),
    UnexpectedSchema,
    InvalidValue(&'static str),
    IndexOutOfRange,
}

fn required_text(value: &Value, field: &'static str) -> Result<String, MeshReplayError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or(MeshReplayError::MissingField(field))
}

fn optional_text(value: &Value, field: &'static str, default: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn required_vec3(value: &Value, field: &'static str) -> Result<[f32; 3], MeshReplayError> {
    value
        .get(field)
        .ok_or(MeshReplayError::MissingField(field))
        .and_then(parse_vec3_value)
}

fn parse_vec3_value(value: &Value) -> Result<[f32; 3], MeshReplayError> {
    if let Some(array) = value.as_array() {
        if array.len() != 3 {
            return Err(MeshReplayError::InvalidValue("vec3"));
        }
        return Ok([
            parse_f32(&array[0], "vec3.x")?,
            parse_f32(&array[1], "vec3.y")?,
            parse_f32(&array[2], "vec3.z")?,
        ]);
    }
    Ok([
        parse_f32(
            value
                .get("x")
                .ok_or(MeshReplayError::MissingField("vec3.x"))?,
            "vec3.x",
        )?,
        parse_f32(
            value
                .get("y")
                .ok_or(MeshReplayError::MissingField("vec3.y"))?,
            "vec3.y",
        )?,
        parse_f32(
            value
                .get("z")
                .ok_or(MeshReplayError::MissingField("vec3.z"))?,
            "vec3.z",
        )?,
    ])
}

fn parse_f32(value: &Value, field: &'static str) -> Result<f32, MeshReplayError> {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map(|number| number as f32)
        .ok_or(MeshReplayError::InvalidValue(field))
}

fn parse_index(value: &Value, vertex_count: usize) -> Result<usize, MeshReplayError> {
    let index = value
        .as_u64()
        .ok_or(MeshReplayError::InvalidValue("triangle index"))? as usize;
    if index >= vertex_count {
        Err(MeshReplayError::IndexOutOfRange)
    } else {
        Ok(index)
    }
}

fn validate_bounds(minimum: [f32; 3], maximum: [f32; 3]) -> Result<(), MeshReplayError> {
    if (0..3).any(|axis| {
        !minimum[axis].is_finite() || !maximum[axis].is_finite() || maximum[axis] <= minimum[axis]
    }) {
        Err(MeshReplayError::InvalidValue("bounds"))
    } else {
        Ok(())
    }
}

fn unique_edges(triangles: &[[usize; 3]]) -> Vec<[usize; 2]> {
    let mut edges = Vec::new();
    for triangle in triangles {
        for [a, b] in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            let edge = if a <= b { [a, b] } else { [b, a] };
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
    }
    edges
}

fn select_representative_edges(
    edges: &[[usize; 2]],
    frames: &[MeshReplayFrame],
) -> [[usize; 2]; SELECTED_SEGMENT_COUNT] {
    let mut selected = [[0, 0]; SELECTED_SEGMENT_COUNT];
    if edges.is_empty() || frames.is_empty() {
        return selected;
    }

    let positions = &frames[0].positions;
    let mut scored_edges: Vec<(usize, f32)> = edges
        .iter()
        .enumerate()
        .map(|(index, [start_index, end_index])| {
            let start = positions.get(*start_index).copied().unwrap_or([0.0; 3]);
            let end = positions.get(*end_index).copied().unwrap_or([0.0; 3]);
            let length_score = distance_squared(start, end);
            let motion_score = edge_motion_score(*start_index, *end_index, frames);
            (index, motion_score * 10.0 + length_score)
        })
        .collect();
    scored_edges.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    for (slot, selected_edge) in selected.iter_mut().enumerate() {
        let edge_index = scored_edges[slot % scored_edges.len()].0;
        *selected_edge = edges[edge_index];
    }
    selected
}

fn edge_motion_score(start_index: usize, end_index: usize, frames: &[MeshReplayFrame]) -> f32 {
    let Some(first) = frames.first() else {
        return 0.0;
    };
    let Some(start_first) = first.positions.get(start_index).copied() else {
        return 0.0;
    };
    let Some(end_first) = first.positions.get(end_index).copied() else {
        return 0.0;
    };

    frames
        .iter()
        .map(|frame| {
            let start = frame
                .positions
                .get(start_index)
                .copied()
                .unwrap_or(start_first);
            let end = frame.positions.get(end_index).copied().unwrap_or(end_first);
            distance_squared(start_first, start) + distance_squared(end_first, end)
        })
        .fold(0.0, f32::max)
}

fn distance_squared(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = right[0] - left[0];
    let dy = right[1] - left[1];
    let dz = right[2] - left[2];
    dx * dx + dy * dy + dz * dz
}

fn sanitize_marker_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sequence_parses_with_browser_schema() {
        let sequence = MeshReplaySequence::from_json_str(DEFAULT_SEQUENCE_JSON).unwrap();
        assert_eq!(sequence.frame_count(), 4);
        assert_eq!(sequence.vertex_count, 8);
        assert_eq!(sequence.triangles.len(), 6);
        assert_eq!(sequence.frame_index_at(0.0), 0);
        assert_eq!(sequence.frame_index_at(0.61), 2);
    }

    #[test]
    fn runtime_advances_and_projects_visible_segments() {
        let mut runtime = MeshReplayRuntime::default();
        runtime.configure(MeshReplayConfig::normalized(
            true,
            "public-synthetic-hand-sequence".to_string(),
            1.0,
            0.75,
        ));
        let first = runtime.step(0.0);
        assert!(first.enabled);
        assert_eq!(first.frame_index, 0);
        let first_uniforms = runtime.uniforms();
        assert_eq!(first_uniforms.enabled, 1.0);
        assert_eq!(first_uniforms.opacity, 0.75);
        assert!(first_uniforms.segments[0][0] > 0.0);

        let second = runtime.step(0.7);
        assert!(second.changed_frame);
        assert_eq!(second.frame_index, 2);
        let second_uniforms = runtime.uniforms();
        assert_ne!(first_uniforms.segments, second_uniforms.segments);
    }

    #[test]
    fn parser_rejects_wrong_schema_and_bad_indices() {
        let wrong_schema =
            DEFAULT_SEQUENCE_JSON.replace(SURFACE_SEQUENCE_SCHEMA_ID, "rusty.bad.schema");
        assert_eq!(
            MeshReplaySequence::from_json_str(&wrong_schema).unwrap_err(),
            MeshReplayError::UnexpectedSchema
        );

        let bad_index = DEFAULT_SEQUENCE_JSON.replace("[2, 6, 4]", "[2, 99, 4]");
        assert_eq!(
            MeshReplaySequence::from_json_str(&bad_index).unwrap_err(),
            MeshReplayError::IndexOutOfRange
        );
    }

    #[test]
    fn disabled_runtime_returns_empty_uniforms() {
        let mut runtime = MeshReplayRuntime::default();
        runtime.step(1.0);
        assert_eq!(runtime.uniforms(), MeshReplayUniforms::disabled());
    }

    #[test]
    fn markers_use_quest_adapter_lane_and_matter_source_schema() {
        let mut runtime = MeshReplayRuntime::default();
        runtime.configure(MeshReplayConfig::normalized(
            true,
            "public-synthetic-hand-sequence".to_string(),
            1.0,
            0.75,
        ));
        runtime.step(0.0);

        let config_marker = runtime.config_marker_line("startup");
        assert!(config_marker.starts_with("RUSTY_QUEST_MAKEPAD_MESH_REPLAY "));
        assert!(config_marker.contains("schema=rusty.quest.makepad.mesh_replay.v1"));
        assert!(
            config_marker.contains("sourceSchema=rusty.matter.tools.glb_mesh_surface_sequence.v1")
        );
        assert!(!config_marker.contains("RUSTY_XR"));
        assert!(!config_marker.contains("rusty.xr"));

        let frame_marker = runtime.frame_marker_line("xr-update");
        assert!(frame_marker.starts_with("RUSTY_QUEST_MAKEPAD_MESH_REPLAY "));
        assert!(frame_marker.contains("schema=rusty.quest.makepad.mesh_replay.v1"));
        assert!(!frame_marker.contains("RUSTY_XR"));
        assert!(!frame_marker.contains("rusty.xr"));
    }
}
