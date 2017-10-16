use glfw;

pub fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
            info!("received esc key, will close window");
            window.set_should_close(true);
        }
        _ => {}
    }
}
