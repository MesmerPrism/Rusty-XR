use jni::{
    objects::{GlobalRef, JValue},
    JavaVM,
};
use rusty_xr_quest_diagnostics::{SurfaceTextureOesEyeStatus, SurfaceTextureOesIngestStatus};

use super::{
    source_metadata::OesInputSourceKind,
    surface_texture_oes_gl::{create_external_oes_texture, delete_gl_texture},
    EglContext, VIEW_COUNT,
};

pub(super) const DEFAULT_OES_SURFACE_WIDTH: i32 = 1280;
pub(super) const DEFAULT_OES_SURFACE_HEIGHT: i32 = 1280;

pub(super) struct SurfaceTextureOesOutputResources {
    surface_textures: Vec<GlobalRef>,
    output_surfaces: Vec<GlobalRef>,
    decode_probe: Option<GlobalRef>,
    textures: Vec<u32>,
}

impl SurfaceTextureOesOutputResources {
    pub(super) fn create(
        java_vm: &JavaVM,
        egl: &EglContext,
        source_kind: OesInputSourceKind,
        status: &mut SurfaceTextureOesIngestStatus,
    ) -> Result<Self, String> {
        egl.make_current()?;
        let mut textures = Vec::with_capacity(VIEW_COUNT);
        let mut env = java_vm
            .attach_current_thread()
            .map_err(|error| format!("attach JNI thread for SurfaceTexture probe: {error}"))?;
        let mut surface_textures = Vec::with_capacity(VIEW_COUNT);
        let mut output_surfaces = Vec::with_capacity(VIEW_COUNT);

        for view_index in 0..VIEW_COUNT {
            let texture = create_external_oes_texture()?;
            let texture_name = i32::try_from(texture)
                .map_err(|_| format!("external OES texture id {texture} does not fit JNI int"))?;
            let surface_texture = env
                .new_object(
                    "android/graphics/SurfaceTexture",
                    "(I)V",
                    &[JValue::Int(texture_name)],
                )
                .map_err(|error| {
                    delete_gl_texture(texture);
                    format!("create Android SurfaceTexture for eye {view_index}: {error}")
                })?;
            env.call_method(
                &surface_texture,
                "setDefaultBufferSize",
                "(II)V",
                &[
                    JValue::Int(DEFAULT_OES_SURFACE_WIDTH),
                    JValue::Int(DEFAULT_OES_SURFACE_HEIGHT),
                ],
            )
            .map_err(|error| {
                delete_gl_texture(texture);
                format!("set SurfaceTexture default buffer size for eye {view_index}: {error}")
            })?;
            let output_surface = env
                .new_object(
                    "android/view/Surface",
                    "(Landroid/graphics/SurfaceTexture;)V",
                    &[JValue::Object(&surface_texture)],
                )
                .map_err(|error| {
                    delete_gl_texture(texture);
                    format!("create Android Surface for eye {view_index}: {error}")
                })?;
            let surface_texture_ref = env.new_global_ref(&surface_texture).map_err(|error| {
                delete_gl_texture(texture);
                format!("promote SurfaceTexture global reference for eye {view_index}: {error}")
            })?;
            let output_surface_ref = env.new_global_ref(&output_surface).map_err(|error| {
                delete_gl_texture(texture);
                format!("promote Surface global reference for eye {view_index}: {error}")
            })?;

            textures.push(texture);
            surface_textures.push(surface_texture_ref);
            output_surfaces.push(output_surface_ref);
            let eye_name = if view_index == 0 { "left" } else { "right" };
            let mut eye = SurfaceTextureOesEyeStatus::for_stream(
                view_index as u32,
                source_kind.stream_label(view_index),
                eye_name,
            )
            .mark_surface_ready();
            eye.source_width = Some(DEFAULT_OES_SURFACE_WIDTH as u32);
            eye.source_height = Some(DEFAULT_OES_SURFACE_HEIGHT as u32);
            status.eyes.push(eye);
        }

        Ok(Self {
            surface_textures,
            output_surfaces,
            decode_probe: None,
            textures,
        })
    }

    pub(super) fn surface_textures(&self) -> &[GlobalRef] {
        &self.surface_textures
    }

    pub(super) fn output_surfaces(&self) -> &[GlobalRef] {
        &self.output_surfaces
    }

    pub(super) fn set_decode_probe(&mut self, decode_probe: GlobalRef) {
        self.decode_probe = Some(decode_probe);
    }

    pub(super) fn has_decode_probe(&self) -> bool {
        self.decode_probe.is_some()
    }

    pub(super) fn surface_texture(&self, view_index: usize) -> Option<&GlobalRef> {
        self.surface_textures.get(view_index)
    }

    pub(super) fn texture(&self, view_index: usize) -> Option<u32> {
        self.textures.get(view_index).copied()
    }

    pub(super) fn release(self, java_vm: &JavaVM) {
        if let Ok(mut env) = java_vm.attach_current_thread() {
            if let Some(decode_probe) = &self.decode_probe {
                let _ = env.call_method(decode_probe.as_obj(), "stop", "()V", &[]);
            }
            for surface in &self.output_surfaces {
                let _ = env.call_method(surface.as_obj(), "release", "()V", &[]);
            }
            for surface_texture in &self.surface_textures {
                let _ = env.call_method(surface_texture.as_obj(), "release", "()V", &[]);
            }
        }
        for texture in &self.textures {
            delete_gl_texture(*texture);
        }
    }
}
