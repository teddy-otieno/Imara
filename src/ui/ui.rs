use std::ffi::c_void;
use std::ptr::null;

use glfw::MouseButton;
use nalgebra::Vector3;
use nphysics3d::utils::UserData;

use crate::core::{Engine, FontFace};
use crate::game_world::world::AssetSource;
use crate::game_world::world::World;
use crate::renderer::draw::{draw_quad_with_default_shader, draw_text};
use crate::utils::{get_at_index, Cords};

static mut SHADER_TEXT_ID: u32 = 0;
pub static mut UI_QUAD_SHADER_ID: u32 = 0;
static mut ENGINE_PTR: *const Engine = null();

macro_rules! font_shader {
    () => {
        String::from("font_shader")
    };
}

macro_rules! quad_shader {
    () => {
        String::from("quad_shader")
    };
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
    fn compute_intersect_with_cursor_cords(&mut self, engine: &Engine, cords: &Cords<f32>) {

        let id = String::from(self.get_id());
        let view_object = self.get_view_object_mut();
        if does_cursor_intersect(
            cords,
            view_object.position,
            view_object.size.unwrap_or(ViewDimens::zerod()),
            view_object.padding,
        ) {
            println!("{}", id);
            view_object.cursor_hover_state = CursorState::Hover;
        } else {
            if view_object.cursor_hover_state != CursorState::Neither {
                view_object.cursor_hover_state = CursorState::Leave;
            }
        }
    }

    fn receive_cursor_cords(&mut self, engine: &Engine, cords: Cords<f32>) {
        self.compute_intersect_with_cursor_cords(&engine, &cords);
    }

    fn get_view_object(&self) -> &ViewObject;
    fn get_view_object_mut(&mut self) -> &mut ViewObject;

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
    ///Keystrokes will be sent this view
    pub focused_view: Option<Box<dyn View>>,
    pub root: Option<Box<dyn View>>,
}

impl UITree {
    pub fn new() -> Self {
        UITree {
            root: None,
            focused_view: None,
        }
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


pub struct ViewObject {
    cursor_hover_state: CursorState,
    pub id: Box<str>,
    pub background_vao: i32,
    pub background_vbo: i32,
    pub size: Option<ViewDimens>,
    pub background_color: Box<[f32; 3]>,
    pub padding: i32,
    pub scale: f32,
    pub position: ViewDimens,
    pub z_index: Option<u32>,
}

impl ViewObject {

    fn new(
        id: Box<str>, 
        position: ViewDimens, 
        size: Option<ViewDimens>, 
        padding: i32, 
        scale: f32,
        background_color: Box<[f32; 3]>,
        z_index: Option<u32>
    ) -> Self {

        unsafe {
            let (background_vao, background_vbo) = initialize_background_buffers();

            Self {
                id,
                size,
                padding,
                scale,
                position,
                background_vao,
                background_vbo,
                background_color,
                cursor_hover_state: CursorState::Neither,
                z_index
            }
        }
    }
}

pub struct TextView {
    text_vao: i32,
    text_vbo: i32,
    text_shader_id: u32,
    text_length: u32,
    text_height: u32,
    cursor_hover: CursorState,
    view: ViewObject,

    pub text: String,
    pub color: Option<Vector3<f32>>,
    //Note(teddy) Incase the size is not passed, use the fonts width and heights and update this value

    pub on_hover: Option<Box<dyn FnMut(*mut Self)>>,
    pub on_mouse_leave: Option<Box<dyn FnMut(*mut Self)>>,
    pub on_click: Option<Box<dyn Fn(*mut Self)>>,
    pub on_right_click: Option<Box<dyn Fn(*mut Self)>>,
    pub on_middle_click: Option<Box<dyn Fn(*mut Self)>>,
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
                view: ViewObject::new(id, position, size, padding, scale, Box::new([0.4, 0.4, 0.4]), None),
                text,
                text_height: engine.font_face.font_size as u32,
                text_length: length_of_text,
                text_vao: vao as i32,
                text_vbo: vbo as i32,
                cursor_hover: CursorState::Neither,
                text_shader_id: SHADER_TEXT_ID,
                color: None,

                on_hover: None,
                on_mouse_leave: None,
                on_click: None,
                on_right_click: None,
                on_middle_click: None,
            }
        }
    }
}

impl View for TextView {
    fn get_id(&self) -> &str {
        &(*self.view.id)
    }

    fn get_view_object(&self) -> &ViewObject { &self.view }
    fn get_view_object_mut(&mut self) -> &mut ViewObject { &mut self.view }

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
            let size = self.view.size.unwrap();

            let text_position = (
                (self.view.position.x + self.view.padding) as f32,
                (self.view.position.y - engine.font_face.font_size as i32 - self.view.padding) as f32,
            );

            let quad_size = (
                (size.y + (self.view.padding << 1)) as f32,
                (size.x + (self.view.padding << 1)) as f32,
            );

            draw_quad_with_default_shader(
                engine,
                self.view.background_vao as u32,
                self.view.background_vbo as u32,
                -0.3,
                (self.view.position.x as f32, self.view.position.y as f32),
                quad_size,
                // &[0.2, 0.2, 0.2],
                &self.view.background_color,
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
            self.view.position,
            self.view.size.unwrap_or(ViewDimens::zerod()),
            self.view.padding,
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
        match self.view.size {
            Some(size) => Some(ViewDimens::new(
                size.x + (self.view.padding << 1),
                size.y + (self.view.padding << 1),
            )),

            None => None,
        }
    }

    fn set_position(&mut self, position: ViewPosition) {
        self.view.position = position;
    }

    fn get_position(&self) -> Option<ViewPosition> {
        Some(self.view.position)
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

    let _ = world.resources.add_resource(
        AssetSource::Shader(
            font_shader!(),
            String::from("font_vert.glsl"),
            String::from("font_frag.glsl"),
            None,
        ),
        false,
    );

    let _ = world.resources.add_resource(
        AssetSource::Shader(
            quad_shader!(),
            String::from("ui_quad_vert.glsl"),
            String::from("ui_quad_frag.glsl"),
            None,
        ),
        false,
    );

    let shader_container_ref = world.resources.shaders.read().unwrap();

    let shader_id = &shader_container_ref[&font_shader!()].unwrap();
    let quad_shader = &shader_container_ref[&quad_shader!()].unwrap();

    unsafe {
        SHADER_TEXT_ID = *shader_id;
        UI_QUAD_SHADER_ID = *quad_shader;
    };
    Ok(())
}

pub fn propagate_cursor_pos_to_ui(engine: *mut Engine, cords: Cords<f32>) {
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

    // result
    false
}

pub fn propagate_key_stroke(engine: *mut Engine, key: glfw::Key) -> bool {
    unimplemented!()
}

pub enum Orientation {
    Vertical,
    Horizontal,
}

pub struct SimpleUIContainer {
    children: Vec<Box<dyn View>>,
    orientation: Orientation,
    view: ViewObject,
}

impl SimpleUIContainer {
    pub fn new(
        id: Box<str>,
        dimensions: Option<ViewDimens>,
        position: ViewPosition,
        orientation: Orientation,
        padding: i32,
        scale: f32
    ) -> Self {
        Self {
            view: ViewObject::new(id, position, dimensions, padding, scale, Box::new([1.0, 1.0, 1.0]), None),
            children: vec![],
            orientation,
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

        self.view.size = Some(new_dimensions);

        // //Flip the y for this quad
        // if let Some(position) = self.view.position {
        //     self.view.position = Some(ViewDimens::new(position.x, position.y));
        // }
    }
}

impl View for SimpleUIContainer {

    fn get_id(&self) -> &str {
        &(self.view.id)
    }

    fn handle_button_click(
        &mut self,
        engine: &Engine,
        button: &Vec<MouseButton>,
        cords: Cords<f32>,
    ) -> bool {
        let container_position =self.view.position;

        if does_cursor_intersect(
            &cords,
            //self.position.unwrap_or(ViewDimens::zerod()),
            container_position,
            self.view.size.unwrap_or(ViewDimens::zerod()),
            0,
        ) {
            for view in &mut self.children {
                view.handle_button_click(engine, button, cords);
            }
        }

        true
    }

    fn update(&mut self, engine: &Engine) -> UIResult {

        let quad_size  = self.view.size;
        let default_dimensions = ViewDimens::new(10, 10);

        let quad_size = (
            (quad_size.unwrap_or(default_dimensions).y) as f32,
            (quad_size.unwrap_or(default_dimensions).x) as f32,
        );

        let quad_position = (
            self.view.position.x as f32,
            self.view.position.y as f32 + quad_size.0,
        );


        if true {

        unsafe {
            draw_quad_with_default_shader(
                engine,
                self.view.background_vao as u32,
                self.view.background_vbo as u32,
                0.0,
                quad_position,
                quad_size,
                // &[0.2, 0.2, 0.2],
                &[0.6, 0.3, 0.3],
            );
        }
        }

        //TODO(teddy) This initial position will be the position of the container
        //TODO(teddy) optimize this to prevent recalculations
        match self.orientation {
            Orientation::Vertical => {
                let mut initial_y_position = self.view.position.y;

                for view in self.children.iter_mut() {
                    let view_dimensions = view.get_view_dimensions().unwrap_or(ViewDimens::zerod());
                    view.set_position(ViewPosition::new(
                        self.view.position.x,
                        initial_y_position + view_dimensions.y,
                    ));

                    initial_y_position += view_dimensions.y;
                    view.update(engine).unwrap();
                }
            }

            Orientation::Horizontal => {
                let mut intial_x_position = self.view.position.x;

                for view in self.children.iter_mut() {
                    let view_dimensions = view.get_view_dimensions().unwrap_or(ViewDimens::zerod());

                    view.set_position(ViewPosition::new(
                        intial_x_position,
                        self.view.position.y + view_dimensions.y,
                    ));
                    intial_x_position += view_dimensions.x;
                    view.update(engine).unwrap();
                }
            }
        }

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

    fn get_view_object(&self) -> &ViewObject { &self.view }

    fn get_view_object_mut(&mut self) -> &mut ViewObject { &mut self.view }
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
