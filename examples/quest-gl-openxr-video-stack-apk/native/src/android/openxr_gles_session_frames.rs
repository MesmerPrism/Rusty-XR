use openxr as xr;
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, OpenXrGlesFeasibilityState, OpenXrGlesFeasibilityStatus,
};
use std::time::Instant;

use super::{log_info, log_status, VIEW_TYPE};

pub(super) enum OesLocatedViews {
    SubmitValid(Vec<xr::View>),
    NotSubmitValid(xr::ViewStateFlags),
}

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

pub(super) fn begin_openxr_frame(
    frame_wait: &mut xr::FrameWaiter,
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
) -> Result<xr::FrameState, String> {
    let frame_state = frame_wait
        .wait()
        .map_err(|error| format!("wait OpenXR frame: {error}"))?;
    frame_stream
        .begin()
        .map_err(|error| format!("begin OpenXR frame: {error}"))?;
    Ok(frame_state)
}

pub(super) fn locate_submit_valid_views(
    session: &xr::Session<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    stage: &xr::Space,
) -> Result<OesLocatedViews, String> {
    let (view_state_flags, views) = session
        .locate_views(VIEW_TYPE, predicted_display_time, stage)
        .map_err(|error| format!("locate OpenXR views: {error}"))?;
    let views_valid = view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        && view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID)
        && views.iter().all(view_pose_is_submit_valid);
    if views_valid {
        Ok(OesLocatedViews::SubmitValid(views))
    } else {
        Ok(OesLocatedViews::NotSubmitValid(view_state_flags))
    }
}

fn view_pose_is_submit_valid(view: &xr::View) -> bool {
    let pose = view.pose;
    let values = [
        pose.position.x,
        pose.position.y,
        pose.position.z,
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let orientation_norm_squared = pose.orientation.x * pose.orientation.x
        + pose.orientation.y * pose.orientation.y
        + pose.orientation.z * pose.orientation.z
        + pose.orientation.w * pose.orientation.w;
    orientation_norm_squared.is_finite() && orientation_norm_squared > 0.0
}
