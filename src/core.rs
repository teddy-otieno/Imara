use std::collections::{HashMap, HashSet};
use std::ffi::{c_void, CString};
use std::hash::{Hash, Hasher};

use freetype::freetype;
use glfw::{Action, FlushedMessages, Key, MouseButton, WindowEvent};
use nalgebra::{Matrix4, Point2, Point3, Vector3, Vector4};
use ncollide3d::query::Ray;

use crate::game_world::world::{EntityID, World, FONT_ASSETS_DIR};
use crate::gl_bindings::Display;
use crate::systems::system::SystemType;
use crate::ui::ui::{propagate_button_click, propagate_cursor_pos_to_ui, UITree, View};
use crate::utils::Cords;

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    EntityCreated(EntityID),
    EntityRemoved(EntityID),
    CastRay(CastRayDat),
    RayCasted(CastedRay),
}

///Some events will be locked to routine running in a seperate thread like loading assets.
///This wrapper struct will be used to mark events that are pending executions so that systems
///will be aware of their presence as they update the game states

static mut EVENT_IDS: u64 = 0;
#[derive(Debug, Clone)]
pub struct Event {
    pub id: u64,
    pub event_type: EventType,
    pending_systems: Vec<SystemType>,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Event {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Event {}

impl Event {
    pub fn new(event: EventType) -> Self {
        Self {
            id: unsafe {
                let temp = EVENT_IDS;
                EVENT_IDS += 1;
                temp
            },
            event_type: event,
            pending_systems: vec![],
        }
    }

    pub fn register_pending_system(&mut self, register_system: SystemType) {
        if self.pending_systems.contains(&register_system) {
            return;
        }

        self.pending_systems.push(register_system);
    }

    pub fn remove_pending_system(&mut self, remove_system: SystemType) {
        self.pending_systems.retain(|x| *x != remove_system);
    }

    pub fn get_pending_system(&self) -> &Vec<SystemType> {
        &self.pending_systems
    }

    pub fn is_pending_for(&self, system: SystemType) -> bool {
        self.pending_systems.contains(&system)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CastRayDat {
    pub id: usize,
    pub ray: Ray<f32>,
}

#[derive(Debug, Clone, Copy)]
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

#[inline(always)]
pub fn mouse_clicked(engine: &Engine, button: &MouseButton) -> bool {
    let result = engine.mouse_button_keys.iter().find(|b| *button == **b);

    match result {
        Some(_) => true,
        None => false,
    }
}

pub struct Engine {
    pub display: Display,
    pub camera: Camera,
    pub dir_lights: Light,
    pub pressed_keys: Vec<Key>,
    pub mouse_button_keys: Vec<MouseButton>,
    pub select_mode: bool,
    pub font_face: FontFace,
    view_toggle: bool,
    cursor_mode_toggle: bool,

    pub ui_view: Vec<Box<dyn View>>,
    pub ui_tree: Option<*mut UITree>,
    pub ui_frame_buffer: Option<u32>,
}

#[inline(always)]
fn check_button(button: &MouseButton, action: &Action, buttons: &mut Vec<MouseButton>) {
    match action {
        Action::Press => {
            if let None = buttons.iter().find(|b| **b == *button) {
                buttons.push(*button);
            }
        }

        Action::Release => {
            if let Some(button) = buttons.clone().into_iter().find(|b| b == button) {
                buttons.retain(|b| *b != button);
            }
        }

        _ => (),
    }
}

//TODO(teddy) have an init routine
impl Engine {
    pub fn new(display: Display, font_face: FontFace) -> Self {
        Self {
            display,
            camera: Camera::new(),
            view_toggle: true,
            pressed_keys: vec![],
            mouse_button_keys: vec![],
            dir_lights: Light {
                color: [1.0, 1.0, 1.0],
                direction: [10.0, 30.0, 0.0],
            },
            select_mode: false,
            cursor_mode_toggle: true,
            font_face,
            ui_view: vec![],
            ui_frame_buffer: None,
            ui_tree: None,
        }
    }

    pub fn get_ui_tree(&mut self) -> Option<&mut UITree> {
        unsafe { self.ui_tree.as_ref().unwrap().as_mut() }
    }

    pub fn update(&mut self, event_manager: &mut EventManager) {
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
                    } else {
                        let cords = Cords {
                            x: *x as f32,
                            y: *y as f32,
                        };

                        self.camera.new_cords = cords;
                        propagate_cursor_pos_to_ui(self, cords)
                    }
                }

                WindowEvent::MouseButton(button, action, _modifiers) => {
                    //Note(teddy) This event was not handled in UI meaning button click wasn't in a ui element

                    match button {
                        MouseButton::Button1 => {
                            check_button(button, action, &mut self.mouse_button_keys);
                        }

                        MouseButton::Button2 => {
                            check_button(button, action, &mut self.mouse_button_keys);
                        }

                        MouseButton::Button3 => {
                            check_button(button, action, &mut self.mouse_button_keys);
                        }

                        //TODO(teddy) will make the other buttons remappable for the next projects
                        _ => (),
                    }

                    //TODO(teddy) Move the ui to its own system
                    if !propagate_button_click(self, &self.mouse_button_keys, self.camera.new_cords)
                    {
                        let direction = compute_ray_from_mouse_cords(
                            (self.camera.new_cords.x, self.camera.new_cords.y),
                            self.camera.view_port,
                            self.camera.perspective(),
                            self.camera.view(),
                        );

                        dbg!(&direction);
                        dbg!(&self.camera.camera_front);
                        let ray = Ray::new(Point3::from(self.camera.position), direction);

                        let ray_cast_event =
                            Event::new(EventType::CastRay(CastRayDat { id: 0, ray }));

                        dbg!(&ray_cast_event);
                        unsafe {
                            (*eve_ptr).add_engine_event(ray_cast_event);
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
    }
}

//Handle user defined events
#[derive(Debug)]
pub struct EventManager {
    pub window_events: Vec<WindowEvent>,

    which_buff: bool,
    engine_events: Vec<Event>,
    engine_events1: Vec<Event>,

    pending_events: Vec<Event>,
    pending_events_for_the_next_cycle: Vec<Event>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            window_events: vec![],
            engine_events: vec![],
            engine_events1: vec![],
            pending_events: vec![],
            pending_events_for_the_next_cycle: vec![],
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

    pub fn get_engine_events(&mut self) -> Vec<Event> {
        //Note(teddy) Add pending events

        if self.which_buff {
            let mut temp = self.engine_events.clone();
            let event_array = self
                .pending_events
                .iter()
                .map(|x| x.clone())
                .collect::<Vec<Event>>();
            temp.extend_from_slice(event_array.as_slice());
            temp
        } else {
            let mut temp = self.engine_events1.clone();
            let event_array = self
                .pending_events
                .iter()
                .map(|x| x.clone())
                .collect::<Vec<Event>>();

            temp.extend_from_slice(event_array.as_slice());
            temp
        }
    }

    //Note(teddy)
    //Issue will arising when the lock is held for too long.
    //We can timestamp the events and cancel events that have lived for a period of time
    //to ensure program correctness
    pub fn add_pending(&mut self, mut event: Event, system_type: SystemType) {
        if let Some(existing) = self
            .pending_events
            .iter_mut()
            .find(|x| (**x).id == event.id)
        {
            existing.register_pending_system(system_type);
            return;
        }
        event.register_pending_system(system_type);

        //Note(teddy) Preventing pending events added from the current update cycle to be processed by the next system[s]
        self.pending_events_for_the_next_cycle.push(event);
    }

    pub fn remove_pending(&mut self, event_id: u64, system_type: SystemType) {
        let (id, is_pending_systems_empty) = {
            let event = self
                .pending_events
                .iter_mut()
                .find(|x| (**x).id == event_id)
                .unwrap();

            if event.get_pending_system().contains(&system_type) {
                event.remove_pending_system(system_type);
            }

            (event.id, event.get_pending_system().is_empty())
        };

        if is_pending_systems_empty {
            self.pending_events.retain(|x| (*x).id != id);
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

        //Bind pending events for the next cycle to pending events loop

        self.pending_events
            .extend_from_slice(self.pending_events_for_the_next_cycle.as_slice());
        self.pending_events.dedup();
        self.pending_events_for_the_next_cycle.clear();
    }
}

enum CameraMovement {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub struct ViewPortDimensions {
    pub width: i32,
    pub height: i32,
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
            position: Vector3::new(-1.0, 0.0, 0.0),
            camera_front: Vector3::new(1.0, 0.0, 0.0),
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
        Matrix4::look_at_lh(
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

    fn update_position(&mut self, motion: CameraMovement, speed: Option<f32>) {
        let camera_speed = 2.5;

        match motion {
            CameraMovement::Up => {
                self.position -= speed.unwrap_or(camera_speed) * self.camera_front;
            }

            CameraMovement::Down => {
                self.position += speed.unwrap_or(camera_speed) * self.camera_front;
            }

            CameraMovement::Left => {
                self.position += speed.unwrap_or(camera_speed)
                    * self.camera_front.cross(&self.camera_up).normalize();
            }

            CameraMovement::Right => {
                self.position -= speed.unwrap_or(camera_speed)
                    * self.camera_front.cross(&self.camera_up).normalize();
            }
        }
    }
}

#[inline]
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
        engine
            .camera
            .update_position(CameraMovement::Up, Some(0.05));
    }

    if contains_key!(engine, Key::S) {
        engine
            .camera
            .update_position(CameraMovement::Down, Some(0.05));
    }

    if contains_key!(engine, Key::A) {
        engine
            .camera
            .update_position(CameraMovement::Left, Some(0.05));
    }

    if contains_key!(engine, Key::D) {
        engine
            .camera
            .update_position(CameraMovement::Right, Some(0.05));
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
                        .set_cursor_mode(glfw::CursorMode::Disabled);
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
