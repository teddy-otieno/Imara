use std::collections::HashMap;
use std::ffi::{c_void, CString};

use freetype::freetype;
use glfw::Key;
use glfw::{Action, FlushedMessages, WindowEvent};
use nalgebra::{Matrix, Matrix4, Point2, Point3, Vector3, Vector4};
use ncollide3d::query::Ray;

use crate::game_world::components::HighlightComponent;
use crate::game_world::world::{EntityID, World, FONT_ASSETS_DIR};
use crate::gl_bindings::Display;
use crate::ui::ui::{propagate_button_click, propagate_cursor_pos_to_ui, UITree, View};
use crate::utils::Cords;

#[derive(Debug)]
pub enum Event {
    EntityCreated(EntityID),
    EntityRemoved(EntityID),
    CastRay(CastRayDat),
    RayCasted(CastedRay),
}

#[derive(Debug)]
pub struct CastRayDat {
    pub id: usize,
    pub ray: Ray<f32>,
}

#[derive(Debug)]
pub struct CastedRay {
    pub id: usize,
    pub entity: Option<EntityID>,
}

#[repr(C)]
#[derive(Debug)]
pub struct Light {
    pub color: [f32; 3],
    pub direction: [f32; 3],
}

pub struct Engine {
    pub display: Display,
    pub camera: Camera,
    pub dir_lights: Light,
    pub pressed_keys: Vec<Key>,
    pub select_mode: bool,
    pub font_face: FontFace,
    view_toggle: bool,
    cursor_mode_toggle: bool,

    pub ui_view: Vec<Box<dyn View>>,
    pub ui_tree: UITree,
    pub ui_frame_buffer: Option<u32>,
}

//TODO(teddy) have an init routine
impl Engine {
    pub fn new(display: Display, font_face: FontFace) -> Self {
        Self {
            display,
            camera: Camera::new(),
            view_toggle: true,
            pressed_keys: vec![],
            dir_lights: Light {
                color: [1.0, 1.0, 1.0],
                direction: [10.0, 30.0, 0.0],
            },
            select_mode: false,
            cursor_mode_toggle: false,
            font_face,
            ui_view: vec![],
            ui_frame_buffer: None,
            ui_tree: UITree::new(),
        }
    }

    pub fn update(&mut self, world: &mut World, event_manager: &mut EventManager) {
        let eve_ptr: *mut EventManager = event_manager;

        for event in event_manager.window_events.iter() {
            match event {
                WindowEvent::Size(width, height) => {
                    self.camera.view_port = (*width, *height);
                    unsafe { gl::Viewport(0, 0, *width, *height) }
                }

                WindowEvent::CursorPos(x, y) => {
                    if !self.cursor_mode_toggle {
                        self.camera.update_look(*x, *y);
                    }

                    let cords = Cords {
                        x: *x as f32,
                        y: *y as f32,
                    };

                    self.camera.new_cords = cords;
                    propagate_cursor_pos_to_ui(self, cords)
                }

                WindowEvent::MouseButton(button, _action, _modifiers) => {
                    //Note(teddy) This event was not handled in UI meaning button click wasn't in a ui element
                    if !propagate_button_click(self, button, self.camera.new_cords) {
                        let direction = compute_ray_from_mouse_cords(
                            (self.camera.new_cords.x, self.camera.new_cords.y),
                            self.camera.view_port,
                            self.camera.perspective(),
                            self.camera.view(),
                        );

                        dbg!(&direction);
                        dbg!(&self.camera.camera_front);
                        let ray = Ray::new(Point3::from(self.camera.position), direction);

                        unsafe {
                            (*eve_ptr).add_engine_event(Event::CastRay(CastRayDat { id: 0, ray }));
                        }
                    }
                }

                WindowEvent::Key(key, _, action, _modifier) => {
                    if self.pressed_keys.contains(key) && *action == Action::Release {
                        self.pressed_keys.retain(|s| s != key);
                    } else if *action == Action::Press {
                        self.pressed_keys.push(*key);
                    }
                }

                _ => (),
            }
        }

        self.handle_world_events(world, event_manager);
    }

    fn handle_world_events(&mut self, world: &mut World, event_manager: &EventManager) {
        for event in event_manager.get_engine_events() {
            match event {
                Event::RayCasted(CastedRay { id: _, entity }) if entity.is_some() => {
                    world.components.highlightable[entity.unwrap()] = Some(HighlightComponent {
                        color: [0.0, 1.0, 0.0],
                    });
                }
                _ => (),
            }
        }
    }
}

//Handle user defined events
#[derive(Debug)]
pub struct EventManager {
    pub window_events: Vec<WindowEvent>,

    which_buff: bool,
    engine_events: Vec<Event>,
    engine_events1: Vec<Event>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            window_events: vec![],
            engine_events: vec![],
            engine_events1: vec![],
            which_buff: true,
        }
    }

    pub fn handle_events(&mut self, events: FlushedMessages<(f64, WindowEvent)>) {
        self.window_events = events.into_iter().map(|(_, event)| event).collect();
    }

    pub fn add_event(&mut self, event: Event) {
        //Note(teddy) add new events to the next buffer

        if self.which_buff {
            self.engine_events.push(event);
        } else {
            self.engine_events1.push(event);
        }
    }

    pub fn add_engine_event(&mut self, event: Event) {
        //Note(teddy) Events dispatched by systesm will be added to the next event buffer
        if !self.which_buff {
            self.engine_events.push(event);
        } else {
            self.engine_events1.push(event);
        }
    }

    pub fn get_engine_events(&self) -> &Vec<Event> {
        if self.which_buff {
            &self.engine_events
        } else {
            &self.engine_events1
        }
    }

    pub fn clear(&mut self) {
        self.window_events.clear();

        if self.which_buff {
            self.engine_events.clear();
        } else {
            self.engine_events1.clear();
        }

        self.which_buff = !self.which_buff;
    }
}

enum CameraMovement {
    Up,
    Down,
    Left,
    Right,
}

pub struct Camera {
    pub position: Vector3<f32>,
    pub previous_cords: (f32, f32),
    pub new_cords: Cords<f32>,
    pub camera_front: Vector3<f32>,
    pub first_move: bool,
    pub fov: f32,
    camera_up: Vector3<f32>,
    yaw: f32,
    pitch: f32,
    pub view_port: (i32, i32),
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vector3::new(10.0, 700.0, 20.0),
            camera_front: Vector3::new(0.0, 0.0, -1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            first_move: true,
            // fov: 0.785398 std::f64::consts::FRAC_PI_4,
            fov: std::f32::consts::FRAC_PI_4,
            yaw: -90.0,
            pitch: 0.0,
            previous_cords: (0.0, 0.0),
            new_cords: Cords { x: 0.0, y: 0.0 },
            view_port: (1000, 600),
        }
    }

    pub fn perspective(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(
            self.view_port.0 as f32 / self.view_port.1 as f32,
            self.fov,
            0.1,
            100000.0,
        )
    }

    pub fn view(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(
            &Point3::from(self.position),
            &Point3::from(self.position + self.camera_front),
            &self.camera_up,
        )
    }

    fn update_look(&mut self, x: f64, y: f64) {
        if self.first_move {
            self.previous_cords = (x as f32, y as f32);
            self.first_move = false;
        }

        let mut offset = (
            x as f32 - self.previous_cords.0,
            y as f32 - self.previous_cords.1,
        );
        self.previous_cords.0 = x as f32;
        self.previous_cords.1 = y as f32;

        let sensitivity = 0.5;
        offset = (offset.0 * sensitivity, offset.1 * sensitivity);

        self.yaw += offset.0;
        self.pitch += offset.1;

        if self.pitch > 89.0 {
            self.pitch = 89.0;
        }

        if self.pitch < -89.0 {
            self.pitch = -89.0
        }

        let x_dir = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        let y_dir = self.pitch.to_radians().sin();
        let z_dir = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();

        self.camera_front = Vector3::new(x_dir, y_dir, z_dir).normalize();
    }

    fn update_position(&mut self, motion: CameraMovement) {
        let camera_speed = 2.5;

        match motion {
            CameraMovement::Up => {
                self.position += camera_speed * self.camera_front;
            }

            CameraMovement::Down => {
                self.position -= camera_speed * self.camera_front;
            }

            CameraMovement::Left => {
                self.position -=
                    camera_speed * self.camera_front.cross(&self.camera_up).normalize();
            }

            CameraMovement::Right => {
                self.position +=
                    camera_speed * self.camera_front.cross(&self.camera_up).normalize();
            }
        }
    }
}

fn compute_ray_from_mouse_cords(
    cords: (f32, f32),
    screen_cords: (i32, i32),
    projection_matrix: Matrix4<f32>,
    view_matrix: Matrix4<f32>,
) -> Vector3<f32> {
    //Normalize the device cordinates
    let x = (2.0 * cords.0) / screen_cords.0 as f32 - 1.0;
    let y = 1.0 - (2.0 * cords.1) / screen_cords.1 as f32;

    let ray_normalized_devices_cords: Vector4<f32> = Vector4::new(x, y, 1.0, 1.0);

    //FIXME(teddy) Inverse computation should be handled incase it fails
    let map_to_camera_space: Matrix4<f32> =
        (projection_matrix * view_matrix).try_inverse().unwrap();

    let mut mapped_direction: Vector4<f32> = map_to_camera_space * ray_normalized_devices_cords;
    mapped_direction /= mapped_direction.w;
    mapped_direction.xyz().normalize()
}

macro_rules! contains_key {
    ($engine:expr, $key:expr) => {
        $engine.pressed_keys.contains(&$key)
    };
}

static mut L_CLICKED: bool = false;
static mut M_CLICKED: bool = false;

pub fn camera_behaviour(engine: &mut Engine) {
    if contains_key!(engine, Key::W) {
        engine.camera.update_position(CameraMovement::Up);
    }

    if contains_key!(engine, Key::S) {
        engine.camera.update_position(CameraMovement::Down);
    }

    if contains_key!(engine, Key::A) {
        engine.camera.update_position(CameraMovement::Left);
    }

    if contains_key!(engine, Key::D) {
        engine.camera.update_position(CameraMovement::Right);
    }

    unsafe {
        if contains_key!(engine, Key::M) {
            if !M_CLICKED {
                engine.cursor_mode_toggle = !engine.cursor_mode_toggle;

                if engine.cursor_mode_toggle {
                    engine
                        .display
                        .window
                        .set_cursor_mode(glfw::CursorMode::Normal);
                } else {
                    engine
                        .display
                        .window
                        .set_cursor_mode(glfw::CursorMode::Hidden);
                    engine.camera.first_move = true;
                }
                M_CLICKED = true;
            }
        } else {
            M_CLICKED = false;
        }

        if contains_key!(engine, Key::L) {
            if !L_CLICKED {
                engine.view_toggle = !engine.view_toggle;

                if engine.view_toggle {
                    gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
                } else {
                    gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
                }

                L_CLICKED = true;
            }
        } else {
            L_CLICKED = false;
        }
    }
    if contains_key!(engine, Key::Escape) {
        std::process::exit(0);
    }
}

#[derive(Debug)]
pub struct FontFace {
    font_name: String,  //TODO(teddy) Get the name of the font from the ttf files
    pub font_size: u32, //Similar to the font-size
    pub chars: HashMap<char, FontChar>,
}

#[derive(Debug)]
pub enum FontError {
    FailedToLoadFontLib,
    UnableToLoadFont,
    FailedToLoadGlyph,
}

#[derive(Debug)]
pub struct FontChar {
    pub texture: u32,
    pub size: Point2<i32>,
    pub bearing: Point2<i32>,
    pub advance: i32,
}

//TODO(teddy) Return the font-face loaded
//Reuse the font-face incase the ui will require different font sizes

//Note(teddy) Caller can generate fonts for different sizes depending on their needs
//The unnecessary fonts should be freed accordingly
pub unsafe fn load_fonts(font_size: u32) -> Result<FontFace, FontError> {
    let mut ft_lib: freetype::FT_Library = std::ptr::null_mut();
    if freetype::FT_Init_FreeType(&mut ft_lib) != 0 {
        return Err(FontError::FailedToLoadFontLib);
    }

    let font_path =
        CString::new(format!("{}{}", FONT_ASSETS_DIR, "Roboto-Regular.ttf").as_str()).unwrap();
    let mut font_face: freetype::FT_Face = std::ptr::null_mut();
    if freetype::FT_New_Face(ft_lib, font_path.as_ptr(), 0, &mut font_face) != 0 {
        return Err(FontError::UnableToLoadFont);
    }

    freetype::FT_Set_Pixel_Sizes(font_face, 0, font_size);

    let mut characters = HashMap::new();

    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1); //Note(teddY) Disable byte-alignment restriction
    for c in 0..128 {
        if freetype::FT_Load_Char(font_face, c, freetype::FT_LOAD_RENDER as i32) != 0 {
            return Err(FontError::FailedToLoadGlyph);
        }

        let width = (*(&*font_face).glyph).bitmap.width as i32;
        let height = (*(&*font_face).glyph).bitmap.rows as i32;

        let mut texture: u32 = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RED as i32,
            width,
            height,
            0,
            gl::RED,
            gl::UNSIGNED_BYTE,
            (*(*font_face).glyph).bitmap.buffer as *const c_void,
        );

        //Set the texture paramaters
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        let character = FontChar {
            texture,
            size: Point2::new(width, height),
            bearing: Point2::new(
                (*(*font_face).glyph).bitmap_left,
                (*(*font_face).glyph).bitmap_top,
            ),
            advance: (*(*font_face).glyph).advance.x as i32,
        };

        characters.insert(c as u8 as char, character);
    }

    freetype::FT_Done_Face(font_face);
    freetype::FT_Done_FreeType(ft_lib);

    Ok(FontFace {
        font_name: String::from(""),
        font_size,
        chars: characters,
    })
}
