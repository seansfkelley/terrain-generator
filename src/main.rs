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
#[macro_use]
extern crate lazy_static;
extern crate multimap;

mod shaders;
mod controls;
mod camera;
mod util;
mod file;
mod objects;

use std::{ ptr, path, collections };
use std::os::raw::{ c_void, c_char };
use std::ffi::CStr;
use std::sync::mpsc::Receiver;
use glfw::Context;
use gl::types::*;
use wavefront_obj::{ obj, mtl };
use util::assert_no_gl_error;

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

fn mapify_mtl(mtl_set: mtl::MtlSet) -> collections::HashMap<String, mtl::Material> {
    let mut map = collections::HashMap::new();
    for m in mtl_set.materials {
        // TODO: Compiles, but seems bleh. Need to understand borrowing semantics better.
        map.insert(m.name.clone(), m.clone());
    }
    map
}

fn load_local_object<'a>(name: &str, program: &'a shaders::Program) -> objects::RenderableObject<'a> {
    let p = path::Path::new("./objects");
    let o = obj::parse(file::read_file_contents(&*p.join(name))).unwrap();
    let m = match o.material_library {
        Some(mtl_name) => Some(mapify_mtl(mtl::parse(file::read_file_contents(&*p.join(mtl_name))).unwrap())),
        None => None,
    };
    objects::RenderableObject::new(o.objects[1].clone(), m, program)
}

fn render(glfw: &mut glfw::Glfw, window: &mut glfw::Window, events: Receiver<(f64, glfw::WindowEvent)>) {
    glfw.poll_events();
    controls::init_window_controls(window);

    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        assert_no_gl_error();
    }

    let vs_basic = shaders::compile_shader("./shaders/basic_w_color.vert", gl::VERTEX_SHADER);
    let fs_basic = shaders::compile_shader("./shaders/given_color.frag", gl::FRAGMENT_SHADER);
    let program_basic = shaders::Program::new(vs_basic, fs_basic, vec!["mvp"], vec!["in_Position", "in_FragmentColor"]);

    let vs_phong = shaders::compile_shader("./shaders/phong.vert", gl::VERTEX_SHADER);
    let fs_phong = shaders::compile_shader("./shaders/phong.frag", gl::FRAGMENT_SHADER);
    let program_phong = shaders::Program::new(vs_phong, fs_phong, vec!["mvp"], vec!["in_Position", "in_ColorAmbient", "in_ColorDiffuse"]);

    info!("successfully created shaders/program");

    let mut renderables = vec![
        load_local_object("icosahedron.obj", &program_basic),
        load_local_object("dodecahedron.obj", &program_basic),
        load_local_object("shuttle.obj", &program_phong)
    ];
    info!("successfully initialized static data");

    let mut object_to_render = 3;

    let mut last_time = glfw.get_time() as f32;
    let mut camera = camera::Camera::new();
    camera.translate(camera::TranslateDirection::Forward, -2.0);

    info!("beginning event loop");
    while !window.should_close() {
        let t = glfw.get_time() as f32;
        let delta_t = t - last_time;
        last_time = t;

        controls::move_camera_from_inputs(&mut camera, window, delta_t);

        let view = camera.view_mat();
        let projection = camera.projection_mat(ASPECT_RATIO);

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            let index_to_render = object_to_render - 1;
            if index_to_render < renderables.len() {
                renderables[index_to_render].render(view, projection);
            }
        }

        window.swap_buffers();
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            debug!("received event: {:?}", event);
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
                    info!("received esc key, will close window");
                    window.set_should_close(true);
                },
                // TODO: There's got to be some idiomatic way to do this.
                glfw::WindowEvent::Key(glfw::Key::Num1, _, glfw::Action::Press, _) => {
                    object_to_render = 1;
                },
                glfw::WindowEvent::Key(glfw::Key::Num2, _, glfw::Action::Press, _) => {
                    object_to_render = 2;
                },
                glfw::WindowEvent::Key(glfw::Key::Num3, _, glfw::Action::Press, _) => {
                    object_to_render = 3;
                },
                glfw::WindowEvent::Key(glfw::Key::Num4, _, glfw::Action::Press, _) => {
                    object_to_render = 4;
                },
                glfw::WindowEvent::Key(glfw::Key::Num5, _, glfw::Action::Press, _) => {
                    object_to_render = 5;
                },
                glfw::WindowEvent::Key(glfw::Key::Num6, _, glfw::Action::Press, _) => {
                    object_to_render = 6;
                },
                glfw::WindowEvent::Key(glfw::Key::Num7, _, glfw::Action::Press, _) => {
                    object_to_render = 7;
                },
                glfw::WindowEvent::Key(glfw::Key::Num8, _, glfw::Action::Press, _) => {
                    object_to_render = 8;
                },
                glfw::WindowEvent::Key(glfw::Key::Num9, _, glfw::Action::Press, _) => {
                    object_to_render = 9;
                },
                glfw::WindowEvent::Key(glfw::Key::Num0, _, glfw::Action::Press, _) => {
                    object_to_render = 10;
                },
                _ => {}
            }
        }
    }
}
