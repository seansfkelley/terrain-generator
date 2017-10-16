#![feature(libc)]

extern crate glfw;
extern crate gl;
extern crate glm;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate num_traits;
extern crate wavefront_obj;

mod shaders;
mod controls;
mod camera;
mod util;
mod event_handlers;
mod file;

use std::mem;
use std::ptr;
use std::vec;
use std::os::raw::{ c_void, c_char };
use std::ffi::{ CString, CStr };
use std::sync::mpsc::Receiver;
use num_traits::identities::One;
use glfw::Context;
use gl::types::*;
use wavefront_obj::obj;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const ASPECT_RATIO: f32 = (WIDTH as f32) / (HEIGHT as f32);

extern "system" fn gl_debug_message(
    source: GLenum,
    type_: GLenum,
    id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const GLchar,
    _user_param: *mut c_void
) {
    unsafe {
        debug!(
            "OpenGL [{}]: (source: {}, type: {}, id: {}) {}",
            severity,
            source,
            type_,
            id,
            CStr::from_ptr(message).to_str().unwrap());
    }
}

fn main() {
    env_logger::init().unwrap();

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    info!("successfully initialized GLFW");

    glfw.window_hint(glfw::WindowHint::Samples(Option::Some(4)));
    glfw.window_hint(glfw::WindowHint::ContextVersionMajor(4));
    glfw.window_hint(glfw::WindowHint::ContextVersionMinor(0));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    glfw.window_hint(glfw::WindowHint::OpenGlDebugContext(true));

    let (mut window, events) = glfw
        .create_window(WIDTH, HEIGHT, "terrain-generator", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    info!("successfully created window");

    window.set_key_polling(true);
    window.make_current();

    gl::load_with(|s| window.get_proc_address(s) as *const _);

    unsafe {
        info!("OpenGL version {}, GLSL version {}",
            CStr::from_ptr(gl::GetString(gl::VERSION) as *const c_char).to_string_lossy(),
            CStr::from_ptr(gl::GetString(gl::SHADING_LANGUAGE_VERSION) as *const c_char).to_string_lossy());

        let mut major_version = -1;
        let mut minor_version = -1;
        gl::GetIntegerv(gl::MAJOR_VERSION, &mut major_version);
        gl::GetIntegerv(gl::MINOR_VERSION, &mut minor_version);
        if major_version > 4 || (major_version == 4 && minor_version >= 3) {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(gl_debug_message, ptr::null());
            gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, ptr::null(), gl::TRUE);
            info!("enabled OpenGL logging at 'debug' level");
        } else {
            // Fucking OS X is using a 7-year-old OpenGL version that doesn't support this.
            warn!("OpenGL version is too old; will not enable OpenGL debug logging");
        }
    }

    render(&mut glfw, &mut window, events);
}

fn render(glfw: &mut glfw::Glfw, window: &mut glfw::Window, events: Receiver<(f64, glfw::WindowEvent)>) {
    glfw.poll_events();
    controls::init_window_controls(window);

    let vs = shaders::compile_shader("./shaders/basic.vert", gl::VERTEX_SHADER);
    let fs = shaders::compile_shader("./shaders/white.frag", gl::FRAGMENT_SHADER);
    let obj_file = obj::parse(file::read_file_contents("./objects/icosahedron.obj"))
        .unwrap();

    let mut vertices = vec::Vec::<obj::Vertex>::new();

    // I tried noodling around with flat_map, but type system complains in ways I don't yet understand how to fix.
    obj_file
        .objects
        .iter()
        .filter(|o| o.vertices.len() > 0)
        .for_each(|o| {
            o
                .geometry
                .iter()
                .for_each(|g| {
                    g
                        .shapes
                        .iter()
                        .for_each(|s| {
                            match s.primitive {
                                obj::Primitive::Triangle(
                                    (v1, _, _),
                                    (v2, _, _),
                                    (v3, _, _),
                                ) => {
                                    vertices.push(o.vertices[v1]);
                                    vertices.push(o.vertices[v2]);
                                    vertices.push(o.vertices[v3]);
                                },
                                _ => { panic!("got non-triangle primitive"); },
                            }
                        })
                })
        });

    let mut flattened_vertices = vec::Vec::<f32>::new();

    vertices.iter()
        .for_each(|v| {
            flattened_vertices.push(v.x as f32);
            flattened_vertices.push(v.y as f32);
            flattened_vertices.push(v.z as f32);
        });

    let program = shaders::link_program(vs, fs);

    info!("successfully created shaders/program");

    let mut vao = 0;
    let mut vbo = 0;

    let matrix_id;

    unsafe {
        gl::Enable(gl::CULL_FACE);

        // VAO
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // VBO
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (flattened_vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            flattened_vertices.as_ptr() as *const _,
            gl::STATIC_DRAW);

        // initialize shaders
        gl::UseProgram(program);

        // MVP
        matrix_id = gl::GetUniformLocation(program, CString::new("mvp").unwrap().as_ptr());

        gl::BindFragDataLocation(
            program,
            0,
            CString::new("out_Color").unwrap().as_ptr());

        // vertex data layout
        let position_attrib = gl::GetAttribLocation(
            program,
            CString::new("in_Position").unwrap().as_ptr()) as GLuint;
        gl::EnableVertexAttribArray(position_attrib);
        gl::VertexAttribPointer(
            position_attrib,
            3,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            0,
            ptr::null());
    }

    info!("successfully initialized static data");
    info!("beginning event loop");

    let mut last_time = glfw.get_time() as f32;
    let mut camera = camera::Camera::new();

    while !window.should_close() {
        let t = glfw.get_time() as f32;
        let delta_t = t - last_time;
        last_time = t;

        controls::move_camera_from_mouse(&mut camera, window, delta_t);

        let model_mat = glm::Mat4::one();
        let mvp = camera.projection_mat(ASPECT_RATIO) * camera.view_mat() * model_mat;
        let mvp_array = util::arrayify_mat4(mvp);

        unsafe {
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UniformMatrix4fv(matrix_id, 1, gl::FALSE, &*mvp_array as *const f32);
            gl::DrawArrays(gl::TRIANGLES, 0, flattened_vertices.len() as i32);
        }

        window.swap_buffers();
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            event_handlers::handle_window_event(window, event);
        }
    }
}
