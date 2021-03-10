use crate::core::Engine;
use crate::renderer::draw::draw_text;
use std::ptr::null;
use std::ffi::c_void;
use crate::game_world::world::World;
use nphysics3d::utils::union_find::union;
use crate::game_world::world::{AssetSource};
use nalgebra::Vector3;

static mut SHADER_TEXT_ID: u32 = 0;

pub trait View {
    fn update(&self, engine: &Engine) -> UIResult;
}

#[derive(Debug)]
pub enum UIError {
    UnableToInitializeFramebuffer
}

pub struct TextView {
    text_vao: i32,
    text_vbo: i32,
    text_shader_id: u32,
    pub text: String,
    pub position: (f32, f32),
    pub scale: f32,
}

pub type UIResult = Result<(), UIError>;

impl TextView {
    pub fn new(text: String, position: (f32, f32), scale: f32) -> Self {
        let mut vbo: u32 = 0;
        let mut vao: u32 = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (std::mem::size_of::<f32>() * 6 * 4) as isize,
                null(),
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0, 4,
                gl::FLOAT, gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as i32,
                0 as *const c_void,
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);

            Self {
                text,
                position,
                scale,
                text_vao: vao as i32,
                text_vbo: vbo as i32,
                text_shader_id: SHADER_TEXT_ID as u32,
            }
        }
    }
}

impl View for TextView {
    fn update(&self, engine: &Engine) -> UIResult {
        unsafe {
            draw_text(
                self.text_vao as u32,
                self.text_vbo as u32,
                &engine,
                self.text_shader_id,
                self.text.as_str(),
                self.position.0, self.position.1,
                0.5,
                Vector3::new(1.0, 1.0, 1.0),
            );
        }

        Ok(())
    }
}

pub fn add_ui_element(engine: &mut Engine, view: Box<dyn View>) {
    engine.ui_view.push(view);
}


///Create framebuffer
/// Create shader id
pub fn init_ui(engine: &mut Engine, world: &mut World) -> UIResult {

    let mut fbo: u32 = 0;

    unsafe {
        gl::GenFramebuffers(1, &mut fbo);
    }

    if fbo == 0 {
        return Err(UIError::UnableToInitializeFramebuffer);
    }
    engine.ui_frame_buffer = Some(fbo);

    let shader_id = world.resources.add_resource(AssetSource::Shader(
        String::from("font_vert.glsl"),
        String::from("font_frag.glsl"),
        None,
    ));

    unsafe { SHADER_TEXT_ID = shader_id as u32 };

    Ok(())
}

pub fn propagate_cursor_pos_to_ui(engine: &mut Engine, cords: (f32, f32)){ }