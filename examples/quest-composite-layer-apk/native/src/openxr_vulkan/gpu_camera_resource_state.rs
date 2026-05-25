use ash::vk;

use super::{
    gpu_camera_import::CameraHardwareBufferImportPlan,
    gpu_camera_pipeline::create_gpu_camera_pipeline_resources,
    gpu_camera_resources::{GpuCameraFormatKey, GpuCameraPipelineResources},
};

pub(super) struct GpuCameraResourceState {
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    projection_uniform_alignment: vk::DeviceSize,
    render_pass: vk::RenderPass,
    resources: Option<GpuCameraPipelineResources>,
}

impl GpuCameraResourceState {
    pub(super) fn new(
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        projection_uniform_alignment: vk::DeviceSize,
        render_pass: vk::RenderPass,
    ) -> Self {
        Self {
            memory_properties,
            projection_uniform_alignment,
            render_pass,
            resources: None,
        }
    }

    pub(super) fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.memory_properties
    }

    pub(super) fn resources(&self) -> Option<&GpuCameraPipelineResources> {
        self.resources.as_ref()
    }

    pub(super) fn needs_format_rebuild(&self, format_key: GpuCameraFormatKey) -> bool {
        self.resources
            .as_ref()
            .map(|resources| resources.format_key != format_key)
            .unwrap_or(true)
    }

    pub(super) unsafe fn rebuild_for_import_plan(
        &mut self,
        device: &ash::Device,
        import_plan: &CameraHardwareBufferImportPlan,
    ) -> Result<(), String> {
        self.destroy(device);
        self.resources = Some(create_gpu_camera_pipeline_resources(
            device,
            &self.memory_properties,
            self.projection_uniform_alignment,
            self.render_pass,
            import_plan.format_key,
            &import_plan.format_props,
        )?);
        Ok(())
    }

    pub(super) unsafe fn destroy(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
    }
}
