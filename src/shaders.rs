extern crate gl;

use std::ptr;
use std::str;
use std::ffi::CString;
use gl::types::*;

use file;
use util;

pub fn compile_shader(filename: &str, type_: GLenum) -> GLuint {
    info!("loading shader from {} of type 0x{:X}", filename, type_);

    let shader_src = file::read_file_contents(filename);
    let shader;

    unsafe {
        debug!("creating shader");
        shader = gl::CreateShader(type_);
        util::die_if_zero(shader, "error creating shader");
        util::die_if_gl_error();

        debug!("providing shader source");
        gl::ShaderSource(
            shader,
            1,
            &CString::new(shader_src.as_bytes()).unwrap().as_ptr() as *const *const i8,
            ptr::null());
        util::die_if_gl_error();

        debug!("compiling shader");
        gl::CompileShader(shader);
        util::die_if_gl_error();

        debug!("checking shader status");
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        util::die_if_gl_error();

        if status != (gl::TRUE as GLint) {
            debug!("shader failed to initialize, collecting information");

            debug!("fetching log length");
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);

            debug!("fetching log content (length {})", len);
            let mut buf = Vec::<u8>::with_capacity(len as usize - 1);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);

            debug!("blowing up");
            panic!("{}", str::from_utf8(buf.as_slice()).ok().expect("ShaderInfoLog not valid utf8").trim());
        }
    }

    info!("successfully generated shader for {} with id {}", filename, shader);
    return shader;
}

pub fn link_program(vertex_shader: GLuint, fragment_shader: GLuint) -> GLuint {
    info!("creating program with vertex shader {} and fragment shader {}", vertex_shader, fragment_shader);

    unsafe {
        debug!("creating shader");
        let program = gl::CreateProgram();
        util::die_if_zero(program, "error creating program");
        util::die_if_gl_error();

        debug!("attaching vertex shader");
        gl::AttachShader(program, vertex_shader);
        util::die_if_gl_error();

        debug!("attaching fragment shader");
        gl::AttachShader(program, fragment_shader);
        util::die_if_gl_error();

        debug!("linking program");
        gl::LinkProgram(program);
        util::die_if_gl_error();

        debug!("checking program status");
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
        util::die_if_gl_error();

        if status != (gl::TRUE as GLint) {
            debug!("program failed to initialize, collecting information");
            let mut len: GLint = 0;

            debug!("fetching log length");
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);

            debug!("fetching log content (length {})", len);
            let mut buf = Vec::<u8>::with_capacity(len as usize - 1);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);

            debug!("blowing up");
            panic!("{}", str::from_utf8(buf.as_slice()).ok().expect("ProgramInfoLog not valid utf8").trim());
        }

        info!("successfully created program with id {}", program);
        return program;
    }
}
