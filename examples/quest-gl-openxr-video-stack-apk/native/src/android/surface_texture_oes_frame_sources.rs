use super::{log_info, surface_texture_oes_probe::SurfaceTextureOesProbe, VIEW_COUNT};

pub(super) struct OesEyeTextureSample {
    pub(super) texture: u32,
    pub(super) source_sequence: u64,
    pub(super) queued_pts_us: Option<i64>,
    pub(super) surface_timestamp_ns: Option<i64>,
    pub(super) transform_hash: Option<String>,
    pub(super) transform_matrix: [f32; 16],
    pub(super) update_tex_image_count: u64,
    pub(super) frame_age_at_submit_ms: Option<f32>,
}

pub(super) struct OesRenderFrameSources {
    samples: [Option<OesEyeTextureSample>; VIEW_COUNT],
}

impl OesRenderFrameSources {
    pub(super) fn from_probe(
        probe: Option<&SurfaceTextureOesProbe>,
        include_submit_age: bool,
    ) -> Self {
        let samples = std::array::from_fn(|view_index| {
            let probe = probe?;
            let mut sample = probe.updated_eye_texture(view_index)?;
            if include_submit_age {
                sample.frame_age_at_submit_ms = sample
                    .queued_pts_us
                    .and_then(|queued_pts_us| probe.frame_age_at_submit_ms(queued_pts_us));
            }
            Some(sample)
        });

        Self { samples }
    }

    pub(super) fn eye(&self, view_index: usize) -> Option<&OesEyeTextureSample> {
        self.samples.get(view_index)?.as_ref()
    }
}

pub(super) fn log_oes_submit_diagnostic(
    view_index: usize,
    frame_count: u64,
    source: &OesEyeTextureSample,
    render_path: &str,
) {
    let payload = serde_json::json!({
        "schema": "rusty.xr.quest.openxr_gles_oes_submit.v1",
        "view_index": view_index,
        "frame_count": frame_count,
        "source_sequence": source.source_sequence,
        "queued_pts_us": source.queued_pts_us,
        "surface_texture_timestamp_ns": source.surface_timestamp_ns,
        "transform_matrix_hash": source.transform_hash,
        "update_tex_image_count": source.update_tex_image_count,
        "frame_age_at_submit_ms": source.frame_age_at_submit_ms,
        "render_path": render_path,
    });
    log_info(format!(
        "Rusty XR OpenXR GLES OES submit diagnostic {payload}"
    ));
}
