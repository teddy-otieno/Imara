use std::ffi::c_void;
use std::ptr::null;

use glfw::MouseButton;
use nalgebra::Vector3;

use crate::core::{Engine, FontFace};
use crate::game_world::world::AssetSource;
use crate::game_world::world::World;
use crate::renderer::draw::{draw_quad_with_default_shader, draw_text};
use crate::utils::{get_at_index, Cords};

static mut SHADER_TEXT_ID: u32 = 0;
pub static mut UI_QUAD_SHADER_ID: u32 = 0;
static mut ENGINE_PTR: *const Engine = null();


macro_rules! font_shader {
    () => {String::from("font_shader")}
}

macro_rules! quad_shader {
    () => {String::from("quad_shader")}
}

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

impl Dimensions<i32> {
    pub fn zerod() -> Self {
        Self { x: 0, y: 0 }
    }
}

pub type ViewDimens = Dimensions<i32>;
pub type ViewPosition = Dimensions<i32>;

pub trait View {
    fn get_id(&self) -> &str;
    fn update(&mut self, engine: &Engine) -> UIResult;
    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>);

    fn receive_cursor_cords(&mut self, engine: &Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }

    fn handle_button_click(
        &mut self,
        engine: &Engine,
        clicked_buttons: &Vec<MouseButton>,
        cords: Cords<f32>,
    ) -> bool;

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

#[derive(PartialEq)]
enum CursorState {
    Hover,
    Leave,
    Neither,
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
    cursor_hover: CursorState,

    pub text: String,
    pub position: ViewDimens,
    pub size: Option<ViewDimens>,
    pub color: Option<Vector3<f32>>,
    pub background_color: [f32; 3],
    pub padding: i32,
    //Note(teddy) Incase the size is not passed, use the fonts width and heights and update this value
    pub scale: f32,

    pub on_hover: Option<Box<dyn FnMut(*mut TextView)>>,
    pub on_mouse_leave: Option<Box<dyn FnMut(*mut TextView)>>,
    pub on_click: Option<Box<dyn Fn(*mut TextView)>>,
    pub on_right_click: Option<Box<dyn Fn(*mut TextView)>>,
    pub on_middle_click: Option<Box<dyn Fn(*mut TextView)>>,
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
        (6 * 3 * std::mem::size_of::<f32>()) as isize,
        null(),
        gl::DYNAMIC_DRAW,
    );

    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(
        0,
        3, //Using vec3 when drawing quads inside the shader
        gl::FLOAT,
        gl::FALSE,
        (3 * std::mem::size_of::<f32>()) as i32,
        0 as *const c_void,
    );

    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::BindVertexArray(0);

    (vao as i32, vbo as i32)
}

impl TextView {
    pub fn new(
        id: Box<str>,
        text: String,
        position: ViewPosition,
        scale: f32,
        padding: i32,
    ) -> Self {
        let mut vbo: u32 = 0;
        let mut vao: u32 = 0;

        let engine = unsafe { ENGINE_PTR.as_ref().unwrap() };
        let length_of_text = get_the_length_of_text(&text, &engine.font_face);

        let size = Some(Dimensions::new(
            length_of_text as i32,
            engine.font_face.font_size as i32,
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
                padding,
                text_length: length_of_text,
                text_vao: vao as i32,
                text_vbo: vbo as i32,
                background_vao,
                background_vbo,
                background_color: [0.4, 0.4, 0.4],
                cursor_hover: CursorState::Neither,
                text_shader_id: SHADER_TEXT_ID,
                on_hover: None,
                on_mouse_leave: None,
                on_click: None,
                on_right_click: None,
                on_middle_click: None,
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

        match self.cursor_hover {
            CursorState::Hover => {
                if let Some(func) = &mut self.on_hover {
                    func(view);
                }
            }

            CursorState::Leave => {
                if let Some(func) = &mut self.on_mouse_leave {
                    println!("Mouse leaving");
                    func(view);
                    self.cursor_hover = CursorState::Neither;
                }
            }

            _ => (),
        };

        let default_text_color: Vector3<f32> = Vector3::new(1.0, 1.0, 1.0);
        let color = match &self.color {
            Some(color) => color,
            None => &default_text_color,
        };

        unsafe {
            let size = self.size.unwrap();

            let text_position = (
                (self.position.x + self.padding) as f32,
                (self.position.y - engine.font_face.font_size as i32 - self.padding) as f32,
            );

            let quad_size = (
                (size.y + (self.padding << 1)) as f32,
                (size.x + (self.padding << 1)) as f32,
            );

            draw_quad_with_default_shader(
                engine,
                self.background_vao as u32,
                self.background_vbo as u32,
                -0.3,
                (self.position.x as f32, self.position.y as f32),
                quad_size,
                // &[0.2, 0.2, 0.2],
                &self.background_color,
            );
            draw_text(
                self.text_vao as u32,
                self.text_vbo as u32,
                &engine,
                self.text_shader_id,
                self.text.as_str(),
                text_position.0,
                text_position.1,
                1.0,
                color,
            );
        }

        Ok(())
    }

    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>) {
        //Note(teddy) Internall cordinate space is flipped with the screen cordinate space on the y-axis.
        //So the y position will be subbed from the padding.
        if does_cursor_intersect(
            cords,
            self.position,
            self.size.unwrap_or(ViewDimens::zerod()),
            self.padding,
        ) {
            self.cursor_hover = CursorState::Hover;
        } else {
            if self.cursor_hover != CursorState::Neither {
                self.cursor_hover = CursorState::Leave;
            }
        }
    }

    fn receive_cursor_cords(&mut self, engine: &Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }

    fn handle_button_click(
        &mut self,
        engine: &Engine,
        clicked_buttons: &Vec<MouseButton>,
        cords: Cords<f32>,
    ) -> bool {
        if does_cursor_intersect(
            &cords,
            self.position,
            self.size.unwrap_or(ViewDimens::zerod()),
            self.padding,
        ) {
            //Note(teddy) Left Click
            let self_ptr: *mut TextView = self;
            if let Some(_) = clicked_buttons
                .iter()
                .find(|b: &&MouseButton| **b == MouseButton::Button1)
            {
                if let Some(func) = &self.on_click {
                    func(self_ptr);
                }
            } else if let Some(_) = clicked_buttons
                .iter()
                .find(|b: &&MouseButton| **b == MouseButton::Button2)
            {
                //Right Click
                println!("Right click was clicked");
                if let Some(func) = &self.on_right_click {
                    func(self_ptr);
                }
            } else if let Some(_) = clicked_buttons
                .iter()
                .find(|b: &&MouseButton| **b == MouseButton::Button3)
            {
                //Middleclick
                println!("Middle click was clicked");
                if let Some(func) = &self.on_middle_click {
                    func(self_ptr);
                }
            }
        }

        true
    }
    fn get_view_dimensions(&self) -> Option<ViewDimens> {
        match self.size {
            Some(size) => Some(ViewDimens::new(
                size.x + (self.padding << 1),
                size.y + (self.padding << 1),
            )),

            None => None,
        }
    }

    fn set_position(&mut self, position: ViewPosition) {
        self.position = position;
    }

    fn get_position(&self) -> Option<ViewPosition> {
        Some(self.position)
    }
}

fn does_cursor_intersect(
    cords: &Cords<f32>,
    position: ViewDimens,
    size: ViewDimens,
    padding: i32,
) -> bool {
    let quad_size = (
        (size.x + (padding << 1)) as f32,
        (size.y + (padding << 1)) as f32,
    );

    let quad_position = (position.x as f32, position.y as f32 - quad_size.1 as f32);

    let min_x = quad_position.0;
    let min_y = quad_position.1;

    //FIXME(teddy): The length doesn't seem to match the actual screen length
    let max_x = min_x + quad_size.0;
    let max_y = min_y + quad_size.1;

    (cords.x > min_x as f32 && cords.x < max_x as f32)
        && (cords.y > min_y as f32 && cords.y < max_y as f32)
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

    let _ = world.resources.add_resource(AssetSource::Shader(
        font_shader!(),
        String::from("font_vert.glsl"),
        String::from("font_frag.glsl"),
        None,
    ));

    let _ = world.resources.add_resource(AssetSource::Shader(
        quad_shader!(),
        String::from("ui_quad_vert.glsl"),
        String::from("ui_quad_frag.glsl"),
        None,
    ));

    let shader_id = &world.resources.shaders[&font_shader!()];
    let quad_shader = &world.resources.shaders[&quad_shader!()];

    unsafe {
        SHADER_TEXT_ID = *shader_id;
        UI_QUAD_SHADER_ID = *quad_shader;
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
        if let Some(view) = &mut (&mut *engine).get_ui_tree().unwrap().root {
            view.receive_cursor_cords(&mut *engine, cords);
        }
    }
}

///Mouse click propagated and received by the ui will return false;
///Incase a ui element receives and process the event, it should return a false
pub fn propagate_button_click(
    engine: *mut Engine,
    button: &Vec<MouseButton>,
    cords: Cords<f32>,
) -> bool {
    let mut result = true;
    let eng_ref = unsafe { engine.as_mut().unwrap() };
    let ref_for_view = unsafe { engine.as_mut().unwrap() };

    if let Some(view) = &mut eng_ref.get_ui_tree().unwrap().root {
        result = view.handle_button_click(ref_for_view, button, cords);
    }

    result
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
    background_vao: i32,
    background_vbo: i32,
}

impl SimpleUIContainer {
    pub fn new(
        id: Box<str>,
        dimensions: Option<ViewDimens>,
        position: Option<ViewPosition>,
        orientation: Orientation,
    ) -> Self {
        let background_buffers = unsafe { initialize_background_buffers() };

        Self {
            id,
            children: vec![],
            dimensions,
            position,
            orientation,
            background_vao: background_buffers.0,
            background_vbo: background_buffers.1,
        }
    }

    fn recalculate_dimensions(&mut self) {
        let mut new_dimensions = ViewDimens::zerod();

        //Note(teddy) Updating the length and height based on the orientation of the container
        match self.orientation {
            Orientation::Vertical => {
                let mut height: i32 = 0;
                let mut view_dimens = ViewDimens::zerod();

                for child in self.children.iter() {
                    view_dimens = child.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    new_dimensions.x = std::cmp::max(view_dimens.x, new_dimensions.x);
                    height += view_dimens.y;
                }

                new_dimensions.y = height;
            }

            Orientation::Horizontal => {
                let mut width: i32 = 0;
                let mut view_dimens = ViewDimens::zerod();

                for child in self.children.iter() {
                    view_dimens = child.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    new_dimensions.y = std::cmp::max(view_dimens.y, new_dimensions.y);
                    width += view_dimens.x;
                }

                new_dimensions.x = width;
            }
        }

        self.dimensions = Some(new_dimensions);

        //Flip the y for this quad
        if let Some(position) = self.position {
            self.position = Some(ViewDimens::new(position.x, position.y));
        }
    }
}

impl View for SimpleUIContainer {
    fn get_id(&self) -> &str {
        &(self.id)
    }

    fn handle_button_click(
        &mut self,
        engine: &Engine,
        button: &Vec<MouseButton>,
        cords: Cords<f32>,
    ) -> bool {
        let container_position = match self.position {
            Some(position) => ViewDimens::new(position.x, position.y + self.dimensions.unwrap().y),

            None => ViewDimens::zerod(),
        };

        if does_cursor_intersect(
            &cords,
            //self.position.unwrap_or(ViewDimens::zerod()),
            container_position,
            self.dimensions.unwrap_or(ViewDimens::zerod()),
            0,
        ) {
            for view in &mut self.children {
                view.handle_button_click(engine, button, cords);
            }
        }

        true
    }

    fn update(&mut self, engine: &Engine) -> UIResult {
        let quad_size = (
            (self.dimensions.unwrap_or(ViewDimens::new(10, 10)).y) as f32,
            (self.dimensions.unwrap_or(ViewDimens::new(10, 10)).x) as f32,
        );

        let quad_position = (
            self.position.unwrap().x as f32,
            self.position.unwrap().y as f32 + quad_size.0,
        );

        unsafe {
            draw_quad_with_default_shader(
                engine,
                self.background_vao as u32,
                self.background_vbo as u32,
                -0.4,
                quad_position,
                quad_size,
                // &[0.2, 0.2, 0.2],
                &[0.6, 0.3, 0.3],
            );
        }

        //TODO(teddy) This initial position will be the position of the container
        //TODO(teddy) optimize this to prevent recalculations
        match self.orientation {
            Orientation::Vertical => {
                let mut initial_y_position = self.position.unwrap().y;

                for view in self.children.iter_mut() {
                    let view_dimensions = view.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    view.set_position(ViewPosition::new(
                        self.position.unwrap().x,
                        initial_y_position + view_dimensions.y,
                    ));

                    initial_y_position += view_dimensions.y;
                    view.update(engine).unwrap();
                }
            }

            Orientation::Horizontal => {
                let mut intial_x_position = self.position.unwrap().x;

                for view in self.children.iter_mut() {
                    let view_dimensions = view.get_view_dimensions().unwrap_or(ViewDimens::zerod());

                    view.set_position(ViewPosition::new(
                        intial_x_position,
                        self.position.unwrap().y + view_dimensions.y,
                    ));
                    intial_x_position += view_dimensions.x;
                    view.update(engine).unwrap();
                }
            }
        }

        //Draw items in a column
        // panic!("Stopped the process");
        Ok(())
    }

    fn compute_intersect_with_cursor_cords(&mut self, _engine: &Engine, cords: &Cords<f32>) {
        //TODO(teddy) implement a simple ui container
    }

    fn receive_cursor_cords(&mut self, engine: &Engine, cords: Cords<f32>) {
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
