use crate::core::{Engine, FontFace};
use crate::renderer::draw::draw_text;
use nalgebra::Vector3;
use nphysics3d::utils::union_find::union;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::null;

use crate::game_world::world::AssetSource;
use crate::game_world::world::World;
use crate::utils::Cords;

static mut SHADER_TEXT_ID: u32 = 0;
static mut ENGINE_PTR: *const Engine = null();

#[derive(Copy, Clone, Debug)]
pub struct Dimensions<T> {
    pub x: T,
    pub y: T,
}

impl<T> Dimensions<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

pub type ViewDimens = Dimensions<u32>;
pub type ViewPosition = Dimensions<u32>;

pub trait View {
    fn update(&mut self, engine: &Engine) -> UIResult;
    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>);

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }
    fn add_child(&mut self, child: Box<dyn View>) {} //Note(teddy) Only used by container views
    fn update_dimensions(&mut self, dimensions: ViewDimens) {}
    fn get_view_dimensions(&self) -> Option<ViewDimens> {
        None
    }
    fn set_position(&mut self, position: ViewPosition) {}
    fn get_position(&self) -> Option<ViewPosition>;
}

pub struct UITree {
    pub root: Option<Box<dyn View>>,
}

impl UITree {
    pub fn new() -> Self {
        UITree { root: None }
    }
}

#[derive(Debug)]
pub enum UIError {
    UnableToInitializeFramebuffer,
}

pub struct TextView {
    text_vao: i32,
    text_vbo: i32,
    text_shader_id: u32,
    text_length: u32,
    text_height: u32,
    cursor_hover: bool,
    pub text: String,
    pub position: ViewDimens,
    pub size: Option<ViewDimens>,
    pub color: Option<Vector3<f32>>,
    //Note(teddy) Incase the size is not passed, use the fonts width and heights and update this value
    pub scale: f32,

    pub on_hover: Option<Box<dyn FnMut(*mut TextView)>>,
    pub on_mouse_leave: Option<Box<dyn FnMut(*mut TextView)>>,
}

pub type UIResult = Result<(), UIError>;

impl TextView {
    pub fn new(text: String, position: ViewPosition, scale: f32) -> Self {
        let mut vbo: u32 = 0;
        let mut vao: u32 = 0;

        let engine = unsafe { ENGINE_PTR.as_ref().unwrap() };
        let length_of_text = get_the_length_of_text(&text, &engine.font_face);

        let size = Some(Dimensions::new(
            length_of_text,
            engine.font_face.font_size as u32,
        ));
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
                0,
                4,
                gl::FLOAT,
                gl::FALSE,
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
                on_mouse_leave: None,
                color: None,
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

        if !self.cursor_hover && self.on_mouse_leave.is_some() {
            self.on_mouse_leave.as_mut().unwrap()(view);
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
                self.position.x as f32,
                self.position.y as f32,
                1.0,
                color,
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

        if (cords.x > min_x as f32 && cords.x < max_x as f32)
            && (cords.y > min_y as f32 && cords.y < max_y as f32)
        {
            self.cursor_hover = true;
        } else {
            //TODO(teddy) Add on mouse leave event
            self.cursor_hover = false;
        }
    }

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }

    fn get_view_dimensions(&self) -> Option<ViewDimens> {
        self.size
    }

    fn set_position(&mut self, position: ViewPosition) {
        self.position = position;
    }

    fn get_position(&self) -> Option<ViewPosition> {
        Some(self.position)
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
    // unsafe {
    //     for view in (&mut *engine).ui_view.iter_mut() {
    //         view.receive_cursor_cords(&mut *engine, cords);
    //     }
    // }

    unsafe {
        if let Some(view) = &mut (&mut *engine).ui_tree.root {
            view.receive_cursor_cords(&mut *engine, cords);
        }
    }
}

pub struct SimpleUIContainer {
    children: Vec<Box<dyn View>>,
    dimensions: Option<ViewDimens>,
}

impl SimpleUIContainer {
    pub fn new(dimensions: Option<ViewDimens>) -> Self {
        Self {
            children: vec![],
            dimensions,
        }
    }
}

impl View for SimpleUIContainer {
    fn add_child(&mut self, child: Box<dyn View>) {
        self.children.push(child);
    }

    fn update(&mut self, engine: &Engine) -> UIResult {
        //TODO(teddy) This initial position will be the position of the container
        let mut initial_position = 0;
        for view in self.children.iter_mut() {
            if let Some(view_dimens) = view.get_view_dimensions() {
                view.set_position(ViewPosition::new(0, initial_position));
                initial_position += view_dimens.y;
            }

            view.update(engine).unwrap();
        }

        //Draw items in a column
        // panic!("Stopped the process");
        Ok(())
    }

    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>) {
        //TODO(teddy) implement a simple ui container
    }

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);

        for view in self.children.iter_mut() {
            view.receive_cursor_cords(engine, cords)
        }
    }

    fn get_position(&self) -> Option<ViewPosition> {
        None
    }
}
