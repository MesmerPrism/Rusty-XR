use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    ptr,
};

use super::{
    glAttachShader, glCompileShader, glCreateProgram, glCreateShader, glDeleteProgram,
    glDeleteShader, glGetProgramInfoLog, glGetProgramiv, glGetShaderInfoLog, glGetShaderiv,
    glGetUniformLocation, glLinkProgram, glShaderSource, GL_COMPILE_STATUS, GL_INFO_LOG_LENGTH,
    GL_LINK_STATUS,
};

pub(super) fn compile_shader(shader_type: u32, source: &str) -> Result<u32, String> {
    let source = CString::new(source).map_err(|error| format!("shader CString: {error}"))?;
    unsafe {
        let shader = glCreateShader(shader_type);
        if shader == 0 {
            return Err("glCreateShader returned 0".to_string());
        }
        let ptr = source.as_ptr();
        glShaderSource(shader, 1, &ptr, ptr::null());
        glCompileShader(shader);
        let mut compiled = 0;
        glGetShaderiv(shader, GL_COMPILE_STATUS, &mut compiled);
        if compiled == 0 {
            let info_log = shader_info_log(shader);
            glDeleteShader(shader);
            return Err(format!("OES copy shader compile failed: {info_log}"));
        }
        Ok(shader)
    }
}

pub(super) fn link_program(vertex_shader: u32, fragment_shader: u32) -> Result<u32, String> {
    unsafe {
        let program = glCreateProgram();
        if program == 0 {
            return Err("glCreateProgram returned 0".to_string());
        }
        glAttachShader(program, vertex_shader);
        glAttachShader(program, fragment_shader);
        glLinkProgram(program);
        let mut linked = 0;
        glGetProgramiv(program, GL_LINK_STATUS, &mut linked);
        if linked == 0 {
            let info_log = program_info_log(program);
            glDeleteProgram(program);
            return Err(format!("OES copy program link failed: {info_log}"));
        }
        Ok(program)
    }
}

pub(super) fn uniform_location(program: u32, name: &str) -> Result<c_int, String> {
    let name_cstring =
        CString::new(name).map_err(|error| format!("uniform name CString: {error}"))?;
    let location = unsafe { glGetUniformLocation(program, name_cstring.as_ptr()) };
    if location < 0 {
        Err(format!("shader did not expose uniform {name}"))
    } else {
        Ok(location)
    }
}

pub(super) fn delete_shader(shader: u32) {
    unsafe {
        if shader != 0 {
            glDeleteShader(shader);
        }
    }
}

fn shader_info_log(shader: u32) -> String {
    unsafe {
        let mut length = 0;
        glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut length);
        if length <= 1 {
            return String::from("no shader info log");
        }
        let mut buffer = vec![0_u8; length as usize];
        glGetShaderInfoLog(
            shader,
            length,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast::<c_char>(),
        );
        CStr::from_ptr(buffer.as_ptr().cast::<c_char>())
            .to_string_lossy()
            .into_owned()
    }
}

fn program_info_log(program: u32) -> String {
    unsafe {
        let mut length = 0;
        glGetProgramiv(program, GL_INFO_LOG_LENGTH, &mut length);
        if length <= 1 {
            return String::from("no program info log");
        }
        let mut buffer = vec![0_u8; length as usize];
        glGetProgramInfoLog(
            program,
            length,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast::<c_char>(),
        );
        CStr::from_ptr(buffer.as_ptr().cast::<c_char>())
            .to_string_lossy()
            .into_owned()
    }
}
