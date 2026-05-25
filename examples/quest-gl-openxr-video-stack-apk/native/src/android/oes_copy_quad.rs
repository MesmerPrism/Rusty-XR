use std::{
    mem,
    os::raw::{c_int, c_void},
    ptr,
};

use super::{
    glBindBuffer, glBufferData, glDeleteBuffers, glDisableVertexAttribArray, glDrawArrays,
    glEnableVertexAttribArray, glGenBuffers, glVertexAttribPointer, GL_ARRAY_BUFFER, GL_FLOAT,
    GL_STATIC_DRAW, GL_TRIANGLE_STRIP,
};

pub(super) struct OesCopyQuad {
    vertex_buffer: u32,
}

impl OesCopyQuad {
    pub(super) fn new() -> Result<Self, String> {
        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0, //
            1.0, -1.0, 1.0, 0.0, //
            -1.0, 1.0, 0.0, 1.0, //
            1.0, 1.0, 1.0, 1.0,
        ];
        let mut vertex_buffer = 0;
        unsafe {
            glGenBuffers(1, &mut vertex_buffer);
            if vertex_buffer == 0 {
                return Err("glGenBuffers returned 0 for OES copy quad".to_string());
            }
            glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
            glBufferData(
                GL_ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as isize,
                vertices.as_ptr().cast(),
                GL_STATIC_DRAW,
            );
            glBindBuffer(GL_ARRAY_BUFFER, 0);
        }
        Ok(Self { vertex_buffer })
    }

    pub(super) fn draw(&self) {
        unsafe {
            glBindBuffer(GL_ARRAY_BUFFER, self.vertex_buffer);
            let stride = (4 * mem::size_of::<f32>()) as c_int;
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 2, GL_FLOAT, 0, stride, ptr::null());
            glEnableVertexAttribArray(1);
            glVertexAttribPointer(
                1,
                2,
                GL_FLOAT,
                0,
                stride,
                (2 * mem::size_of::<f32>()) as *const c_void,
            );
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
            glDisableVertexAttribArray(0);
            glDisableVertexAttribArray(1);
            glBindBuffer(GL_ARRAY_BUFFER, 0);
        }
    }
}

impl Drop for OesCopyQuad {
    fn drop(&mut self) {
        unsafe {
            if self.vertex_buffer != 0 {
                glDeleteBuffers(1, &self.vertex_buffer);
            }
        }
    }
}
