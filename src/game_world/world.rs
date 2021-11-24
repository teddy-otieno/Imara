use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use std::thread;
use std::io::Write;
use std::{
    collections::{HashMap, LinkedList},
    ops::{Deref, DerefMut},
};

use nalgebra::Vector3;
use nphysics3d::material::{BasicMaterial, MaterialHandle};
use nphysics3d::object::BodyStatus;
use serde::{Deserialize, Serialize};

use super::components::*;
use crate::core::{Engine, Event, EventManager, EventType};
use crate::obj_parser::{load_obj, NormalObj, TexturedObj};
use crate::renderer::shaders::create_shader;
use crate::logs::LogManager;
use crate::logs::Logable;

const WORLD_LEVELS_DIR: &'static str = "./assets/levels/";
pub const OBJ_ASSETS_DIR: &'static str = "./assets/objects/";
const SHADER_ASSETS_DIR: &'static str = "./assets/shaders/";
pub const FONT_ASSETS_DIR: &'static str = "./assets/fonts/";

static mut ENTITY_ID: usize = 0;
pub const ENTITY_SIZE: usize = 100_000;
pub type EntityID = usize;

pub enum MeshType {
    Textured(TexturedObj),
    Normal(NormalObj),
}

#[derive(Serialize, Deserialize)]
pub enum ObjType {
    Textured,
    Normal,
}

pub enum AssetSource {
    Mesh(ObjType, String),

    /// Name of shader, Vert, Frag, Option<Geo>
    Shader(String, String, String, Option<String>),
    Texture(String),
}

///Enum used by add resource function
#[derive(Debug)]
pub enum ResourceResult {
    Mesh(String),
    Shader(String),
    Texture(String),
}

pub struct Mesh {
    mesh_type: Option<MeshType>,
    is_loaded: bool,
}

impl Mesh {
    fn new() -> Self {
        Self {
            mesh_type: None,
            is_loaded: false,
        }
    }
}

impl Deref for Mesh {
    type Target = Option<MeshType>;

    fn deref(&self) -> &Self::Target {
        &self.mesh_type
    }
}

impl DerefMut for Mesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mesh_type
    }
}

struct ResourceLogs {

}

impl ResourceLogs {
    fn new() -> Self {
        Self {}
    }
}

impl Logable for ResourceLogs {
    fn to_string(&self) -> String {
        String::new()
    }
}

type MeshDataContainer = HashMap<String, Mesh>;
type ShaderContainer = HashMap<String, Option<u32>>;
//Render component will hold the mesh id and a copy of the mesh's vertex data
pub struct Resources {
    pub mesh_data: Arc<RwLock<MeshDataContainer>>,
    pub shaders: Arc<RwLock<ShaderContainer>>,
    log_manager: *mut LogManager,
}

impl Resources {
    pub fn new(log_manager: *mut LogManager) -> Self {
        //Note(ted) Loading and compiling the shaders
        Self {
            mesh_data: Arc::new(RwLock::new(HashMap::new())),
            shaders: Arc::new(RwLock::new(HashMap::new())),
            log_manager
        }
    }

    ///Threaded signal that the value is required immediately
    pub fn add_resource(&mut self, resource: AssetSource, threaded: bool) -> ResourceResult {
        //TODO(teddy) Spawn a thread to load the resources

        let mesh_shrd_ref = Arc::clone(&self.mesh_data);
        let shader_shrd_ref = Arc::clone(&self.shaders);

        match resource {
            AssetSource::Mesh(obj_type, location) => match obj_type {
                ObjType::Normal => {
                    let result = location.clone();

                    let mut mesh_container = mesh_shrd_ref.write().unwrap();

                    match mesh_container.get_mut(&result) {
                        Some(mesh) if mesh.is_loaded => {
                            return ResourceResult::Mesh(result);
                        }

                        None => {
                            //Note(teddy) Mesh is not created
                            mesh_container.insert(location.clone(), Mesh::new());
                        }

                        _ => unreachable!(),
                    }

                    drop(mesh_container); //Release lock

                    let mesh_ref_for_thread = Arc::clone(&self.mesh_data);
                    let load_mesh_routine = move || {
                        let mesh: NormalObj =
                            load_obj(format!("{}{}", OBJ_ASSETS_DIR, location).as_str()).unwrap();

                        let mut mesh_container = mesh_ref_for_thread.write().unwrap();
                        let mesh_type_ref = mesh_container.get_mut(&location).unwrap();
                        mesh_type_ref.mesh_type = Some(MeshType::Normal(mesh));
                        drop(mesh_container);
                    };

                    let mut mesh_container = mesh_shrd_ref.write().unwrap();
                    //Note(teddy) Check whether the mesh already exists so that we can use the cached data

                    let mesh = mesh_container.get_mut(&result).unwrap();
                    if threaded {
                        thread::spawn(load_mesh_routine);
                    } else {
                        load_mesh_routine();
                    }
                    mesh.is_loaded = true;

                    ResourceResult::Mesh(result)
                }

                ObjType::Textured => ResourceResult::Mesh(String::new()),
            },

            AssetSource::Shader(name, vertex, fragment, geo) => {
                let copy_for_result = name.clone();
                {
                    let mut shader_container = shader_shrd_ref.write().unwrap();
                    shader_container.insert(name.clone(), None);
                }

                let load_and_compile_shader_routine = move || {
                    //TODO(teddy) Handle this error gracefully
                    let geometry_shader = match geo {
                        Some(source) => Some(format!("{}{}", SHADER_ASSETS_DIR, source)),
                        None => None,
                    };

                    let shader = unsafe {
                        create_shader(
                            format!("{}{}", SHADER_ASSETS_DIR, vertex),
                            format!("{}{}", SHADER_ASSETS_DIR, fragment),
                            geometry_shader,
                        )
                        .unwrap()
                    };

                    let mut shader_container = shader_shrd_ref.write().unwrap();
                    shader_container.insert(name.clone(), Some(shader));
                };

                if threaded {
                    thread::spawn(load_and_compile_shader_routine);
                } else {
                    load_and_compile_shader_routine();
                }

                ResourceResult::Shader(copy_for_result)
            }

            AssetSource::Texture(_) => ResourceResult::Texture(String::new()),
        }
    }
}

pub struct World {
    event_manager: *mut EventManager,
    pub font_shader: u32,
    pub resources: Resources,
    pub components: Components,
    pub entities: LinkedList<EntityID>,
    pub deleted_entities: LinkedList<EntityID>,
}

impl World {
    pub fn new(event_manager: *mut EventManager, log_manager: *mut LogManager) -> Self {
        Self {
            event_manager,
            font_shader: 0,
            resources: Resources::new(log_manager),
            components: Components::new(ENTITY_SIZE),
            entities: LinkedList::new(),
            deleted_entities: LinkedList::new(),
        }
    }

    pub fn create_entity(&mut self) -> EntityID {
        let id = match self.deleted_entities.pop_front() {
            Some(recycled_id) => {
                //FIXME(teddy) We may incure some cache misses while iterating through entities
                self.entities.push_back(recycled_id);
                recycled_id
            }

            None => {
                let new_id = unsafe { ENTITY_ID };
                unsafe { ENTITY_ID += 1 };
                self.entities.push_back(new_id);
                self.components.create_entry();
                new_id
            }
        };

        let event_manager = unsafe { self.event_manager.as_mut().unwrap() };
        event_manager.add_event(Event::new(EventType::EntityCreated(id)));
        id
    }

    pub fn save(&mut self) {
        let mut world_entities = File::create("game_world").unwrap();
        let header = StorageFileHeader{ total_entities: self.entities.len() as u32 };
        let entity_objects = self.entities.iter().map(|entity_id| {
            Entity {
                transform: if let Some(transform_component) = &self.components.positionable[*entity_id] {
                    TransformData {
                        is_present: 1,
                        translation: [transform_component.position.translation.x, transform_component.position.translation.y, transform_component.position.translation.z],
                        rotation: [0.0, 0.0, 0.0],
                        scale: transform_component.scale
                    }
                } else {
                    TransformData {
                        is_present: 0,
                        translation: [0.0; 3],
                        rotation: [0.0; 3],
                        scale: 1.0
                    }

                },

                render: if let Some(render_component) = &self.components.renderables[*entity_id] {

                    assert!(render_component.mesh_label.len() <= 1024, "{}", true);
                    assert!(render_component.shader_label.len() <= 1024,"{}", true);
                    assert!(render_component.shader_label.len() <= 1024, "{}", true);

                    let textures_labels: Vec<[u8; 1024]> =  render_component.textures
                        .iter()
                        .map(|a| {
                            copy_string_to_bytes(a)
                        }).collect();

                    let mut textures: [[u8; 1024]; 8] = [[0; 1024]; 8];
                    for (i, label) in textures_labels.iter().enumerate() {
                        textures[i] = *label
                    }

                    RenderData {
                        is_present: 1,
                        mesh: copy_string_to_bytes(&render_component.mesh_label),
                        shader: copy_string_to_bytes(&render_component.shader_label),
                        textures: textures
                    }
                } else {
                    RenderData::default()
                },

                physics: if let Some(physics_data) = &self.components.physics[*entity_id] {
                    PhysicsData::default()
                } else {
                    PhysicsData::default()
                }
            }
        }).collect();
        write_entity_to_disk(&mut world_entities, header, entity_objects);

    }
}

fn copy_string_to_bytes(string: &String) -> [u8; 1024] {

    let mut mesh_data_output: [u8; 1024] = [0; 1024];
    for (i, char) in string.as_bytes().iter().enumerate() {
        mesh_data_output[i] = *char;
    }

    mesh_data_output
}

#[inline(always)]
fn write_entity_to_disk(
    file: &mut File,
    header: StorageFileHeader,
    entities: Vec<Entity>
) {

    //Note(teddy) writing the file headers
    let slice = unsafe { std::slice::from_raw_parts(
       &header as *const _ as *const u8,
        std::mem::size_of::<StorageFileHeader>()) 
    };
    file.write(slice).unwrap();

    println!("Writing entity to disk");
    let entity_data = unsafe { 
        entities
            .iter()
            .flat_map(|e: &Entity| any_as_u8_slice(e))
            .map(|byte| *byte)
            .collect::<Vec<u8>>() 
    };
    file.write_all(entity_data.as_slice()).unwrap()
}


unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        std::mem::size_of::<T>()
        )
}

#[repr(C)]
pub struct StorageFileHeader {
    total_entities: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ShaderObject {
    name: String,
    vert: String,
    frag: String,
    geo: Option<String>,
}

pub struct Level {
    entities: Vec<Entity>,
    shader_programs: Vec<ShaderObject>,
    meshes: Vec<(ObjType, String)>,
    font_shader: [String; 3],
}


#[repr(C)]
struct Entity {
    transform: TransformData,
    render: RenderData,
    physics: PhysicsData,
}


#[repr(C)]
struct TransformData {
    is_present: u8,
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
}

enum Body {
    Static = 0,
    Kinematic = 1,
    Dynamic = 2,
}


#[repr(C)]
struct PhysicsData {
    is_present: u8,
    mass: f32,
    gravity: bool,
    body: u8,
    velocity: [f32; 3],
    restitution: f32,
    friction: f32,
}

impl PhysicsData {
    fn default() -> Self {
        Self {
            is_present: 0,
            mass: 0.0,
            gravity: false,
            body: 0,
            velocity: [0.0; 3],
            restitution: 0.0,
            friction: 0.0
        }
    }
}


//Note(teddy) have a fixed size for the strings
#[repr(C)]
struct RenderData {
    is_present: u8,
    textures: [[u8; 1024]; 8],
    mesh: [u8; 1024],
    shader: [u8; 1024],
}

impl RenderData {
    fn default() -> Self {
        Self {
            is_present: 0,
            textures: [[0; 1024]; 8],
            mesh: [0; 1024],
            shader: [0; 1024]
        }
    }
}

pub enum WorldError {
    LevelNotFound,
    FailedToOpenLevel,
    UnableToParseFile,
}


fn load_game_world() -> Vec<Entity>{
    unimplemented!()
}