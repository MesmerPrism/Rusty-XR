use std::time::Instant;

#[derive(Clone, Copy, Default)]
pub(super) struct CameraRenderCadenceFrame {
    pub(super) render_frame_count: u64,
    pub(super) distinct_frame_count: u64,
    pub(super) repeated_render_frame_count: u64,
    pub(super) renders_per_camera_frame_avg: f64,
    pub(super) max_consecutive_render_frames_per_camera_frame: u64,
    pub(super) consumed_frame_hz: f64,
    pub(super) projection_render_hz: f64,
}

#[derive(Default)]
pub(super) struct CameraRenderCadenceStats {
    started: Option<Instant>,
    render_frame_count: u64,
    distinct_frame_count: u64,
    repeated_render_frame_count: u64,
    last_camera_frame_index: Option<u64>,
    current_consecutive_render_frames: u64,
    max_consecutive_render_frames_per_camera_frame: u64,
}

impl CameraRenderCadenceStats {
    pub(super) fn record(&mut self, camera_frame_index: u64) -> CameraRenderCadenceFrame {
        let started = *self.started.get_or_insert_with(Instant::now);
        self.render_frame_count = self.render_frame_count.saturating_add(1);

        if self.last_camera_frame_index == Some(camera_frame_index) {
            self.repeated_render_frame_count = self.repeated_render_frame_count.saturating_add(1);
            self.current_consecutive_render_frames =
                self.current_consecutive_render_frames.saturating_add(1);
        } else {
            self.distinct_frame_count = self.distinct_frame_count.saturating_add(1);
            self.last_camera_frame_index = Some(camera_frame_index);
            self.current_consecutive_render_frames = 1;
        }

        self.max_consecutive_render_frames_per_camera_frame = self
            .max_consecutive_render_frames_per_camera_frame
            .max(self.current_consecutive_render_frames);

        let elapsed_seconds = started.elapsed().as_secs_f64();
        let hz_divisor = if elapsed_seconds > 0.001 {
            elapsed_seconds
        } else {
            f64::INFINITY
        };
        let renders_per_camera_frame_avg = if self.distinct_frame_count > 0 {
            self.render_frame_count as f64 / self.distinct_frame_count as f64
        } else {
            0.0
        };

        CameraRenderCadenceFrame {
            render_frame_count: self.render_frame_count,
            distinct_frame_count: self.distinct_frame_count,
            repeated_render_frame_count: self.repeated_render_frame_count,
            renders_per_camera_frame_avg,
            max_consecutive_render_frames_per_camera_frame: self
                .max_consecutive_render_frames_per_camera_frame,
            consumed_frame_hz: self.distinct_frame_count as f64 / hz_divisor,
            projection_render_hz: self.render_frame_count as f64 / hz_divisor,
        }
    }
}
