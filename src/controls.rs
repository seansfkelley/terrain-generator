use glfw;
use camera;

const LOOK_SPEED: f32 = 0.05;
const MOVE_SPEED: f32 = 8.0;

fn get_half_dimensions(window: &glfw::Window) -> (f64, f64) {
    let (width, height) = window.get_size();
    (width as f64 / 2.0, height as f64 / 2.0)
}

pub fn init_window_controls(window: &mut glfw::Window) {
    let (half_width, half_height) = get_half_dimensions(window);

    window.set_cursor_mode(glfw::CursorMode::Disabled);
    window.set_cursor_pos(half_width, half_height);
}

pub fn move_camera_from_mouse(camera: &mut camera::Camera, window: &mut glfw::Window, delta_t: f32) {
    let (half_width, half_height) = get_half_dimensions(window);

    let (mouse_x, mouse_y) = window.get_cursor_pos();
    window.set_cursor_pos(half_width, half_height);

    camera.look(camera::LookDirection::Horizontal, LOOK_SPEED * delta_t * (half_width - mouse_x) as f32);
    camera.look(camera::LookDirection::Vertical, LOOK_SPEED * delta_t * (half_height - mouse_y) as f32);

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
}
