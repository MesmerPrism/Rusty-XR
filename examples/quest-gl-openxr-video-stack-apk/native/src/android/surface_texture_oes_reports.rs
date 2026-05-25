use rusty_xr_quest_diagnostics::{SurfaceTextureOesIngestState, SurfaceTextureOesIngestStatus};

use super::{
    log_info,
    source_metadata::OesProjectionMetadata,
    source_metadata_labels::stream_projection_metadata_log_message,
    surface_texture_oes_callbacks::{
        latest_decode_report_after, projection_metadata_report_snapshot, report_view_index,
    },
    VIEW_COUNT,
};

pub(super) struct SurfaceTextureOesReportState {
    last_report_sequence: u64,
    projection_metadata: [Option<OesProjectionMetadata>; VIEW_COUNT],
}

impl SurfaceTextureOesReportState {
    pub(super) fn new() -> Self {
        Self {
            last_report_sequence: 0,
            projection_metadata: std::array::from_fn(|_| None),
        }
    }

    pub(super) fn projection_metadata_pair(
        &self,
    ) -> Option<(&OesProjectionMetadata, &OesProjectionMetadata)> {
        Some((
            self.projection_metadata[0].as_ref()?,
            self.projection_metadata[1].as_ref()?,
        ))
    }

    pub(super) fn apply_latest_decode_report(
        &mut self,
        status: &mut SurfaceTextureOesIngestStatus,
    ) {
        let Some(report_json) = latest_decode_report_after(&mut self.last_report_sequence) else {
            return;
        };
        let Ok(report) = serde_json::from_str::<serde_json::Value>(&report_json) else {
            return;
        };
        if let Some(decoder_name) = report.get("decoder_name").and_then(|value| value.as_str()) {
            if !decoder_name.is_empty() {
                status.codec_name = Some(decoder_name.to_string());
            }
        }
        let event_name = report
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if event_name == "frame_available"
            && status.state == SurfaceTextureOesIngestState::DecoderStarted
        {
            status.state = SurfaceTextureOesIngestState::FrameAvailable;
        }
        let Some(view_index) = report_view_index(&report) else {
            return;
        };
        self.apply_projection_metadata_report(view_index, &report);
        self.apply_cached_projection_metadata_reports();
        let Some(eye) = status.eyes.get_mut(view_index) else {
            return;
        };
        if let Some(width) = report
            .get("width")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
        {
            eye.source_width = Some(width);
        }
        if let Some(height) = report
            .get("height")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
        {
            eye.source_height = Some(height);
        }
        if let Some(error) = report.get("error").and_then(|value| value.as_str()) {
            eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
            eye.latest_decoder_error = Some(error.to_string());
        } else if let Some(error) = report.get("last_error").and_then(|value| value.as_str()) {
            eye.latest_decoder_error = Some(error.to_string());
        }
    }

    fn apply_cached_projection_metadata_reports(&mut self) {
        for report_json in projection_metadata_report_snapshot().into_iter().flatten() {
            let Ok(report) = serde_json::from_str::<serde_json::Value>(&report_json) else {
                continue;
            };
            let Some(view_index) = report_view_index(&report) else {
                continue;
            };
            self.apply_projection_metadata_report(view_index, &report);
        }
    }

    fn apply_projection_metadata_report(&mut self, view_index: usize, report: &serde_json::Value) {
        if self
            .projection_metadata
            .get(view_index)
            .and_then(|value| value.as_ref())
            .is_some()
        {
            return;
        }
        let Some(metadata) = report
            .get("header_projection_metadata")
            .and_then(|metadata| OesProjectionMetadata::parse(metadata).ok())
        else {
            return;
        };
        log_info(stream_projection_metadata_log_message(
            view_index, &metadata,
        ));
        if let Some(slot) = self.projection_metadata.get_mut(view_index) {
            *slot = Some(metadata);
        }
    }
}
