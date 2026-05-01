use crate::ColorRgba;

/// OpenXR extension name for runtime display-refresh requests.
pub const XR_FB_DISPLAY_REFRESH_RATE_EXTENSION: &str = "XR_FB_display_refresh_rate";

/// Accessibility-oriented flash limit used by WCAG for general web content.
///
/// Intentional XR strobe stimuli can exceed this limit by design. Profiles that
/// do so should require explicit operator acknowledgement and should not be
/// treated as normal UI animation.
pub const WCAG_GENERAL_FLASH_LIMIT_HZ: f32 = 3.0;

/// Lower bound of the common photosensitive-seizure risk band cited by public
/// epilepsy guidance. Individual sensitivity can fall outside this band.
pub const PHOTOSENSITIVE_RISK_BAND_MIN_HZ: f32 = 5.0;

/// Upper bound of the common photosensitive-seizure risk band cited by public
/// epilepsy guidance. Individual sensitivity can fall outside this band.
pub const PHOTOSENSITIVE_RISK_BAND_MAX_HZ: f32 = 30.0;

/// Public warning text for examples that intentionally flash large visual areas.
pub const FULL_FIELD_STROBE_WARNING: &str = "WARNING: This profile uses intense full-field flashing light. It can trigger seizures or other adverse reactions in people with photosensitive epilepsy or other light-sensitive conditions, and it may also cause headache, nausea, dizziness, migraine, eyestrain, anxiety, or discomfort. Use only with explicit informed opt-in, stop immediately if symptoms occur, and do not present it as a treatment or wellness claim.";

/// Source that produces the visual state changes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum VisualStrobeMode {
    /// Alternate a rendered projection layer between two full-field colors.
    FullFieldColorAlternation {
        first_color: ColorRgba,
        second_color: ColorRgba,
    },
    /// Alternate between two runtime-created passthrough color LUT handles.
    PassthroughLutAlternation {
        first_lut_id: String,
        second_lut_id: String,
        weight: f32,
    },
}

impl VisualStrobeMode {
    pub const fn full_field_color_alternation(
        first_color: ColorRgba,
        second_color: ColorRgba,
    ) -> Self {
        Self::FullFieldColorAlternation {
            first_color,
            second_color,
        }
    }

    pub fn passthrough_lut_alternation(
        first_lut_id: impl Into<String>,
        second_lut_id: impl Into<String>,
        weight: f32,
    ) -> Self {
        Self::PassthroughLutAlternation {
            first_lut_id: first_lut_id.into(),
            second_lut_id: second_lut_id.into(),
            weight,
        }
    }

    pub fn stable_id(&self) -> &'static str {
        match self {
            Self::FullFieldColorAlternation { .. } => "full-field-color-alternation",
            Self::PassthroughLutAlternation { .. } => "passthrough-lut-alternation",
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::FullFieldColorAlternation {
                first_color,
                second_color,
            } => color_is_unit(*first_color) && color_is_unit(*second_color),
            Self::PassthroughLutAlternation {
                first_lut_id,
                second_lut_id,
                weight,
            } => {
                !first_lut_id.trim().is_empty()
                    && !second_lut_id.trim().is_empty()
                    && first_lut_id != second_lut_id
                    && unit(*weight)
            }
        }
    }
}

/// Safety classification for an intentional visual strobe profile.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisualStrobeSafetyClass {
    /// No repeated high-contrast flashing is requested.
    #[default]
    NotStrobing,
    /// Flashing is at or below the WCAG general flash threshold.
    AccessibilityBounded,
    /// Flashing exceeds general accessibility limits and requires explicit gatekeeping.
    ResearchStimulus,
}

impl VisualStrobeSafetyClass {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::NotStrobing => "not-strobing",
            Self::AccessibilityBounded => "accessibility-bounded",
            Self::ResearchStimulus => "research-stimulus",
        }
    }

    pub const fn requires_explicit_acknowledgement(self) -> bool {
        matches!(self, Self::ResearchStimulus)
    }
}

/// Portable descriptor for a visual strobe stimulus.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct VisualStrobeProfile {
    pub mode: VisualStrobeMode,
    /// Full A/B cycles per second. One cycle contains both visual states.
    pub target_cycle_hz: f32,
    /// Fraction of each cycle spent in the first state.
    pub duty_cycle: f32,
    /// Optional runtime display-refresh request for adapters that expose
    /// `XR_FB_display_refresh_rate`.
    pub display_refresh_request_hz: Option<f32>,
    pub safety_class: VisualStrobeSafetyClass,
}

impl VisualStrobeProfile {
    pub fn new(
        mode: VisualStrobeMode,
        target_cycle_hz: f32,
        display_refresh_request_hz: Option<f32>,
    ) -> Self {
        Self {
            mode,
            target_cycle_hz,
            duty_cycle: 0.5,
            display_refresh_request_hz,
            safety_class: safety_class_for_cycle_hz(target_cycle_hz),
        }
    }

    pub fn full_field_red_black(target_cycle_hz: f32, display_refresh_request_hz: f32) -> Self {
        Self::new(
            VisualStrobeMode::full_field_color_alternation(
                ColorRgba::new(1.0, 0.0, 0.0, 1.0),
                ColorRgba::new(0.0, 0.0, 0.0, 1.0),
            ),
            target_cycle_hz,
            Some(display_refresh_request_hz),
        )
    }

    pub fn passthrough_phase_inverted_lut(
        target_cycle_hz: f32,
        display_refresh_request_hz: f32,
        first_lut_id: impl Into<String>,
        second_lut_id: impl Into<String>,
    ) -> Self {
        Self::new(
            VisualStrobeMode::passthrough_lut_alternation(first_lut_id, second_lut_id, 1.0),
            target_cycle_hz,
            Some(display_refresh_request_hz),
        )
    }

    pub fn timing_plan(&self, display_refresh_hz: f32) -> Option<StrobeFrequencyPlan> {
        StrobeFrequencyPlan::analyze(self.target_cycle_hz, display_refresh_hz)
    }

    pub fn is_valid(&self) -> bool {
        self.mode.is_valid()
            && self.target_cycle_hz.is_finite()
            && self.target_cycle_hz >= 0.0
            && self.duty_cycle.is_finite()
            && self.duty_cycle > 0.0
            && self.duty_cycle < 1.0
            && match self.display_refresh_request_hz {
                Some(hz) => hz.is_finite() && hz > 0.0,
                None => true,
            }
    }
}

/// Display-frame feasibility for a requested square-wave strobe frequency.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrobeFrameFeasibility {
    /// No flashing was requested.
    #[default]
    NotStrobing,
    /// Each half-cycle lands on an integer number of display frames.
    ExactHalfCycleFrames,
    /// The requested average switch rate is possible, but half-cycles must
    /// alternate frame counts or jitter against display cadence.
    FrameQuantizedAverageOnly,
    /// The requested switch rate is above one state change per display frame.
    ExceedsDisplaySwitchBudget,
}

impl StrobeFrameFeasibility {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::NotStrobing => "not-strobing",
            Self::ExactHalfCycleFrames => "exact-half-cycle-frames",
            Self::FrameQuantizedAverageOnly => "frame-quantized-average-only",
            Self::ExceedsDisplaySwitchBudget => "exceeds-display-switch-budget",
        }
    }
}

/// Derived timing facts for a target strobe frequency at a display refresh rate.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrobeFrequencyPlan {
    pub target_cycle_hz: f32,
    pub target_switch_hz: f32,
    pub display_refresh_hz: f32,
    pub frames_per_cycle: f32,
    pub frames_per_half_cycle: f32,
    pub feasibility: StrobeFrameFeasibility,
    pub requires_every_frame_toggle: bool,
}

impl StrobeFrequencyPlan {
    pub fn analyze(target_cycle_hz: f32, display_refresh_hz: f32) -> Option<Self> {
        if !target_cycle_hz.is_finite()
            || !display_refresh_hz.is_finite()
            || target_cycle_hz < 0.0
            || display_refresh_hz <= 0.0
        {
            return None;
        }

        if target_cycle_hz == 0.0 {
            return Some(Self {
                target_cycle_hz,
                target_switch_hz: 0.0,
                display_refresh_hz,
                frames_per_cycle: f32::INFINITY,
                frames_per_half_cycle: f32::INFINITY,
                feasibility: StrobeFrameFeasibility::NotStrobing,
                requires_every_frame_toggle: false,
            });
        }

        let target_switch_hz = target_cycle_hz * 2.0;
        let frames_per_cycle = display_refresh_hz / target_cycle_hz;
        let frames_per_half_cycle = display_refresh_hz / target_switch_hz;
        let requires_every_frame_toggle = approx_equal(frames_per_half_cycle, 1.0);
        let feasibility = if target_switch_hz > display_refresh_hz + 1.0e-4 {
            StrobeFrameFeasibility::ExceedsDisplaySwitchBudget
        } else if approx_integer(frames_per_half_cycle) {
            StrobeFrameFeasibility::ExactHalfCycleFrames
        } else {
            StrobeFrameFeasibility::FrameQuantizedAverageOnly
        };

        Some(Self {
            target_cycle_hz,
            target_switch_hz,
            display_refresh_hz,
            frames_per_cycle,
            frames_per_half_cycle,
            feasibility,
            requires_every_frame_toggle,
        })
    }

    pub fn is_display_representable(self) -> bool {
        !matches!(
            self.feasibility,
            StrobeFrameFeasibility::ExceedsDisplaySwitchBudget
        )
    }
}

pub fn safety_class_for_cycle_hz(target_cycle_hz: f32) -> VisualStrobeSafetyClass {
    if !target_cycle_hz.is_finite() || target_cycle_hz <= 0.0 {
        VisualStrobeSafetyClass::NotStrobing
    } else if target_cycle_hz <= WCAG_GENERAL_FLASH_LIMIT_HZ {
        VisualStrobeSafetyClass::AccessibilityBounded
    } else {
        VisualStrobeSafetyClass::ResearchStimulus
    }
}

fn approx_integer(value: f32) -> bool {
    approx_equal(value, value.round())
}

fn approx_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= 1.0e-4
}

fn color_is_unit(color: ColorRgba) -> bool {
    unit(color.r) && unit(color.g) && unit(color.b) && unit(color.a)
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
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
    fn strobe_profiles_classify_safety() {
        assert_eq!(
            safety_class_for_cycle_hz(0.0),
            VisualStrobeSafetyClass::NotStrobing
        );
        assert_eq!(
            safety_class_for_cycle_hz(3.0),
            VisualStrobeSafetyClass::AccessibilityBounded
        );
        assert_eq!(
            safety_class_for_cycle_hz(10.0),
            VisualStrobeSafetyClass::ResearchStimulus
        );
    }

    #[test]
    fn full_field_profile_is_valid_and_gated() {
        let profile = VisualStrobeProfile::full_field_red_black(10.0, 120.0);

        assert!(profile.is_valid());
        assert_eq!(profile.mode.stable_id(), "full-field-color-alternation");
        assert!(profile.safety_class.requires_explicit_acknowledgement());
    }

    #[test]
    fn display_plan_marks_exact_and_quantized_frequencies() {
        let ten = StrobeFrequencyPlan::analyze(10.0, 120.0).expect("valid plan");
        assert_eq!(
            ten.feasibility,
            StrobeFrameFeasibility::ExactHalfCycleFrames
        );
        near(ten.frames_per_half_cycle, 6.0);

        let forty = StrobeFrequencyPlan::analyze(40.0, 120.0).expect("valid plan");
        assert_eq!(
            forty.feasibility,
            StrobeFrameFeasibility::FrameQuantizedAverageOnly
        );
        near(forty.frames_per_half_cycle, 1.5);

        let sixty = StrobeFrequencyPlan::analyze(60.0, 120.0).expect("valid plan");
        assert_eq!(
            sixty.feasibility,
            StrobeFrameFeasibility::ExactHalfCycleFrames
        );
        assert!(sixty.requires_every_frame_toggle);
        near(sixty.frames_per_half_cycle, 1.0);
    }

    #[test]
    fn display_plan_rejects_switch_rate_above_frame_rate() {
        let plan = StrobeFrequencyPlan::analyze(60.0, 72.0).expect("valid plan");

        assert_eq!(
            plan.feasibility,
            StrobeFrameFeasibility::ExceedsDisplaySwitchBudget
        );
        assert!(!plan.is_display_representable());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn strobe_profile_round_trips_with_serde() {
        let profile = VisualStrobeProfile::passthrough_phase_inverted_lut(
            40.0,
            120.0,
            "opponent-a",
            "opponent-b",
        );

        let encoded = serde_json::to_string(&profile).expect("profile should serialize");
        let decoded: VisualStrobeProfile =
            serde_json::from_str(&encoded).expect("profile should deserialize");

        assert_eq!(decoded, profile);
    }
}
