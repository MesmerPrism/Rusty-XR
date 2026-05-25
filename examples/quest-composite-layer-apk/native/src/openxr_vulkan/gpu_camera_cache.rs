use ash::vk;

use super::gpu_camera_resources::{GpuCameraImport, GpuCameraImportKey, GpuCameraStereoDescriptor};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GpuCameraImportCacheStats {
    pub(super) import_count: usize,
    pub(super) stereo_descriptor_count: usize,
    pub(super) import_success_count: u64,
    pub(super) import_failure_count: u64,
    pub(super) import_cache_hit_count: u64,
    pub(super) import_cache_miss_count: u64,
    pub(super) import_cache_evict_count: u64,
}

#[derive(Default)]
pub(super) struct GpuCameraImportCache {
    imports: Vec<GpuCameraImport>,
    stereo_descriptors: Vec<GpuCameraStereoDescriptor>,
    import_success_count: u64,
    import_failure_count: u64,
    import_cache_hit_count: u64,
    import_cache_miss_count: u64,
    import_cache_evict_count: u64,
}

impl GpuCameraImportCache {
    pub(super) fn stats(&self) -> GpuCameraImportCacheStats {
        GpuCameraImportCacheStats {
            import_count: self.imports.len(),
            stereo_descriptor_count: self.stereo_descriptors.len(),
            import_success_count: self.import_success_count,
            import_failure_count: self.import_failure_count,
            import_cache_hit_count: self.import_cache_hit_count,
            import_cache_miss_count: self.import_cache_miss_count,
            import_cache_evict_count: self.import_cache_evict_count,
        }
    }

    pub(super) fn import_count(&self) -> usize {
        self.imports.len()
    }

    pub(super) fn stereo_descriptor_count(&self) -> usize {
        self.stereo_descriptors.len()
    }

    pub(super) fn record_import_success(&mut self) {
        self.import_success_count = self.import_success_count.saturating_add(1);
    }

    pub(super) fn record_import_failure(&mut self) {
        self.import_failure_count = self.import_failure_count.saturating_add(1);
    }

    pub(super) fn record_import_hit(&mut self) {
        self.import_cache_hit_count = self.import_cache_hit_count.saturating_add(1);
    }

    pub(super) fn record_import_miss(&mut self) {
        self.import_cache_miss_count = self.import_cache_miss_count.saturating_add(1);
    }

    pub(super) fn import(&self, index: usize) -> Option<&GpuCameraImport> {
        self.imports.get(index)
    }

    pub(super) fn stereo_descriptor(&self, index: usize) -> Option<&GpuCameraStereoDescriptor> {
        self.stereo_descriptors.get(index)
    }

    pub(super) fn import_index(&self, key: GpuCameraImportKey) -> Option<usize> {
        self.imports.iter().position(|import| import.key == key)
    }

    pub(super) fn stereo_descriptor_index(
        &self,
        left_key: GpuCameraImportKey,
        right_key: GpuCameraImportKey,
    ) -> Option<usize> {
        self.stereo_descriptors.iter().position(|descriptor| {
            descriptor.left_key == left_key && descriptor.right_key == right_key
        })
    }

    pub(super) fn import_image(&self, index: usize) -> Option<vk::Image> {
        self.imports.get(index).map(|import| import.image)
    }

    pub(super) fn import_image_needing_transition(&self, index: usize) -> Option<vk::Image> {
        self.imports
            .get(index)
            .filter(|import| import.needs_layout_transition)
            .map(|import| import.image)
    }

    pub(super) fn mark_import_layout_transitioned(&mut self, index: usize) {
        if let Some(import) = self.imports.get_mut(index) {
            import.needs_layout_transition = false;
        }
    }

    pub(super) fn import_image_view_for_key(
        &self,
        key: GpuCameraImportKey,
    ) -> Option<vk::ImageView> {
        self.imports
            .iter()
            .find(|import| import.key == key)
            .map(|import| import.image_view)
    }

    pub(super) fn push_import(&mut self, import: GpuCameraImport) -> usize {
        self.imports.push(import);
        self.imports.len() - 1
    }

    pub(super) fn push_stereo_descriptor(
        &mut self,
        descriptor: GpuCameraStereoDescriptor,
    ) -> usize {
        self.stereo_descriptors.push(descriptor);
        self.stereo_descriptors.len() - 1
    }

    pub(super) unsafe fn evict_oldest_import(&mut self, device: &ash::Device) {
        let old = self.imports.remove(0);
        self.destroy_stereo_descriptors_for_key(device, old.key);
        old.destroy(device);
        self.import_cache_evict_count = self.import_cache_evict_count.saturating_add(1);
    }

    pub(super) unsafe fn evict_oldest_stereo_descriptor(&mut self, device: &ash::Device) {
        let old = self.stereo_descriptors.remove(0);
        old.destroy(device);
    }

    pub(super) unsafe fn destroy_imports(&mut self, device: &ash::Device) {
        self.destroy_stereo_descriptors(device);
        for import in self.imports.drain(..) {
            import.destroy(device);
        }
    }

    pub(super) unsafe fn destroy_stereo_descriptors_for_key(
        &mut self,
        device: &ash::Device,
        key: GpuCameraImportKey,
    ) {
        let mut index = 0;
        while index < self.stereo_descriptors.len() {
            if self.stereo_descriptors[index].left_key == key
                || self.stereo_descriptors[index].right_key == key
            {
                let old = self.stereo_descriptors.remove(index);
                old.destroy(device);
            } else {
                index += 1;
            }
        }
    }

    pub(super) unsafe fn destroy_stereo_descriptors(&mut self, device: &ash::Device) {
        for descriptor in self.stereo_descriptors.drain(..) {
            descriptor.destroy(device);
        }
    }
}
