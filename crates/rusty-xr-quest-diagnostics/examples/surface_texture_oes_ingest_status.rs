use rusty_xr_quest_diagnostics::{
    FrameRateSummary, SurfaceTextureOesEyeStatus, SurfaceTextureOesIngestState,
    SurfaceTextureOesIngestStatus,
};

fn main() {
    let mut left = SurfaceTextureOesEyeStatus::for_stream(0, "synthetic-h264:left", "left")
        .mark_surface_ready()
        .mark_decoder_started();
    left.source_width = Some(1280);
    left.source_height = Some(720);
    left.record_update(120, 9001, 33_333, 44_444_000, "m44:identity");

    let mut right = SurfaceTextureOesEyeStatus::for_stream(1, "synthetic-h264:right", "right")
        .mark_surface_ready()
        .mark_decoder_started();
    right.source_width = Some(1280);
    right.source_height = Some(720);
    right.record_update(120, 9002, 33_334, 44_445_000, "m44:identity");

    let mut status = SurfaceTextureOesIngestStatus::new()
        .with_eye(left)
        .with_eye(right);
    status.state = SurfaceTextureOesIngestState::TextureUpdated;
    status.session_id = Some(String::from("public-synthetic-session"));
    status.codec_mime = Some(String::from("video/avc"));
    status.texture_update_rate = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0]);
    status.notes.push(String::from(
        "example status only; no native handles included",
    ));

    #[cfg(feature = "serde")]
    {
        println!(
            "{}",
            serde_json::to_string_pretty(&status).expect("serialize OES ingest status")
        );
    }

    #[cfg(not(feature = "serde"))]
    {
        println!(
            "SurfaceTexture/OES ingest ready: {}",
            status.is_iteration4_ready()
        );
    }
}
