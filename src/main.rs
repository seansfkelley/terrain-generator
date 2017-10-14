#![feature(libc)]

extern crate glfw;
extern crate gl;
extern crate glm;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate num_traits;

mod shaders;

use std::mem;
use std::ptr;
use std::ffi::CString;
use num_traits::identities::One;
use glfw::Context;
use gl::types::*;

// Vertex data
static VERTEX_DATA: [GLfloat; 6] = [
    0.0,  0.5,
    0.5, -0.5,
    -0.5, -0.5
];

static WIDTH: u32 = 400;
static HEIGHT: u32 = 300;

fn to_c_str(s: &str) -> *mut std::os::raw::c_char {
    return CString::new(s).unwrap().into_raw();
}

fn main() {
    env_logger::init().unwrap();

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    info!("successfully initialized GLFW");

    glfw.window_hint(glfw::WindowHint::Samples(Option::Some(4)));
    glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
    glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, events) = glfw
        .create_window(WIDTH, HEIGHT, "terrain-generator", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    info!("successfully created window");

    window.set_key_polling(true);
    window.make_current();

    gl::load_with(|s| window.get_proc_address(s) as *const _);

    let vs = shaders::compile_shader("./shaders/basic.vert", gl::VERTEX_SHADER);
    let fs = shaders::compile_shader("./shaders/white.frag", gl::FRAGMENT_SHADER);

    let program = shaders::link_program(vs, fs);

    info!("successfully created shaders/program");

    let mut vao = 0;
    let mut vbo = 0;

    let projection = glm::ext::perspective(glm::builtin::radians(45.0), (WIDTH as f32) / (HEIGHT as f32), 0.1, 100.0);
    let view = glm::ext::look_at(
        glm::Vec3::new(4.0, 3.0, 3.0),
        glm::Vec3::new(0.0, 0.0, 0.0),
        glm::Vec3::new(0.0, 1.0, 0.0)
    );
    let model = glm::Mat4::one();

    let mvp = projection * view * model;

    unsafe {
        // MVP
        let matrix_id = gl::GetUniformLocation(program, to_c_str("mvp"));
        gl::UniformMatrix4fv(matrix_id, 1, gl::FALSE, &mvp[0][0]);

        // VAO
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // VBO
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            mem::transmute(&VERTEX_DATA[0]),
            gl::STATIC_DRAW);

        // initialize shaders
        gl::UseProgram(program);

        gl::BindFragDataLocation(
            program,
            0,
            to_c_str("out_Color"));

        // vertex data layout
        let position_attrib = gl::GetAttribLocation(
            program,
            to_c_str("in_Position")) as GLuint;
        gl::EnableVertexAttribArray(position_attrib);
        gl::VertexAttribPointer(
            position_attrib,
            2,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            0,
            ptr::null());
    }

    info!("successfully initialized static data");
    info!("beginning event loop");

    while !window.should_close() {
        unsafe {
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        window.swap_buffers();
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            handle_window_event(&mut window, event);
        }
    }
}

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
            info!("received esc key, will close window");
            window.set_should_close(true);
        }
        _ => {}
    }
}
