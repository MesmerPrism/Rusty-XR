use android_activity::{InputStatus, MainEvent, PollEvent};
use std::time::{Duration, Instant};

use super::{log_error, log_info};

pub(super) fn wait_for_android_foreground(
    app: &android_activity::AndroidApp,
) -> Result<(), String> {
    let start = Instant::now();
    let mut state = AndroidForegroundState::default();
    log_info("Rusty XR GLES waiting for Android resume/focus before OpenXR session setup");

    loop {
        app.poll_events(Some(Duration::from_millis(50)), |event| {
            if let PollEvent::Main(main_event) = event {
                handle_android_main_event(app, main_event, Some(&mut state), None);
            }
        });

        if state.destroyed {
            return Err("Android activity was destroyed before OpenXR setup".to_string());
        }
        if state.resumed && state.focused && state.has_window {
            log_info("Rusty XR GLES Android activity is foreground; continuing OpenXR setup");
            return Ok(());
        }
        if start.elapsed() >= Duration::from_secs(10) {
            log_error(
                "Timed out waiting for Android focus before GLES/OpenXR setup; continuing best-effort",
            );
            return Ok(());
        }
    }
}

pub(super) fn pump_android_events(app: &android_activity::AndroidApp, running: &mut bool) {
    app.poll_events(Some(Duration::from_millis(0)), |event| {
        if let PollEvent::Main(main_event) = event {
            handle_android_main_event(app, main_event, None, Some(running));
        }
    });
}

#[derive(Default)]
struct AndroidForegroundState {
    resumed: bool,
    focused: bool,
    has_window: bool,
    destroyed: bool,
}

fn handle_android_main_event(
    app: &android_activity::AndroidApp,
    event: MainEvent<'_>,
    mut foreground: Option<&mut AndroidForegroundState>,
    running: Option<&mut bool>,
) {
    match event {
        MainEvent::InputAvailable => {
            drain_input_events(app);
        }
        MainEvent::InitWindow { .. } => {
            log_info("Rusty XR GLES Android native window initialized");
            if let Some(state) = foreground.as_mut() {
                state.has_window = true;
            }
        }
        MainEvent::TerminateWindow { .. } => {
            log_info("Rusty XR GLES Android native window terminated");
            if let Some(state) = foreground.as_mut() {
                state.has_window = false;
            }
        }
        MainEvent::Destroy => {
            log_info("Rusty XR GLES Android activity destroy requested");
            if let Some(state) = foreground.as_mut() {
                state.destroyed = true;
            }
            if let Some(running) = running {
                *running = false;
            }
        }
        MainEvent::Pause => {
            log_info("Rusty XR GLES Android activity paused");
            if let Some(state) = foreground.as_mut() {
                state.resumed = false;
            }
        }
        MainEvent::Resume { .. } => {
            log_info("Rusty XR GLES Android activity resumed");
            if let Some(state) = foreground.as_mut() {
                state.resumed = true;
            }
        }
        MainEvent::GainedFocus => {
            log_info("Rusty XR GLES Android activity gained focus");
            if let Some(state) = foreground.as_mut() {
                state.focused = true;
            }
        }
        MainEvent::LostFocus => {
            log_info("Rusty XR GLES Android activity lost focus");
            if let Some(state) = foreground.as_mut() {
                state.focused = false;
            }
        }
        _ => {}
    }
}

fn drain_input_events(app: &android_activity::AndroidApp) {
    match app.input_events_iter() {
        Ok(mut events) => loop {
            if !events.next(|_| InputStatus::Handled) {
                break;
            }
        },
        Err(error) => {
            log_error(format!("Rusty XR GLES Android input drain failed: {error}"));
        }
    }
}

pub(super) fn keep_activity_alive_after_error(app: android_activity::AndroidApp) {
    log_info("Rusty XR GLES keeping activity alive after setup failure");
    let mut running = true;
    while running {
        app.poll_events(Some(Duration::from_millis(250)), |event| {
            if let PollEvent::Main(MainEvent::Destroy) = event {
                running = false;
            }
        });
    }
    log_info("Rusty XR GLES post-error keepalive exited");
}
