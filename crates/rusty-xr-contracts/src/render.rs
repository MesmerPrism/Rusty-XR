use crate::Vec3;

/// Linear RGBA color.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
    }
}

impl Default for ColorRgba {
    fn default() -> Self {
        Self::WHITE
    }
}

/// Coordinate space for render payload positions.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderCoordinateSpace {
    #[default]
    World,
    View,
    Local,
}

/// Generic point-like render primitive payload.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderPoint {
    pub position: Vec3,
    pub radius_meters: f32,
    pub color: ColorRgba,
    pub normal: Vec3,
    pub flags: u32,
}

impl RenderPoint {
    pub fn new(position: Vec3, radius_meters: f32, color: ColorRgba) -> Self {
        Self {
            position,
            radius_meters,
            color,
            normal: Vec3::UP,
            flags: 0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && self.normal.is_finite()
            && self.radius_meters.is_finite()
            && self.radius_meters >= 0.0
            && self.color.is_finite()
    }
}

/// Generic runtime counter value.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum CounterValue {
    Count(u64),
    Seconds(f64),
    Ratio(f32),
    Flag(bool),
    Text(String),
}

/// Named counter sample without app-specific semantics.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CounterSample {
    pub name: String,
    pub value: CounterValue,
}

impl CounterSample {
    pub fn new(name: impl Into<String>, value: CounterValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// Runtime counters associated with a frame or worker publication.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeCounters {
    pub frame_index: u64,
    pub samples: Vec<CounterSample>,
}

impl RuntimeCounters {
    pub const fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            samples: Vec::new(),
        }
    }

    pub fn push_count(&mut self, name: impl Into<String>, value: u64) {
        self.samples
            .push(CounterSample::new(name, CounterValue::Count(value)));
    }

    pub fn push_seconds(&mut self, name: impl Into<String>, value: f64) {
        self.samples
            .push(CounterSample::new(name, CounterValue::Seconds(value)));
    }

    pub fn get(&self, name: &str) -> Option<&CounterValue> {
        self.samples
            .iter()
            .find(|sample| sample.name == name)
            .map(|sample| &sample.value)
    }
}

/// Render payload produced by an experiment or app subsystem.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderPayload {
    pub frame_index: u64,
    pub coordinate_space: RenderCoordinateSpace,
    pub points: Vec<RenderPoint>,
    pub counters: RuntimeCounters,
}

impl RenderPayload {
    pub fn new(frame_index: u64, coordinate_space: RenderCoordinateSpace) -> Self {
        Self {
            frame_index,
            coordinate_space,
            points: Vec::new(),
            counters: RuntimeCounters::new(frame_index),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.iter().copied().all(RenderPoint::is_valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_counters_can_lookup_named_values() {
        let mut counters = RuntimeCounters::new(4);
        counters.push_count("draw_calls", 3);

        assert_eq!(counters.get("draw_calls"), Some(&CounterValue::Count(3)));
        assert_eq!(counters.get("missing"), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn render_payload_round_trips_with_serde() {
        let mut payload = RenderPayload::new(12, RenderCoordinateSpace::World);
        payload
            .points
            .push(RenderPoint::new(Vec3::UP, 0.03, ColorRgba::WHITE));
        payload.counters.push_count("points", 1);

        let encoded = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: RenderPayload =
            serde_json::from_str(&encoded).expect("payload should deserialize");

        assert_eq!(decoded, payload);
    }
}
