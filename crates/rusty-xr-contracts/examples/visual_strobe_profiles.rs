//! Synthetic visual strobe profile catalog.
//!
//! This example describes intentional visual stimuli only. It does not create
//! OpenXR sessions, request display refresh rates, draw frames, or bypass the
//! safety gate that a real application should put in front of strobing content.

use rusty_xr_contracts::{
    StrobeFrequencyPlan, VisualStrobeProfile, FULL_FIELD_STROBE_WARNING,
    PHOTOSENSITIVE_RISK_BAND_MAX_HZ, PHOTOSENSITIVE_RISK_BAND_MIN_HZ, WCAG_GENERAL_FLASH_LIMIT_HZ,
    XR_FB_DISPLAY_REFRESH_RATE_EXTENSION,
};
use serde::Serialize;

#[derive(Serialize)]
struct SourceLink<'a> {
    label: &'a str,
    url: &'a str,
}

#[derive(Serialize)]
struct CatalogProfile {
    id: String,
    label: String,
    descriptor: VisualStrobeProfile,
    plan_at_120_hz: StrobeFrequencyPlan,
    notes: Vec<&'static str>,
}

#[derive(Serialize)]
struct StrobeCatalog<'a> {
    description: &'a str,
    warning: &'a str,
    risk_band_hz: [f32; 2],
    accessibility_flash_limit_hz: f32,
    display_refresh_extension: &'a str,
    source_links: Vec<SourceLink<'a>>,
    profiles: Vec<CatalogProfile>,
}

fn profile_notes(target_cycle_hz: f32) -> Vec<&'static str> {
    match target_cycle_hz as u32 {
        10 => vec![
            "At 120 Hz display refresh, 10 Hz full cycles use six frames per half-cycle.",
            "This is still an explicit strobe stimulus and exceeds general accessibility flash limits.",
        ],
        40 => vec![
            "At 120 Hz display refresh, 40 Hz full cycles average 1.5 frames per half-cycle.",
            "A native adapter can hit the average switch rate only by quantizing half-cycles across one- and two-frame intervals.",
        ],
        60 => vec![
            "At 120 Hz display refresh, 60 Hz full cycles require a state change every displayed frame.",
            "Any missed frame, runtime reprojection, or refresh-rate fallback will visibly change the delivered stimulus.",
        ],
        _ => vec!["Synthetic profile."],
    }
}

fn main() {
    let frequencies = [10.0, 40.0, 60.0];
    let mut profiles = Vec::new();

    for frequency in frequencies {
        let descriptor = VisualStrobeProfile::full_field_red_black(frequency, 120.0);
        let plan_at_120_hz = descriptor
            .timing_plan(120.0)
            .expect("synthetic strobe profile should have a timing plan");
        profiles.push(CatalogProfile {
            id: format!("full-field-red-black-{frequency:.0}hz"),
            label: format!("Full-field red/black flicker {frequency:.0} Hz"),
            descriptor,
            plan_at_120_hz,
            notes: profile_notes(frequency),
        });
    }

    for frequency in frequencies {
        let descriptor = VisualStrobeProfile::passthrough_phase_inverted_lut(
            frequency,
            120.0,
            "opponent-gradient",
            "opponent-gradient-half-phase",
        );
        let plan_at_120_hz = descriptor
            .timing_plan(120.0)
            .expect("synthetic strobe profile should have a timing plan");
        profiles.push(CatalogProfile {
            id: format!("passthrough-lut-phase-inverted-{frequency:.0}hz"),
            label: format!("Passthrough LUT phase-inverted flicker {frequency:.0} Hz"),
            descriptor,
            plan_at_120_hz,
            notes: profile_notes(frequency),
        });
    }

    let catalog = StrobeCatalog {
        description: "Contracts-only catalog for intentional visual strobe profiles.",
        warning: FULL_FIELD_STROBE_WARNING,
        risk_band_hz: [
            PHOTOSENSITIVE_RISK_BAND_MIN_HZ,
            PHOTOSENSITIVE_RISK_BAND_MAX_HZ,
        ],
        accessibility_flash_limit_hz: WCAG_GENERAL_FLASH_LIMIT_HZ,
        display_refresh_extension: XR_FB_DISPLAY_REFRESH_RATE_EXTENSION,
        source_links: vec![
            SourceLink {
                label: "W3C WCAG flashes guidance",
                url: "https://www.w3.org/WAI/WCAG22/Understanding/three-flashes",
            },
            SourceLink {
                label: "Epilepsy Foundation photosensitivity guidance",
                url: "https://www.epilepsy.com/what-is-epilepsy/seizure-triggers/photosensitivity",
            },
            SourceLink {
                label: "OpenXR XR_FB_display_refresh_rate",
                url: "https://registry.khronos.org/OpenXR/specs/1.0/man/html/XR_FB_display_refresh_rate.html",
            },
        ],
        profiles,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&catalog).expect("catalog should serialize")
    );
}
