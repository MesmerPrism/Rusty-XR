use openxr as xr;
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, OpenXrGlesFeasibilityState, OpenXrGlesFeasibilityStatus,
};
use std::time::Instant;

use super::{log_error, log_info, log_status, VIEW_TYPE};

pub(super) struct OesFrameRateTracker {
    frame_window_start: Instant,
    frame_window_count: u64,
}

impl OesFrameRateTracker {
    pub(super) fn new() -> Self {
        Self {
            frame_window_start: Instant::now(),
            frame_window_count: 0,
        }
    }

    pub(super) fn record_rendered_frame(
        &mut self,
        frame_count: u64,
        status: &mut OpenXrGlesFeasibilityStatus,
    ) {
        self.frame_window_count = self.frame_window_count.saturating_add(1);
        if status.state != OpenXrGlesFeasibilityState::Rendering && frame_count > 0 {
            status.state = OpenXrGlesFeasibilityState::Rendering;
        }
        if frame_count == 1 || frame_count.is_multiple_of(120) {
            let elapsed = self.frame_window_start.elapsed().as_secs_f32().max(0.001);
            let fps = self.frame_window_count as f32 / elapsed;
            status.frame_rate = Some(FrameRateSummary {
                sample_count: frame_count,
                average_fps: fps,
                min_fps: fps,
                max_fps: fps,
            });
            log_info(format!(
                "Rusty XR OpenXR GLES frame frame={} observedOpenXrFps={:.1} iteration2Ready={}",
                frame_count,
                fps,
                status.is_iteration2_ready()
            ));
            log_status(status);
            self.frame_window_start = Instant::now();
            self.frame_window_count = 0;
        }
    }
}

pub(super) fn poll_openxr_session_events(
    instance: &xr::Instance,
    session: &xr::Session<xr::OpenGlEs>,
    event_storage: &mut xr::EventDataBuffer,
    session_running: &mut bool,
) -> Result<bool, String> {
    while let Some(event) = instance
        .poll_event(event_storage)
        .map_err(|error| format!("poll OpenXR event: {error}"))?
    {
        match event {
            xr::Event::SessionStateChanged(event) => match event.state() {
                xr::SessionState::READY => {
                    session
                        .begin(VIEW_TYPE)
                        .map_err(|error| format!("begin OpenXR session: {error}"))?;
                    *session_running = true;
                    log_info("Rusty XR OpenXR GLES state READY -> running");
                }
                xr::SessionState::STOPPING => {
                    session
                        .end()
                        .map_err(|error| format!("end OpenXR session: {error}"))?;
                    *session_running = false;
                    log_info("Rusty XR OpenXR GLES state STOPPING -> ended");
                }
                xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                    return Ok(true);
                }
                state => {
                    log_info(format!("Rusty XR OpenXR GLES state {state:?}"));
                }
            },
            xr::Event::InstanceLossPending(_) => return Ok(true),
            xr::Event::EventsLost(event) => {
                log_error(format!(
                    "Rusty XR OpenXR GLES lost {} event(s)",
                    event.lost_event_count()
                ));
            }
            _ => {}
        }
    }
    Ok(false)
}
