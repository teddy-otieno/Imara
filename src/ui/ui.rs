use crate::core::{Engine, FontFace};
use crate::renderer::draw::draw_text;
use std::ptr::null;
use std::ffi::c_void;
use nalgebra::Vector3;
use nphysics3d::utils::union_find::union;

use crate::game_world::world::World;
use crate::game_world::world::{AssetSource};
use crate::utils::Cords;

static mut SHADER_TEXT_ID: u32 = 0;
static mut ENGINE_PTR: *const Engine = null();

pub trait View {
    fn update(&mut self, engine: &Engine) -> UIResult;
    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>);

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }
}

#[derive(Debug)]
pub enum UIError {
    UnableToInitializeFramebuffer
}

pub struct Size {
    width: u32,
    height: u32,
}

pub struct TextView {
    text_vao: i32,
    text_vbo: i32,
    text_shader_id: u32,
    text_length: u32,
    text_height: u32,
    cursor_hover: bool,
    pub text: String,
    pub position: Cords<u32>,
    pub size: Option<Size>,
    pub color: Option<Vector3<f32>>,
    //Note(teddy) Incase the size is not passed, use the fonts width and heights and update this value
    pub scale: f32,

    pub on_hover: Option<Box<dyn FnMut(*mut TextView)>>,
}

pub type UIResult = Result<(), UIError>;

impl TextView {
    pub fn new(text: String, position: Cords<u32>, scale: f32, size: Option<Size>) -> Self {
        let mut vbo: u32 = 0;
        let mut vao: u32 = 0;

        let engine = unsafe { ENGINE_PTR.as_ref().unwrap() };
        let length_of_text = get_the_length_of_text(&text, &engine.font_face);

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
                size,
                text_height: engine.font_face.font_size as u32,
                text_length: length_of_text,
                text_vao: vao as i32,
                text_vbo: vbo as i32,
                cursor_hover: false,
                text_shader_id: SHADER_TEXT_ID as u32,
                on_hover: None,
                color: None
            }
        }
    }
}


impl View for TextView {
    fn update(&mut self, engine: &Engine) -> UIResult {

        let view: *mut TextView = self;
        if self.cursor_hover && self.on_hover.is_some() {
            self.on_hover.as_mut().unwrap()(view);
        }

        let default_text_color: Vector3<f32> = Vector3::new(1.0, 1.0, 1.0);
        let color = match &self.color {
            Some(color) => color,
            None => &default_text_color,
        };

        unsafe {
            draw_text(
                self.text_vao as u32,
                self.text_vbo as u32,
                &engine,
                self.text_shader_id,
                self.text.as_str(),
                self.position.x as f32, self.position.y as f32,
                1.0,
                color
            );
        }

        Ok(())
    }

    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>) {
        let min_x = self.position.x;
        let min_y = self.position.y;

        //FIXME(teddy): The length doesn't seem to match the actual screen length
        let max_x = min_x + self.text_length;
        let max_y = min_y + self.text_height;

        if (cords.x > min_x as f32 && cords.x < max_x as f32) && (cords.y > min_y as f32 && cords.y < max_y as f32) {
            self.cursor_hover = true;
        } else {

            //TODO(teddy) Add on mouse leave event
            self.color = Some(Vector3::new(1.0, 1.0, 1.0));
            self.cursor_hover = false;
        }

    }

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }
}


fn get_the_length_of_text(text: &String, font_face: &FontFace) -> u32 {
    let mut length = 0;
    for c in text.chars() {
        let font_char = &font_face.chars[&c];

        length += (font_char.advance >> 6) as u32;
    }

    length
}

pub fn add_ui_element(engine: &mut Engine, view: Box<dyn View>) {
    engine.ui_view.push(view);
}


///Create framebuffer
/// Create shader id
pub fn init_ui(engine: &mut Engine, world: &mut World) -> UIResult {
    let mut fbo: u32 = 0;


    unsafe {
        ENGINE_PTR = engine;

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

pub fn propagate_cursor_pos_to_ui(engine: *mut Engine, cords: Cords<f32>) {
    unsafe {
        for view in (&mut *engine).ui_view.iter_mut() {
            view.receive_cursor_cords(&mut *engine, cords);
        }
    }
}
