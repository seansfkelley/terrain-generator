extern crate gl;

use std::{ ptr, str };
use std::vec::Vec;
use std::collections::HashMap;
use std::ffi::CString;
use gl::types::*;

use file;
use util::assert_no_gl_error;

pub fn compile_shader(filename: &str, type_: GLenum) -> GLuint {
    info!("loading shader from {} of type 0x{:X}", filename, type_);

    let shader_src = file::read_file_contents(filename);
    let shader;

    unsafe {
        debug!("creating shader");
        shader = gl::CreateShader(type_);
        assert_ne!(shader, 0, "error creating shader");
        assert_no_gl_error();

        debug!("providing shader source");
        gl::ShaderSource(
            shader,
            1,
            &CString::new(shader_src.as_bytes()).unwrap().as_ptr() as *const *const i8,
            ptr::null());
        assert_no_gl_error();

        debug!("compiling shader");
        gl::CompileShader(shader);
        assert_no_gl_error();

        debug!("checking shader status");
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        assert_no_gl_error();

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

#[derive(Debug, Clone)]
pub struct Program {
    pub name: GLuint,
    uniforms: HashMap<String, GLint>,
    attribs: HashMap<String, GLint>,
}

impl Program {
    pub fn new(vertex_shader: GLuint, fragment_shader: GLuint, uniforms: Vec<&str>, attribs: Vec<&str>) -> Program {
        info!("creating program with vertex shader {} and fragment shader {}", vertex_shader, fragment_shader);

        unsafe {
            debug!("creating shader");
            let program = gl::CreateProgram();
            assert_ne!(program, 0, "error creating program");
            assert_no_gl_error();

            debug!("attaching vertex shader");
            gl::AttachShader(program, vertex_shader);
            assert_no_gl_error();

            debug!("attaching fragment shader");
            gl::AttachShader(program, fragment_shader);
            assert_no_gl_error();

            debug!("linking program");
            gl::LinkProgram(program);
            assert_no_gl_error();

            debug!("detaching shaders");
            gl::DetachShader(program, vertex_shader);
            gl::DetachShader(program, fragment_shader);
            assert_no_gl_error();

            debug!("checking program status");
            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            assert_no_gl_error();

            if status != (gl::TRUE as GLint) {
                debug!("program failed to initialize, collecting information");

                debug!("fetching log length");
                let mut len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);

                debug!("fetching log content (length {})", len);
                let mut buf = Vec::<u8>::with_capacity(len as usize - 1);
                buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
                gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);

                debug!("deleting program");
                gl::DeleteProgram(program);

                debug!("blowing up");
                panic!("{}", str::from_utf8(buf.as_slice()).ok().expect("ProgramInfoLog not valid utf8").trim());
            }

            info!("successfully created program with id {}", program);

            debug!("fetching uniform locations");
            let mut uniforms_map: HashMap<String, GLint> = HashMap::new();
            for u in uniforms {
                debug!("fetching for uniform {}", u);
                let location = gl::GetUniformLocation(program, CString::new(u.as_bytes()).unwrap().as_ptr());
                assert_no_gl_error();
                assert_ne!(location, -1i32, "uniform {} not found in program", location);
                debug!("received location {}", location);
                uniforms_map.insert(u.to_owned(), location);
            }

            debug!("fetching attribute locations");
            let mut attribs_map: HashMap<String, GLint> = HashMap::new();
            for a in attribs {
                debug!("fetching for attribute {}", a);
                let location = gl::GetAttribLocation(program, CString::new(a.as_bytes()).unwrap().as_ptr());
                assert_no_gl_error();
                assert_ne!(location, -1i32, "attribute {} not found in program", location);
                debug!("received location {}", location);
                attribs_map.insert(a.to_owned(), location);
            }

            Program {
                name: program,
                uniforms: uniforms_map,
                attribs: attribs_map,
            }
        }
    }

    pub fn get_uniform(&self, uniform: &str) -> GLint {
        *self.uniforms.get(uniform).expect(&format!("unknown uniform {}", uniform))
    }

    pub fn get_attrib(&self, attrib: &str) -> GLint {
        *self.attribs.get(attrib).expect(&format!("unknown attrib {}", attrib))
    }
}
