use glfw::{Context, WindowEvent};
use std::sync::mpsc::Receiver;

pub struct Display {
    pub glfw: glfw::Glfw,
    pub window: glfw::Window,
    pub events_receiver: Receiver<(f64, WindowEvent)>,
}

pub fn init_gl_window_context(initial_size: (u32, u32), window_name: &str) -> Display {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
    glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));

    let (mut window, events) = glfw
        .create_window(
            initial_size.0,
            initial_size.1,
            window_name,
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create glfw window");

    window.set_pos(300, 100);
    //window.set_cursor_mode(glfw::CursorMode::Disabled);
    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_size_polling(true);

    gl::load_with(|s| window.get_proc_address(s) as *const _);
    gl::Viewport::load_with(|s| window.get_proc_address(s));

    unsafe { gl::Viewport(0, 0, initial_size.0 as i32, initial_size.1 as i32) };

    Display {
        glfw,
        window,
        events_receiver: events,
    }
}
