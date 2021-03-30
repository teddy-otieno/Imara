use crate::core::{Engine, FontFace};
use crate::renderer::draw::{draw_quad_with_default_shader, draw_text};
use nalgebra::Vector3;
use std::ffi::c_void;
use std::ptr::null;

use crate::game_world::world::AssetSource;
use crate::game_world::world::World;
use crate::utils::Cords;

static mut SHADER_TEXT_ID: u32 = 0;
pub static mut UI_QUAD_SHADER_ID: u32 = 0;
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

impl Dimensions<u32> {
    pub fn zerod() -> Self {
        Self { x: 0, y: 0 }
    }
}

pub type ViewDimens = Dimensions<u32>;
pub type ViewPosition = Dimensions<u32>;

pub trait View {
    fn get_id(&self) -> &str;
    fn update(&mut self, engine: &Engine) -> UIResult;
    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>);

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }
    fn update_dimensions(&mut self, _dimensions: ViewDimens) {}
    fn get_view_dimensions(&self) -> Option<ViewDimens> {
        None
    }
    fn set_position(&mut self, _position: ViewPosition) {}
    fn get_position(&self) -> Option<ViewPosition>;
}

///Note(teddy) Container specific methods.
///Container is also a view so each container
///must implement the View Trait
pub trait ViewContainer: View {
    fn add_child(&mut self, _child: Box<dyn View>); //Note(teddy) Only used by container views
    fn remove_child(&mut self, child_id: &str) -> UIResult; //Note(teddy) keep the ids immutable

    ///Note(teddy) Iterate throught the entire container children to find the view id
    fn get_view_by_id(&self, child_id: &str) -> Result<&Box<dyn View>, UIError>;
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
    ViewNotFound,
}

pub struct TextView {
    id: Box<str>,
    text_vao: i32,
    text_vbo: i32,
    background_vao: i32,
    background_vbo: i32,
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

#[inline]
unsafe fn initialize_background_buffers() -> (i32, i32) {
    let mut vao: u32 = 0;
    let mut vbo: u32 = 0;

    gl::GenVertexArrays(1, &mut vao);
    gl::GenBuffers(1, &mut vbo);

    gl::BindVertexArray(vao);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        (6 * 2 * std::mem::size_of::<f32>()) as isize,
        null(),
        gl::DYNAMIC_DRAW,
    );

    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(
        0,
        2, //Using vec2 when drawing quads inside the shader
        gl::FLOAT,
        gl::FALSE,
        (2 * std::mem::size_of::<f32>()) as i32,
        0 as *const c_void,
    );

    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::BindVertexArray(0);

    (vao as i32, vbo as i32)
}

impl TextView {
    pub fn new(id: Box<str>, text: String, position: ViewPosition, scale: f32) -> Self {
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

            let (background_vao, background_vbo) = initialize_background_buffers();

            Self {
                id,
                text,
                position,
                scale,
                size,
                text_height: engine.font_face.font_size as u32,
                text_length: length_of_text,
                text_vao: vao as i32,
                text_vbo: vbo as i32,
                background_vao,
                background_vbo,
                cursor_hover: false,
                text_shader_id: SHADER_TEXT_ID,
                on_hover: None,
                on_mouse_leave: None,
                color: None,
            }
        }
    }
}

impl View for TextView {
    fn get_id(&self) -> &str {
        &(*self.id)
    }

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
            let size = self.size.unwrap();
            draw_quad_with_default_shader(
                engine,
                self.background_vao as u32,
                self.background_vbo as u32,
                (
                    self.position.x as f32,
                    (self.position.y + engine.font_face.font_size as u32) as f32,
                ),
                (size.y as f32, size.x as f32),
                &[0.2, 0.2, 0.2],
            );
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

    fn compute_intersect_with_cursor_cords(&mut self, _engine: &Engine, cords: &Cords<f32>) {
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

    let quad_shader = world.resources.add_resource(AssetSource::Shader(
        String::from("ui_quad_vert.glsl"),
        String::from("ui_quad_frag.glsl"),
        None,
    ));

    unsafe {
        SHADER_TEXT_ID = shader_id as u32;
        UI_QUAD_SHADER_ID = quad_shader as u32;
    };
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

pub enum Orientation {
    Vertical,
    Horizontal,
}

pub struct SimpleUIContainer {
    id: Box<str>,
    children: Vec<Box<dyn View>>,
    dimensions: Option<ViewDimens>,
    position: Option<ViewPosition>,
    orientation: Orientation,
}

impl SimpleUIContainer {
    pub fn new(
        id: Box<str>,
        dimensions: Option<ViewDimens>,
        position: Option<ViewPosition>,
        orientation: Orientation,
    ) -> Self {
        Self {
            id,
            children: vec![],
            dimensions,
            position,
            orientation,
        }
    }

    fn recalculate_dimensions(&self) {
        let mut new_dimensions = ViewDimens::zerod();

        //Note(teddy) Updating the length and height based on the orientation of the container
        match self.orientation {
            Orientation::Vertical => {
                let mut height: u32 = 0;
                let mut view_dimens = ViewDimens::zerod();

                for child in self.children.iter() {
                    view_dimens = child.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    new_dimensions.x = std::cmp::max(view_dimens.x, new_dimensions.x);
                    height += view_dimens.y;
                }

                new_dimensions.y = height;
            }

            Orientation::Horizontal => {
                let mut width: u32 = 0;
                let mut view_dimens = ViewDimens::zerod();

                for child in self.children.iter() {
                    view_dimens = child.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    new_dimensions.y = std::cmp::max(view_dimens.y, new_dimensions.y);
                    width += view_dimens.x;
                }

                new_dimensions.x = width;
            }
        }
    }
}

impl View for SimpleUIContainer {
    fn get_id(&self) -> &str {
        &(self.id)
    }

    fn update(&mut self, engine: &Engine) -> UIResult {
        //TODO(teddy) This initial position will be the position of the container
        let mut initial_position = self.position.unwrap().y;
        for view in self.children.iter_mut() {
            view.set_position(ViewPosition::new(
                self.position.unwrap().x,
                initial_position,
            ));
            if let Some(view_dimens) = view.get_view_dimensions() {
                initial_position += view_dimens.y + 10;
            }

            view.update(engine).unwrap();
        }

        //Draw items in a column
        // panic!("Stopped the process");
        Ok(())
    }

    fn compute_intersect_with_cursor_cords(&mut self, _engine: &Engine, cords: &Cords<f32>) {
        //TODO(teddy) implement a simple ui container
    }

    fn receive_cursor_cords(&mut self, engine: &mut Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);

        for view in self.children.iter_mut() {
            view.receive_cursor_cords(engine, cords);
        }
    }

    fn get_position(&self) -> Option<ViewPosition> {
        None
    }
}

impl ViewContainer for SimpleUIContainer {
    fn add_child(&mut self, child: Box<dyn View>) {
        self.children.push(child);
        self.recalculate_dimensions();
    }

    fn get_view_by_id(&self, _child_id: &str) -> Result<&Box<dyn View>, UIError> {
        unimplemented!();
    }

    fn remove_child(&mut self, child_id: &str) -> UIResult {
        if let Some(index) = self
            .children
            .iter()
            .position(|child| child.get_id() == child_id)
        {
            self.children.remove(index);
            Ok(())
        } else {
            Err(UIError::ViewNotFound)
        }
    }
}
