use rusty_xr_quest_diagnostics::{
    EglGlesContextStatus, FrameRateSummary, GlFramebufferCompleteness, OpenXrGlesFeasibilityState,
    OpenXrGlesFeasibilityStatus, OpenXrGlesGraphicsRequirements, OpenXrGlesSwapchainFormat,
    OpenXrGlesViewStatus,
};

fn main() {
    let mut status = OpenXrGlesFeasibilityStatus::new();
    status.state = OpenXrGlesFeasibilityState::Rendering;
    status.runtime_name = Some(String::from("example-runtime"));
    status.runtime_version = Some(String::from("1.0"));
    status.required_extensions[0].available = true;
    status.graphics_requirements = Some(OpenXrGlesGraphicsRequirements::new(
        "OpenGL ES 3.0",
        "OpenGL ES 3.2",
    ));
    status.context =
        Some(EglGlesContextStatus::current_gles("OpenGL ES 3.2").with_rgba_bits(8, 8, 8, 8));
    status
        .swapchain_formats
        .push(OpenXrGlesSwapchainFormat::color(0x8058, "GL_RGBA8").with_selected(true));
    status.views = vec![
        OpenXrGlesViewStatus {
            fbo_status: GlFramebufferCompleteness::Complete,
            acquired_image_index: Some(0),
            last_rendered_frame_index: Some(42),
            ..OpenXrGlesViewStatus::diagnostic_grid(0, 1440, 1584, "left-grid")
        },
        OpenXrGlesViewStatus {
            fbo_status: GlFramebufferCompleteness::Complete,
            acquired_image_index: Some(0),
            last_rendered_frame_index: Some(42),
            ..OpenXrGlesViewStatus::diagnostic_grid(1, 1440, 1584, "right-grid")
        },
    ];
    status.frame_rate = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0, 1.0 / 72.0]);

    println!("{status:#?}");
}
