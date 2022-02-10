use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, RwLock, Mutex};
use std::mem::MaybeUninit;
use std::thread;
use std::io::Write;
use std::{
    collections::{HashMap, LinkedList},
    ops::{Deref, DerefMut},
};

use nalgebra::{Isometry3, Vector3};
use ncollide3d::simba::scalar::SupersetOf;
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
    pub mesh_type: Option<MeshType>,
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
    pub mesh_data: MeshDataContainer,
    pub shaders: ShaderContainer,
}

impl Resources {
    pub fn new(log_manager: *mut LogManager) -> Self {
        //Note(ted) Loading and compiling the shaders
        Self {
            mesh_data: HashMap::new(),
            shaders: HashMap::new(),
        }
    }


    pub fn add_resource(&mut self, resource: AssetSource, threaded: bool) {


        match resource {
            AssetSource::Mesh(obj_type, location) => match obj_type {
                ObjType::Normal => {
                    let result = location.clone();


                    match self.mesh_data.get_mut(&result) {
                        Some(mesh) if mesh.is_loaded => {
                            return;
                        }

                        None => {
                            //Note(teddy) Mesh is not created
                            self.mesh_data.insert(location.clone(), Mesh::new());
                        }

                        _ => unreachable!(),
                    }

                    // drop(mesh_container); //Release lock

                    //Note(teddy) Check whether the mesh already exists so that we can use the cached data
                    let mesh: NormalObj =
                        load_obj(format!("{}{}", OBJ_ASSETS_DIR, location).as_str()).unwrap();

                    let mesh_type_ref = self.mesh_data.get_mut(&location).unwrap();
                    mesh_type_ref.mesh_type = Some(MeshType::Normal(mesh));
                    mesh_type_ref.is_loaded = true;
                }

                ObjType::Textured => (),
            },

            AssetSource::Shader(name, vertex, fragment, geo) => {
                let copy_for_result = name.clone();
                self.shaders.insert(name.clone(), None);

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

                self.shaders.insert(name.clone(), Some(shader));

            }

            AssetSource::Texture(_) => (),
        }

    }
}

const GAME_WORLD_FILE_NAME: &'static str = "game_world";
pub struct World {
    event_manager: *mut EventManager,
    pub font_shader: u32,
    pub resources: Arc<RwLock<Resources>>,
    pub components: Components,
    pub entities: LinkedList<EntityID>,
    pub deleted_entities: LinkedList<EntityID>,
    pub resource_queue: Arc<(Mutex<LinkedList<AssetSource>>, Condvar)>
}

impl World {
    pub fn new(event_manager: *mut EventManager, log_manager: *mut LogManager) -> Self {
        Self {
            event_manager,
            font_shader: 0,
            resources: Arc::new(RwLock::new(Resources::new(log_manager))),
            components: Components::new(ENTITY_SIZE),
            entities: LinkedList::new(),
            deleted_entities: LinkedList::new(),
            resource_queue: Arc::new((Mutex::new(LinkedList::new()), Condvar::new()))
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


    pub fn add_resource(&mut self, resource: AssetSource) {
        match resource {
            AssetSource::Shader(name, vertex, fragment, geo) => {
                let mut resource_manager = self.resources.write().unwrap();
                let copy_for_result = name.clone();
                resource_manager.shaders.insert(name.clone(), None);

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

                resource_manager.shaders.insert(name.clone(), Some(shader));

            }

            _ => {
                let (mutex, cond) = &*self.resource_queue;
                let mut resource_queue = mutex.lock().unwrap();
                resource_queue.push_back(resource);
                cond.notify_one();
            }
        }
    }

    pub fn init_resource_loading_thread(&self) {
        let resource_queue_ref = Arc::clone(&self.resource_queue);
        let resources_ref = Arc::clone(&self.resources);

        std::thread::spawn(move || {
            let (mutex, cond) = &*resource_queue_ref;
            loop {
                let mut lock = mutex.lock().unwrap();
                let mut resource_queue = cond.wait(lock).unwrap();
                //TODO(teddy) add the loading code here
                let mut resource_manager = resources_ref.write().unwrap();
            
                while let Some(item) = resource_queue.pop_front() {
                    resource_manager.add_resource(item, false)
                }
            }
        });
    }

    pub fn save(&mut self) {
        let mut world_entities = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(GAME_WORLD_FILE_NAME)
            .unwrap();
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

                // physics: if let Some(physics_data) = &self.components.physics[*entity_id] {
                //     PhysicsData::default()
                // } else {
                //     PhysicsData::default()
                // }
            }
        }).collect();
        write_entity_to_disk(&mut world_entities, header, entity_objects);

    }

    pub fn load_world(&mut self) { 
        let SIZE_OF_HEADER: usize = std::mem::size_of::<StorageFileHeader>();
        let SIZE_OF_ENTITY: usize = std::mem::size_of::<Entity>();

        let world_entities_file = File::open(GAME_WORLD_FILE_NAME).unwrap();
        let mut buffered_reader = BufReader::new(world_entities_file);

        //Read the entire file to buffer
        let mut temp_buffer = vec![];

        let write_ref = unsafe {
            (&buffered_reader as *const BufReader<File> as *mut BufReader<File>)
                    .as_mut()
                    .unwrap()
        };
        while let Ok(buf) = buffered_reader.fill_buf() {
            temp_buffer.extend_from_slice(&buf);
            if buf.len() == 0 {
                break;
            }
            write_ref.consume(buf.len());
        }


        let file_header_buffer = &temp_buffer[0..SIZE_OF_HEADER];
        //Loading the file header to obtain configurations for the world

        let file_header: MaybeUninit<StorageFileHeader> = MaybeUninit::zeroed();
        let mut storage_header = unsafe { file_header.assume_init() };
        let storage_header_ptr: *mut StorageFileHeader = &mut storage_header;


        unsafe {
            std::ptr::copy(
                file_header_buffer.as_ptr(), 
                storage_header_ptr as *mut u8, 
                std::mem::size_of_val(&file_header_buffer)
            )
        };
        buffered_reader.consume(SIZE_OF_HEADER);

        //Loading the entities
        //



        let entities_data_buffer = &temp_buffer[SIZE_OF_HEADER..];

        dbg!(entities_data_buffer.len());
        dbg!(storage_header.total_entities);
        dbg!(entities_data_buffer.len() as f32 / SIZE_OF_ENTITY as f32);

        let loaded_entities: &[Entity] = unsafe { std::mem::transmute::<&[u8], &[Entity]>(entities_data_buffer) };

        for i in 0..storage_header.total_entities {
            // dbg!(&loaded_entities[i as usize].transform);
            println!("{:?}", loaded_entities[i as usize]);
            self.create_loaded_entity(&loaded_entities[i as usize]).unwrap();
        }
    }

    fn create_loaded_entity(&mut self, entity: &Entity) -> Result<(), String> {

        let new_entity = self.create_entity();

        if entity.render.is_present == 1 {
            let truncate_zeros = |it:[u8; 1024]|  {
                it.into_iter()
                    .filter(|c| **c != 0)
                    .map(|c| *c)
                    .collect::<Vec<u8>>()
            };

            let mesh_label_bytes = truncate_zeros(entity.render.mesh);
            let shader_label_bytes = truncate_zeros(entity.render.shader);

            let mesh_label = unsafe {
                String::from_utf8(mesh_label_bytes).unwrap()
            };
            self.add_resource(AssetSource::Mesh(ObjType::Normal, mesh_label.clone()));
            let shader_label = unsafe {
                String::from_utf8(shader_label_bytes).unwrap()
            };
            //TODO(teddy) Not sure about how the mesh ids work

            println!("Reached here");
            self.components.renderables[new_entity] = Some(
                RenderComponent::new(
                    mesh_label.clone(),
                    shader_label,
                )
            );

        }

        if entity.transform.is_present == 1 {
            let [x, y, z] = entity.transform.translation;
            let [rot_x, rot_y, rot_z] = entity.transform.rotation;

            self.components.positionable[new_entity] = Some(
                TransformComponent::new(
                     Vector3::new(x, y, z), 
                     Vector3::new(rot_x, rot_y, rot_z),
                     entity.transform.scale
                )
            )
        }

        Ok(())
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
    let file_header_data = unsafe { std::slice::from_raw_parts( &header as *const _ as *const u8, std::mem::size_of::<StorageFileHeader>()) };

    let entity_data = unsafe { 
        entities
            .iter()
            .flat_map(|e: &Entity| any_as_u8_slice(e))
            .map(|byte| *byte)
            .collect::<Vec<u8>>() 
    };

    let file_data = file_header_data
        .into_iter()
        .chain(entity_data.iter())
        .map(|byte| *byte)
        .collect::<Vec<u8>>();
    file.write_all(file_data.as_slice()).unwrap();
    println!("Entities written to the disk");
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
#[derive(Debug)]
struct Entity {
    transform: TransformData,
    render: RenderData,
    // physics: PhysicsData,
}


#[repr(C)]
#[derive(Debug)]
struct TransformData {
    is_present: u8,
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
}

#[derive(Debug)]
enum Body {
    Static = 0,
    Kinematic = 1,
    Dynamic = 2,
}


#[repr(C)]
#[derive(Debug)]
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
#[derive(Debug)]
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
