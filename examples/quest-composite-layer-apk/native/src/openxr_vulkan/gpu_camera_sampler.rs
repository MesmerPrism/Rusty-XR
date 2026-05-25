use ash::vk;

use super::gpu_camera_resources::GpuCameraFormatKey;

pub(super) unsafe fn create_camera_sampler_resources(
    device: &ash::Device,
    format_key: GpuCameraFormatKey,
    format_props: &vk::AndroidHardwareBufferFormatPropertiesANDROID<'_>,
) -> Result<(vk::SamplerYcbcrConversion, vk::Sampler), String> {
    let mut external_format =
        vk::ExternalFormatANDROID::default().external_format(format_key.external_format);
    let mut conversion_info = vk::SamplerYcbcrConversionCreateInfo::default()
        .format(format_key.format)
        .ycbcr_model(format_props.suggested_ycbcr_model)
        .ycbcr_range(format_props.suggested_ycbcr_range)
        .components(format_props.sampler_ycbcr_conversion_components)
        .x_chroma_offset(format_props.suggested_x_chroma_offset)
        .y_chroma_offset(format_props.suggested_y_chroma_offset)
        .chroma_filter(vk::Filter::LINEAR);
    if format_key.external_format != 0 {
        conversion_info = conversion_info.push_next(&mut external_format);
    }
    let sampler_ycbcr_conversion = device
        .create_sampler_ycbcr_conversion(&conversion_info, None)
        .map_err(|error| format!("create camera sampler YCbCr conversion: {error}"))?;

    let mut sampler_conversion_info =
        vk::SamplerYcbcrConversionInfo::default().conversion(sampler_ycbcr_conversion);
    let sampler = device
        .create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .push_next(&mut sampler_conversion_info),
            None,
        )
        .map_err(|error| format!("create camera sampler: {error}"))?;

    Ok((sampler_ycbcr_conversion, sampler))
}
