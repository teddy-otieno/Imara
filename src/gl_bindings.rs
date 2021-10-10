use std::sync::mpsc::Receiver;
use std::ffi::{c_void, CString};
use std::convert::TryInto;
use glfw::{Context, WindowEvent};

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

    unsafe { 
        gl::Viewport(0, 0, initial_size.0 as i32, initial_size.1 as i32);
        gl::Enable(gl::DEBUG_OUTPUT);
        gl::DebugMessageCallback(Some(message_callback), 0 as *const c_void);
    };


    // glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    Display {
        glfw,
        window,
        events_receiver: events,
    }
}


extern "system" fn message_callback(
    source: gl::types::GLenum, 
    e_type: gl::types::GLenum, 
    id: gl::types::GLuint,
    severity: gl::types::GLenum,
    length: gl::types::GLsizei,
    message: *const gl::types::GLchar,
    user_param: *mut c_void,
) {

    let mut message_buffer = Vec::with_capacity(length.try_into().unwrap());

    unsafe {
        for i in 0..length { 
            message_buffer.push(*message.offset(i.try_into().unwrap())) 
        }

        let message_bytes: Vec<u8> = message_buffer.into_iter().map(|x| x as u8).collect();
        let c_string = CString::from_vec_unchecked(message_bytes);

        eprintln!("GL CALLBACK: type = {}, severity = {}, {:?}", e_type, severity, c_string);
    }

}

