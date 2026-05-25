use openxr as xr;

use super::{log_error, log_info, VIEW_TYPE};

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
