use openxr as xr;

pub(super) fn projection_openxr_contract_fields(
    openxr_reference_space: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
) -> String {
    let Some(left) = views.first() else {
        return format!(
            "referenceSpace=app-reference-space openxrReferenceSpace={} displayTimeSource=not-logged predictedDisplayTimeSource=not-logged predictedDisplayTimeNs=not-logged viewPoseFovSource=not-logged",
            marker_token(Some(openxr_reference_space), "unknown")
        );
    };
    let right = views.get(1).unwrap_or(left);
    format!(
        "referenceSpace=app-reference-space openxrReferenceSpace={} displayTimeSource=predicted-display-time predictedDisplayTimeSource=predicted-display-time predictedDisplayTimeNs={} viewPoseFovSource=xrLocateViews leftRenderFovTangents={} rightRenderFovTangents={} leftRenderPosition={} rightRenderPosition={} leftRenderOrientation={} rightRenderOrientation={}",
        marker_token(Some(openxr_reference_space), "unknown"),
        predicted_display_time.as_nanos(),
        format_vec4(fov_tangents(left.fov)),
        format_vec4(fov_tangents(right.fov)),
        format_vec4(pose_position(left.pose)),
        format_vec4(pose_position(right.pose)),
        format_vec4(pose_orientation(left.pose)),
        format_vec4(pose_orientation(right.pose))
    )
}

pub(super) fn projection_openxr_contract_log_message(
    frame_index: u64,
    openxr_frame_count: u64,
    aligned_projection: bool,
    openxr_reference_space: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
) -> String {
    format!(
        "Rusty XR OpenXR projection contract frame={} openXrFrameCount={} activeTier=gpu-projected alignedProjection={} {}",
        frame_index,
        openxr_frame_count,
        aligned_projection,
        projection_openxr_contract_fields(openxr_reference_space, predicted_display_time, views)
    )
}

pub(super) fn display_eye_uv_fiducial_marker_fields(config: &crate::RuntimeConfig) -> &'static str {
    use crate::camera_color_pipeline::CameraProjectionEffectMode;
    match config.camera_projection_effect_mode {
        CameraProjectionEffectMode::DisplayEyeUvFiducial => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.display_eye_uv_fiducial.v1 displayEyeUvFiducialCoordinateSpace=display-eye-screen-uv displayEyeUvFiducialUvBasis=projection_screen_uv_base displayEyeUvFiducialShaderFormula=displayEyeUv=(surfaceUv-0.5)*projectionAreaScaleUv+0.5 displayEyeUvFiducialMarkersUv=cyan_upper_left@0.250000,0.250000;red_left_mid@0.250000,0.500000;yellow_top_mid@0.500000,0.250000;green_center@0.500000,0.500000;magenta_bottom_mid@0.500000,0.750000;blue_right_mid@0.750000,0.500000",
        CameraProjectionEffectMode::ProjectionContentUvFiducial => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.display_eye_uv_fiducial.v1 displayEyeUvFiducialCoordinateSpace=projection-content-uv displayEyeUvFiducialUvBasis=full_frame_content_uv displayEyeUvFiducialShaderFormula=contentUv=(projectionScreenUv-(0.5-radiusUv))/(2*radiusUv);projectionScreenUv=(surfaceUv-0.5)*projectionAreaScaleUv+0.5-offsetUv displayEyeUvFiducialMarkersUv=cyan_upper_left@0.250000,0.250000;red_left_mid@0.250000,0.500000;yellow_top_mid@0.500000,0.250000;green_center@0.500000,0.500000;magenta_bottom_mid@0.500000,0.750000;blue_right_mid@0.750000,0.500000",
        CameraProjectionEffectMode::SourceSamplingWitness => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.source_sampling_witness.v1 displayEyeUvFiducialCoordinateSpace=source-sampling-witness displayEyeUvFiducialUvBasis=actual-source-image+full_frame_content_uv+hardware-buffer-sampler-uv displayEyeUvFiducialShaderFormula=contentUv=(projectionScreenUv-(0.5-radiusUv))/(2*radiusUv);sourceSamplerUv=cameraTextureTransform(sourceVisibleUvRect(contentUv)) displayEyeUvFiducialMarkersUv=content_grid_yellow_white@0.125,0.250,0.500;source_sampler_grid_cyan_magenta@0.125,0.250,0.500",
        _ => "displayEyeUvFiducialActive=false",
    }
}

pub(super) fn display_eye_uv_fiducial_contract_log_message(
    frame_index: u64,
    openxr_frame_count: u64,
    marker_fields: &str,
) -> String {
    format!(
        "Rusty XR display-eye UV fiducial contract frame={} openXrFrameCount={} {}",
        frame_index, openxr_frame_count, marker_fields
    )
}

fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}

fn format_vec4(values: [f32; 4]) -> String {
    format!(
        "[{:.6},{:.6},{:.6},{:.6}]",
        values[0], values[1], values[2], values[3]
    )
}

fn fov_tangents(fov: xr::sys::Fovf) -> [f32; 4] {
    [
        fov.angle_left.tan(),
        fov.angle_right.tan(),
        fov.angle_up.tan(),
        fov.angle_down.tan(),
    ]
}

fn pose_position(pose: xr::sys::Posef) -> [f32; 4] {
    [pose.position.x, pose.position.y, pose.position.z, 1.0]
}

fn pose_orientation(pose: xr::sys::Posef) -> [f32; 4] {
    [
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ]
}
