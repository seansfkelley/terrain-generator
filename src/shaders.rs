extern crate gl;

use std::fs;
use std::mem;
use std::ptr;
use std::str;
use std::ffi::CString;
use std::io::Read;
use gl::types::*;

pub fn compile_shader(filename: &str, type_: GLenum) -> GLuint {
    let mut shader_src: String = String::new();
    fs::File::open(filename).unwrap().read_to_string(&mut shader_src).unwrap();
    let shader_src_c_str = CString::new(shader_src.as_bytes()).unwrap().into_raw();
    let shader;

    unsafe {
        shader = gl::CreateShader(type_);
        gl::ShaderSource(shader, 1, mem::transmute(&shader_src_c_str), ptr::null());
        gl::CompileShader(shader);

        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);


        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::new();
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", str::from_utf8(buf.as_slice()).ok().expect("ShaderInfoLog not valid utf8"));
        }
    }

    return shader;
}

pub fn link_program(vs: GLuint, fs: GLuint) -> GLuint {

    unsafe {
        let program = gl::CreateProgram();

        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::new();
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", str::from_utf8(buf.as_slice()).ok().expect("ProgramInfoLog not valid utf8"));
        }

        return program;
    }
}
