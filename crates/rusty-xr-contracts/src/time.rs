/// Timing metadata for one XR/application frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTiming {
    pub frame_index: u64,
    pub frame_start_time_ns: Option<u64>,
    pub predicted_display_time_ns: Option<u64>,
    pub delta_seconds: f32,
}

impl FrameTiming {
    pub const fn new(frame_index: u64, delta_seconds: f32) -> Self {
        Self {
            frame_index,
            frame_start_time_ns: None,
            predicted_display_time_ns: None,
            delta_seconds,
        }
    }

    pub fn with_predicted_display_time_ns(mut self, predicted_display_time_ns: u64) -> Self {
        self.predicted_display_time_ns = Some(predicted_display_time_ns);
        self
    }

    pub fn with_frame_start_time_ns(mut self, frame_start_time_ns: u64) -> Self {
        self.frame_start_time_ns = Some(frame_start_time_ns);
        self
    }

    pub fn is_valid(self) -> bool {
        self.delta_seconds.is_finite() && self.delta_seconds >= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_timing_rejects_negative_delta() {
        assert!(!FrameTiming::new(7, -0.01).is_valid());
        assert!(FrameTiming::new(7, 1.0 / 72.0).is_valid());
    }
}
