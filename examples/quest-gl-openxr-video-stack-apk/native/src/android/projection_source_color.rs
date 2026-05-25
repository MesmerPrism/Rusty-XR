use super::{
    gl_format_label,
    openxr_gles_config::{OesColorControls, OesSourceColorTransfer},
    GL_SRGB8_ALPHA8,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct OesSourceColorContract<'a> {
    pub(super) input_encoding: &'a str,
    pub(super) transform: &'a str,
    pub(super) transform_applied: bool,
    pub(super) output_encoding: &'a str,
    pub(super) swapchain_color_format: &'a str,
    pub(super) swapchain_color_encoding: &'a str,
}

pub(super) fn source_color_contract_fields(fields: OesSourceColorContract<'_>) -> String {
    format!(
        "sourceColorInputEncoding={} sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform={} sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=post_source_color_transfer swapchainColorFormat={} swapchainColorEncoding={}",
        fields.input_encoding,
        fields.transform,
        fields.transform_applied,
        fields.output_encoding,
        fields.swapchain_color_format,
        fields.swapchain_color_encoding
    )
}

pub(super) fn source_color_contract(
    camera_color_controls: OesColorControls,
    swapchain_color_format: u32,
) -> OesSourceColorContract<'static> {
    let transfer = camera_color_controls.source_transfer;
    OesSourceColorContract {
        input_encoding: transfer.input_encoding(),
        transform: transfer.stable_id(),
        transform_applied: transfer != OesSourceColorTransfer::Identity,
        output_encoding: transfer.output_encoding(),
        swapchain_color_format: gl_format_label(swapchain_color_format),
        swapchain_color_encoding: if swapchain_color_format == GL_SRGB8_ALPHA8 {
            "srgb"
        } else {
            "linear-or-runtime-default"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_color_contract_fields_keep_marker_shape() {
        assert_eq!(
            source_color_contract_fields(OesSourceColorContract {
                input_encoding: "external-oes-srgb-nonlinear-rgb",
                transform: "srgb-to-linear",
                transform_applied: true,
                output_encoding: "linear-rgb",
                swapchain_color_format: "GL_SRGB8_ALPHA8",
                swapchain_color_encoding: "srgb",
            }),
            "sourceColorInputEncoding=external-oes-srgb-nonlinear-rgb sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform=srgb-to-linear sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied=true sourceColorOutputEncoding=linear-rgb cameraColorControlStage=post_source_color_transfer swapchainColorFormat=GL_SRGB8_ALPHA8 swapchainColorEncoding=srgb"
        );
    }
}
