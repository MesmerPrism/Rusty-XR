use rusty_xr_contracts::{
    Eye, InvalidProjectionFillPolicy, MatrixStepStatus, MatrixSyntheticVideoSource,
    ProjectionFootprintRowSpan, ProjectionFootprintSummary, ProjectionGuideDomain,
    ProjectionMatrixLaneKind, ProjectionMatrixLaneReport, ProjectionPerformanceMatrixPacket,
    ProjectionPerformanceScorecard, ProjectionStageKind, ProjectionStageTokenRow,
};

fn rows(offset_x: f32) -> [[f32; 3]; 3] {
    [[1.0, 0.0, offset_x], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn main() {
    let source = MatrixSyntheticVideoSource::broker_h264_diagnostic_grid_1280();
    let reference = ProjectionMatrixLaneReport::new(
        "reference",
        "reference hardware-buffer lane",
        ProjectionMatrixLaneKind::OpenXrVulkanHardwareBuffer,
    )
    .with_source_feed(MatrixStepStatus::Passed)
    .with_decoded_texture(MatrixStepStatus::Passed)
    .with_projection_stage(MatrixStepStatus::Passed)
    .with_projection_footprint(MatrixStepStatus::Passed)
    .with_public_or_raw_layer(MatrixStepStatus::Passed)
    .with_effect_or_guide_layer(MatrixStepStatus::NotApplicable)
    .with_performance_budget(MatrixStepStatus::Passed)
    .with_stage_token(
        ProjectionStageTokenRow::new("reference", Eye::Left, ProjectionStageKind::SurfaceToScreen)
            .with_rows(rows(0.0))
            .with_source("run-log"),
    )
    .with_stage_token(
        ProjectionStageTokenRow::new("reference", Eye::Left, ProjectionStageKind::ScreenToSurface)
            .with_rows(rows(0.0))
            .with_source("run-log"),
    )
    .with_stage_token(
        ProjectionStageTokenRow::new("reference", Eye::Left, ProjectionStageKind::SurfaceToCamera)
            .with_token("stc:7c18a2")
            .with_source("run-log"),
    )
    .with_stage_token(
        ProjectionStageTokenRow::new("reference", Eye::Left, ProjectionStageKind::ScreenToCamera)
            .with_rows(rows(0.012))
            .with_source("run-log"),
    )
    .with_footprint(
        ProjectionFootprintSummary::new("reference", "raw")
            .with_active_fraction(0.82)
            .with_bbox_fraction([0.09, 0.10, 0.91, 0.90])
            .with_row_span(ProjectionFootprintRowSpan::new(0.50, 0.80).with_span(0.10, 0.90))
            .with_mask_iou_against_reference(1.0)
            .with_invalid_fill_policy(InvalidProjectionFillPolicy::VisualContinuityFallback)
            .with_guide_domain(ProjectionGuideDomain::ScreenCamera)
            .with_explicit_valid_mask(true),
    )
    .with_performance(ProjectionPerformanceScorecard {
        source_packet_fps: Some(30.0),
        decoded_texture_update_fps: Some(30.0),
        openxr_fps: Some(72.0),
        gpu_percent: Some(68.0),
        pass_count: Some(1),
        intermediate_texture_bytes_per_frame: Some(0),
        app_fatal_count: Some(0),
        gpu_fault_count: Some(0),
        android_runtime_crash_count: Some(0),
        ..ProjectionPerformanceScorecard::default()
    });

    let gl_candidate = ProjectionMatrixLaneReport::new(
        "gl_oes",
        "OpenXR GLES SurfaceTexture/OES lane",
        ProjectionMatrixLaneKind::OpenXrOpenGlSurfaceTextureOes,
    )
    .with_source_feed(MatrixStepStatus::Passed)
    .with_decoded_texture(MatrixStepStatus::Passed)
    .with_projection_stage(MatrixStepStatus::Blocked)
    .with_projection_footprint(MatrixStepStatus::Blocked)
    .with_public_or_raw_layer(MatrixStepStatus::Blocked)
    .with_effect_or_guide_layer(MatrixStepStatus::Blocked)
    .with_performance_budget(MatrixStepStatus::Ambiguous)
    .with_performance(ProjectionPerformanceScorecard {
        source_packet_fps: Some(30.0),
        decoded_texture_update_fps: Some(30.0),
        surface_texture_update_count: Some(120),
        surface_texture_skipped_frame_count: Some(0),
        openxr_fps: Some(72.0),
        pass_count: Some(0),
        fbo_switch_count: Some(0),
        app_fatal_count: Some(0),
        gpu_fault_count: Some(0),
        android_runtime_crash_count: Some(0),
        ..ProjectionPerformanceScorecard::default()
    })
    .with_blocker("raw internal FBO layer and projection-stage rows are not emitted yet");

    let packet = ProjectionPerformanceMatrixPacket::new("matrix-synthetic-001", source)
        .with_lane(reference)
        .with_lane(gl_candidate)
        .with_note("Synthetic packet shape only; renderer artifacts remain adapter-owned.");

    assert!(packet.is_valid());
    println!(
        "{}",
        serde_json::to_string_pretty(&packet).expect("matrix packet should serialize")
    );
}
