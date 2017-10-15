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
mod camera;

use std::mem;
use std::ptr;
use std::os::raw::{ c_void, c_char };
use std::ffi::{ CString, CStr };
use num_traits::identities::One;
use glfw::Context;
use gl::types::*;

// Vertex data
static VERTEX_DATA: [GLfloat; 6] = [
    0.0,  0.5,
    0.5, -1.0,
    -0.5, -1.0
];

const WIDTH: u32 = 400;
const HALF_WIDTH: f32 = (WIDTH as f32) / 2.0;
const HEIGHT: u32 = 300;
const HALF_HEIGHT: f32 = (HEIGHT as f32) / 2.0;
const ASPECT_RATIO: f32 = (WIDTH as f32) / (HEIGHT as f32);

const LOOK_SPEED: f32 = 0.05;
const MOVE_SPEED: f32 = 8.0;

fn to_c_str(s: &str) -> *mut c_char {
    return CString::new(s).unwrap().into_raw();
}

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

fn arrayify_matrix(mat: glm::Mat4) -> *const f32 {
    let mut array = Vec::new();

    for i in 0..4 {
        for j in 0..4 {
            array.push(mat[i][j]);
        }
    }

    return array.as_ptr();
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
    glfw.poll_events();
    window.set_cursor_mode(glfw::CursorMode::Disabled);
    window.set_cursor_pos(HALF_WIDTH as f64, HALF_HEIGHT as f64);

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

    let vs = shaders::compile_shader("./shaders/basic.vert", gl::VERTEX_SHADER);
    let fs = shaders::compile_shader("./shaders/white.frag", gl::FRAGMENT_SHADER);

    let program = shaders::link_program(vs, fs);

    info!("successfully created shaders/program");

    let mut vao = 0;
    let mut vbo = 0;

    let matrix_id;

    unsafe {
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

        // MVP
        matrix_id = gl::GetUniformLocation(program, to_c_str("mvp"));

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

    let mut last_time = glfw.get_time() as f32;
    let mut camera = camera::Camera::new();

    while !window.should_close() {
        let t = glfw.get_time() as f32;
        let delta_t = t - last_time;
        last_time = t;

        let (mouse_x, mouse_y) = window.get_cursor_pos();
        window.set_cursor_pos(HALF_WIDTH as f64, HALF_HEIGHT as f64);

        camera.look(camera::LookDirection::Horizontal, LOOK_SPEED * delta_t * (HALF_WIDTH - mouse_x as f32));
        camera.look(camera::LookDirection::Vertical, LOOK_SPEED * delta_t * (HALF_HEIGHT - mouse_y as f32));

        if window.get_key(glfw::Key::W) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Forward, delta_t * MOVE_SPEED);
        }
        if window.get_key(glfw::Key::S) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Forward, -delta_t * MOVE_SPEED);
        }
        if window.get_key(glfw::Key::A) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Side, -delta_t * MOVE_SPEED);
        }
        if window.get_key(glfw::Key::D) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Side, delta_t * MOVE_SPEED);
        }
        if window.get_key(glfw::Key::R) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Altitude, delta_t * MOVE_SPEED);
        }
        if window.get_key(glfw::Key::F) == glfw::Action::Press {
            camera.translate(camera::TranslateDirection::Altitude, -delta_t * MOVE_SPEED);
        }

        let projection = glm::ext::perspective(glm::builtin::radians(camera.field_of_view), ASPECT_RATIO, 0.1, 100.0);
        let view = glm::ext::look_at(
            camera.pos,
            camera.pos + camera.direction(),
            camera.up(),
        );
        let model = glm::Mat4::one();

        let mvp = projection * view * model;

        unsafe {
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            // apparently the following line must be after shader initialization/selection/linking/whatever else it doesn't work
            gl::UniformMatrix4fv(matrix_id, 1, gl::FALSE, arrayify_matrix(mvp));
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
