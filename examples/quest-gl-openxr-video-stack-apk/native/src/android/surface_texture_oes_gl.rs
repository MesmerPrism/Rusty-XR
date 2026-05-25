use super::{glBindTexture, glGetError, GL_NO_ERROR, GL_TEXTURE_EXTERNAL_OES};
use std::os::raw::c_int;

const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: u32 = 0x2601;
const GL_CLAMP_TO_EDGE: u32 = 0x812F;

#[link(name = "GLESv3")]
unsafe extern "C" {
    fn glGenTextures(n: c_int, textures: *mut u32);
    fn glDeleteTextures(n: c_int, textures: *const u32);
    fn glTexParameteri(target: u32, pname: u32, param: c_int);
}

pub(super) fn create_external_oes_texture() -> Result<u32, String> {
    unsafe {
        while glGetError() != GL_NO_ERROR {}
        let mut texture = 0;
        glGenTextures(1, &mut texture);
        if texture == 0 {
            return Err("glGenTextures returned texture id 0 for external OES texture".into());
        }
        glBindTexture(GL_TEXTURE_EXTERNAL_OES, texture);
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_MIN_FILTER,
            GL_LINEAR as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_MAG_FILTER,
            GL_LINEAR as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_WRAP_S,
            GL_CLAMP_TO_EDGE as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_WRAP_T,
            GL_CLAMP_TO_EDGE as c_int,
        );
        glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0);
        let error = glGetError();
        if error != GL_NO_ERROR {
            delete_gl_texture(texture);
            return Err(format!(
                "external OES texture setup returned GL error 0x{error:04x}"
            ));
        }
        Ok(texture)
    }
}

pub(super) fn delete_gl_texture(texture: u32) {
    if texture != 0 {
        unsafe {
            glDeleteTextures(1, &texture);
        }
    }
}

pub(super) fn identity_texture_transform() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}
