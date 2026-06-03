#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadCameraTexturePath {
    DirectHardwareBufferExternal,
    DirectHardwareBufferYuvPlane,
    DirectCpuYuvPlane,
    BrokerH264CpuYuv,
    BrokerH264HardwareBuffer,
    BrokerH264SurfaceTexture,
}

impl MakepadCameraTexturePath {
    pub(crate) const fn from_video_update(broker_h264_enabled: bool, yuv_enabled: bool) -> Self {
        match (broker_h264_enabled, yuv_enabled) {
            (true, true) => Self::BrokerH264CpuYuv,
            (true, false) => Self::BrokerH264SurfaceTexture,
            (false, true) => Self::DirectCpuYuvPlane,
            (false, false) => Self::DirectHardwareBufferExternal,
        }
    }

    pub(crate) const fn from_direct_video_update(
        direct_hardware_buffer_requested: bool,
        yuv_enabled: bool,
    ) -> Self {
        match (direct_hardware_buffer_requested, yuv_enabled) {
            (true, true) => Self::DirectHardwareBufferYuvPlane,
            (true, false) => Self::DirectHardwareBufferExternal,
            (false, true) => Self::DirectCpuYuvPlane,
            (false, false) => Self::DirectCpuYuvPlane,
        }
    }

    pub(crate) const fn direct_default() -> Self {
        Self::DirectCpuYuvPlane
    }

    pub(crate) const fn from_direct_hardware_buffer_external_enabled(enabled: bool) -> Self {
        if enabled {
            Self::DirectHardwareBufferExternal
        } else {
            Self::direct_default()
        }
    }

    pub(crate) const fn yuv_sampling_enabled(self) -> bool {
        match self {
            Self::DirectHardwareBufferYuvPlane
            | Self::DirectCpuYuvPlane
            | Self::BrokerH264CpuYuv => true,
            Self::DirectHardwareBufferExternal
            | Self::BrokerH264HardwareBuffer
            | Self::BrokerH264SurfaceTexture => false,
        }
    }

    pub(crate) const fn yuv_mode(self) -> f32 {
        if self.yuv_sampling_enabled() {
            1.0
        } else {
            0.0
        }
    }

    pub(crate) const fn makepad_vulkan_import(self) -> bool {
        match self {
            Self::DirectHardwareBufferExternal
            | Self::DirectHardwareBufferYuvPlane
            | Self::BrokerH264HardwareBuffer => true,
            Self::DirectCpuYuvPlane | Self::BrokerH264CpuYuv | Self::BrokerH264SurfaceTexture => {
                false
            }
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal => "direct-camera-hardware-buffer-external",
            Self::DirectHardwareBufferYuvPlane => "direct-camera-hardware-buffer-yuv-plane",
            Self::DirectCpuYuvPlane => "direct-camera-cpu-yuv-plane",
            Self::BrokerH264CpuYuv => "broker-h264-mediacodec-cpu-yuv",
            Self::BrokerH264HardwareBuffer => "broker-h264-mediacodec-hardware-buffer",
            Self::BrokerH264SurfaceTexture => "broker-h264-surface-texture",
        }
    }

    pub(crate) const fn import_plan(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal => "paired-camera-hardware-buffer-external",
            Self::DirectHardwareBufferYuvPlane => "paired-camera-hardware-buffer-yuv-plane",
            Self::DirectCpuYuvPlane => "paired-camera-cpu-yuv-fallback",
            Self::BrokerH264CpuYuv => "broker-h264-stereo-mediacodec-yuv-texture",
            Self::BrokerH264HardwareBuffer => "broker-h264-stereo-mediacodec-hardware-buffer",
            Self::BrokerH264SurfaceTexture => "broker-h264-surface-texture",
        }
    }

    pub(crate) const fn texture_import_path(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal => "makepad-camera-hardware-buffer-vulkan-import",
            Self::DirectHardwareBufferYuvPlane => "makepad-camera-hardware-buffer-vulkan-yuv-plane",
            Self::DirectCpuYuvPlane => "makepad-camera-cpu-yuv-plane",
            Self::BrokerH264CpuYuv => "broker-h264-mediacodec-cpu-yuv",
            Self::BrokerH264HardwareBuffer => {
                "broker-h264-mediacodec-hardware-buffer-vulkan-import"
            }
            Self::BrokerH264SurfaceTexture => "broker-h264-surface-texture",
        }
    }

    pub(crate) const fn cpu_upload_path(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal
            | Self::BrokerH264HardwareBuffer
            | Self::BrokerH264SurfaceTexture => "none",
            Self::DirectHardwareBufferYuvPlane => "none",
            Self::DirectCpuYuvPlane => "makepad-camera-cpu-yuv-plane",
            Self::BrokerH264CpuYuv => "broker-h264-mediacodec-cpu-yuv",
        }
    }

    pub(crate) const fn eye_selection(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal => "per-eye-direct-camera-external-rgb",
            Self::BrokerH264HardwareBuffer => "per-eye-broker-h264-hardware-buffer-external-rgb",
            Self::BrokerH264SurfaceTexture => "per-eye-broker-h264-surface-texture-external-rgb",
            Self::DirectHardwareBufferYuvPlane => "per-eye-direct-camera-hardware-buffer-yuv",
            Self::DirectCpuYuvPlane | Self::BrokerH264CpuYuv => {
                "per-eye-direct-camera-yuv-color-limited601-noswap-border"
            }
        }
    }

    pub(crate) const fn color_conversion(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal
            | Self::BrokerH264HardwareBuffer
            | Self::BrokerH264SurfaceTexture => "external-rgb",
            Self::DirectHardwareBufferYuvPlane => "per-eye-hardware-buffer-yuv-limited-bt601",
            Self::DirectCpuYuvPlane | Self::BrokerH264CpuYuv => "per-eye-yuv-noswap-limited-bt601",
        }
    }

    pub(crate) const fn color_reference(self) -> &'static str {
        match self {
            Self::DirectHardwareBufferExternal => "android-hardware-buffer-external-rgb",
            Self::DirectHardwareBufferYuvPlane => "android-hardware-buffer-yuv-plane",
            Self::DirectCpuYuvPlane | Self::BrokerH264CpuYuv => "android-yuv420-888-plane-order",
            Self::BrokerH264HardwareBuffer => "broker-h264-decoded-android-hardware-buffer",
            Self::BrokerH264SurfaceTexture => "broker-h264-external-texture",
        }
    }

    pub(crate) const fn visual_color_status(self) -> &'static str {
        match self {
            Self::DirectCpuYuvPlane | Self::BrokerH264CpuYuv => "accepted-cpu-yuv-reference",
            Self::DirectHardwareBufferExternal => {
                "experimental-hardware-buffer-external-combined-immutable-default-sampler-ycbcr-candidate"
            }
            Self::BrokerH264HardwareBuffer => {
                "experimental-broker-h264-hardware-buffer-external-ycbcr-candidate"
            }
            Self::DirectHardwareBufferYuvPlane => {
                "experimental-hardware-buffer-yuv-plane-color-not-accepted"
            }
            Self::BrokerH264SurfaceTexture => "experimental-surface-texture-color-not-accepted",
        }
    }

    pub(crate) fn marker_fields(self) -> String {
        format!(
            "cameraTexturePath={} makepadVulkanImport={} textureImportPath={} cpuUploadPath={} visualColorStatus={}",
            self.stable_id(),
            self.makepad_vulkan_import(),
            self.texture_import_path(),
            self.cpu_upload_path(),
            self.visual_color_status(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MakepadCameraTexturePath;

    #[test]
    fn direct_hardware_buffer_path_is_not_cpu_yuv() {
        let path = MakepadCameraTexturePath::from_direct_hardware_buffer_external_enabled(true);

        assert_eq!(path, MakepadCameraTexturePath::DirectHardwareBufferExternal);
        assert!(!path.yuv_sampling_enabled());
        assert!(path.makepad_vulkan_import());
        assert_eq!(path.cpu_upload_path(), "none");
        assert_eq!(
            path.visual_color_status(),
            "experimental-hardware-buffer-external-combined-immutable-default-sampler-ycbcr-candidate"
        );
        assert!(path
            .marker_fields()
            .contains("textureImportPath=makepad-camera-hardware-buffer-vulkan-import"));
    }

    #[test]
    fn direct_hardware_buffer_yuv_update_keeps_gpu_import_without_external_rgb() {
        let path = MakepadCameraTexturePath::from_direct_video_update(true, true);

        assert_eq!(path, MakepadCameraTexturePath::DirectHardwareBufferYuvPlane);
        assert!(path.yuv_sampling_enabled());
        assert!(path.makepad_vulkan_import());
        assert_eq!(path.cpu_upload_path(), "none");
        assert_eq!(
            path.visual_color_status(),
            "experimental-hardware-buffer-yuv-plane-color-not-accepted"
        );
        assert!(path
            .marker_fields()
            .contains("textureImportPath=makepad-camera-hardware-buffer-vulkan-yuv-plane"));
    }

    #[test]
    fn direct_default_keeps_visual_cpu_yuv_path() {
        let path = MakepadCameraTexturePath::direct_default();

        assert_eq!(path, MakepadCameraTexturePath::DirectCpuYuvPlane);
        assert!(path.yuv_sampling_enabled());
        assert!(!path.makepad_vulkan_import());
        assert_eq!(path.visual_color_status(), "accepted-cpu-yuv-reference");
    }

    #[test]
    fn yuv_metadata_enabled_selects_cpu_yuv_path() {
        let path = MakepadCameraTexturePath::from_video_update(false, true);

        assert_eq!(path, MakepadCameraTexturePath::DirectCpuYuvPlane);
        assert!(path.yuv_sampling_enabled());
        assert!(!path.makepad_vulkan_import());
        assert_eq!(path.cpu_upload_path(), "makepad-camera-cpu-yuv-plane");
        assert_eq!(path.visual_color_status(), "accepted-cpu-yuv-reference");
    }

    #[test]
    fn broker_h264_hardware_buffer_path_uses_vulkan_import() {
        let path = MakepadCameraTexturePath::BrokerH264HardwareBuffer;

        assert!(!path.yuv_sampling_enabled());
        assert!(path.makepad_vulkan_import());
        assert_eq!(path.cpu_upload_path(), "none");
        assert!(path
            .marker_fields()
            .contains("cameraTexturePath=broker-h264-mediacodec-hardware-buffer"));
        assert!(path
            .marker_fields()
            .contains("textureImportPath=broker-h264-mediacodec-hardware-buffer-vulkan-import"));
    }
}
