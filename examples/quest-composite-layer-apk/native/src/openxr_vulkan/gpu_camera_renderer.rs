use ash::vk;
use openxr as xr;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::{
    gpu_camera_cache::{GpuCameraImportCache, GpuCameraImportCacheStats},
    gpu_camera_draw::{record_camera_draw, record_stereo_camera_draw},
    gpu_camera_prepare::{prepare_camera_frame, prepare_stereo_camera_frame},
    gpu_camera_resource_state::GpuCameraResourceState,
    projection_geometry::ProjectedStereoHomographies,
};

pub(super) struct GpuCameraRenderer {
    ahb: Option<ash::android::external_memory_android_hardware_buffer::Device>,
    resources: GpuCameraResourceState,
    cache: GpuCameraImportCache,
    pub(super) last_failure: Option<String>,
}

impl GpuCameraRenderer {
    pub(super) unsafe fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        projection_uniform_alignment: vk::DeviceSize,
        render_pass: vk::RenderPass,
        import_supported: bool,
    ) -> Self {
        let ahb = import_supported.then(|| {
            ash::android::external_memory_android_hardware_buffer::Device::new(instance, device)
        });
        Self {
            ahb,
            resources: GpuCameraResourceState::new(
                memory_properties,
                projection_uniform_alignment,
                render_pass,
            ),
            cache: GpuCameraImportCache::default(),
            last_failure: None,
        }
    }

    pub(super) fn cache_stats(&self) -> GpuCameraImportCacheStats {
        self.cache.stats()
    }

    pub(super) unsafe fn prepare_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &HeadsetCameraGpuFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        let Some(ahb) = self.ahb.as_ref() else {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        };

        match prepare_camera_frame(
            ahb,
            &mut self.resources,
            &mut self.cache,
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.cache.record_import_success();
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.cache.record_import_failure();
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    pub(super) unsafe fn prepare_stereo_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &StereoGpuCameraFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        let Some(ahb) = self.ahb.as_ref() else {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        };

        match prepare_stereo_camera_frame(
            ahb,
            &mut self.resources,
            &mut self.cache,
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.cache.record_import_success();
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.cache.record_import_failure();
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    pub(super) unsafe fn record_draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        import_index: usize,
        frame: &HeadsetCameraGpuFrame,
        config: &crate::RuntimeConfig,
    ) {
        let Some(resources) = self.resources.resources() else {
            return;
        };
        let Some(import) = self.cache.import(import_index) else {
            return;
        };

        record_camera_draw(device, cmd, resolution, resources, import, frame, config);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) unsafe fn record_draw_stereo(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        descriptor_index: usize,
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        views: &[xr::View],
        frame_count: u64,
        applied_projection_homographies: Option<ProjectedStereoHomographies>,
    ) {
        let Some(resources) = self.resources.resources() else {
            return;
        };
        let Some(descriptor) = self.cache.stereo_descriptor(descriptor_index) else {
            return;
        };

        record_stereo_camera_draw(
            device,
            cmd,
            resolution,
            resources,
            descriptor,
            frame,
            config,
            views,
            frame_count,
            applied_projection_homographies,
        );
    }

    pub(super) unsafe fn destroy(&mut self, device: &ash::Device) {
        self.cache.destroy_imports(device);
        self.resources.destroy(device);
    }
}
