use std::collections::LinkedList;
use std::fs::File;
use std::io::BufReader;

use nalgebra::Vector3;
use nphysics3d::material::{BasicMaterial, MaterialHandle};
use nphysics3d::object::BodyStatus;
use serde::{Deserialize, Serialize};

use super::components::*;
use crate::core::Event;
use crate::core::EventManager;
use crate::obj_parser::{load_obj, NormalObj, TexturedObj};
use crate::renderer::shaders::create_shader;

const WORLD_LEVELS_DIR: &'static str = "C:\\Users\\teddj\\dev\\work\\Imara\\assets\\levels\\";
const OBJ_ASSETS_DIR: &'static str = "C:\\Users\\teddj\\dev\\work\\Imara\\assets\\objects\\";
const SHADER_ASSETS_DIR: &'static str = "C:\\Users\\teddj\\dev\\work\\Imara\\assets\\shaders\\";
pub const FONT_ASSETS_DIR: &'static str = "C:\\Users\\teddj\\dev\\work\\Imara\\assets\\fonts\\";

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
    Shader(String, String, Option<String>),
    Texture(String),
}

//Render component will hold the mesh id and a copy of the mesh's vertex data
pub struct Resources {
    pub mesh_data: Vec<MeshType>,
    pub shaders: LinkedList<u32>,
}

impl Resources {
    pub fn new() -> Self {
        //Note(ted) Loading and compiling the shaders
        Self {
            mesh_data: vec![],
            shaders: LinkedList::new(),
        }
    }

    pub fn add_resource(&mut self, resource: AssetSource) -> usize {
        match resource {
            AssetSource::Mesh(obj_type, location) => match obj_type {
                ObjType::Normal => {
                    let mesh =
                        load_obj::<NormalObj>(format!("{}{}", OBJ_ASSETS_DIR, location).as_str())
                            .unwrap();
                    let id = self.mesh_data.len();
                    self.mesh_data.push(MeshType::Normal(mesh));

                    id
                }

                ObjType::Textured => 0,
            },

            AssetSource::Shader(vertex, fragment, geo) => {
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
                let id = self.shaders.len();
                self.shaders.push_back(shader);

                id as usize
            }

            AssetSource::Texture(_) => 0,
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
    pub fn new(event_manager: *mut EventManager) -> Self {
        Self {
            event_manager,
            font_shader: 0,
            resources: Resources::new(),
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
        event_manager.add_event(Event::EntityCreated(id));
        id
    }
}

#[derive(Serialize, Deserialize)]
pub struct Level {
    entities: Vec<Entity>,
    shader_programs: Vec<[String; 3]>,
    meshes: Vec<(ObjType, String)>,
    font_shader: [String; 3],
}

#[derive(Serialize, Deserialize)]
struct Entity {
    transform: TransformData,
    physics: PhysicsData,
    render: RenderData,
}

#[derive(Serialize, Deserialize)]
struct TransformData {
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
}

#[derive(Serialize, Deserialize)]
enum Body {
    Static,
    Kinematic,
    Dynamic,
}

#[derive(Serialize, Deserialize)]
struct PhysicsData {
    mass: f32,
    gravity: bool,
    body: Body,
    velocity: [f32; 3],
    restitution: f32,
    friction: f32,
}

#[derive(Serialize, Deserialize)]
struct RenderData {
    textures: Vec<String>,
    mesh: usize,
    shader: usize,
}

pub enum WorldError {
    LevelNotFound,
    FailedToOpenLevel,
    UnableToParseFile,
}

pub fn load_level(source: &str, world: *mut World) -> Result<(), WorldError> {
    let path = format!("{}/{}", WORLD_LEVELS_DIR, source);

    if !std::path::Path::new(&path).exists() {
        return Err(WorldError::LevelNotFound);
    }

    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Err(WorldError::FailedToOpenLevel),
    };

    let buff_reader = BufReader::new(file);

    let level: Level = match serde_json::from_reader(buff_reader) {
        Ok(level) => level,
        Err(_) => return Err(WorldError::UnableToParseFile),
    };

    let world_ref = unsafe { &mut *world };
    for shader in level.shader_programs {
        let geo = if shader[2].is_empty() {
            None
        } else {
            Some(shader[2].clone())
        };

        world_ref.resources.add_resource(AssetSource::Shader(
            shader[0].clone(),
            shader[1].clone(),
            geo,
        ));
    }

    for (obj_type, source) in level.meshes {
        world_ref
            .resources
            .add_resource(AssetSource::Mesh(obj_type, source));
    }

    for entity in level.entities.iter() {
        let id = world_ref.create_entity();

        let translation = Vector3::new(
            entity.transform.translation[0],
            entity.transform.translation[1],
            entity.transform.translation[2],
        );

        let rotation = Vector3::new(
            entity.transform.rotation[0],
            entity.transform.rotation[1],
            entity.transform.rotation[2],
        );

        world_ref.components.positionable[id] = Some(TransformComponent::new(
            translation,
            rotation,
            entity.transform.scale,
        ));

        let get_status = |s: &Body| match s {
            Body::Static => BodyStatus::Static,
            Body::Kinematic => BodyStatus::Kinematic,
            Body::Dynamic => BodyStatus::Dynamic,
        };

        let velocity = Vector3::new(
            entity.physics.velocity[0],
            entity.physics.velocity[1],
            entity.physics.velocity[2],
        );

        let material = MaterialHandle::new(BasicMaterial::new(
            entity.physics.restitution,
            entity.physics.friction,
        ));

        world_ref.components.physics[id] = Some(PhysicsComponent::new(
            entity.physics.mass,
            entity.physics.gravity,
            get_status(&entity.physics.body),
            velocity,
            material,
        ));

        //Hello world
        world_ref.components.renderables[id] = Some(RenderComponent::new(
            entity.render.mesh,
            entity.render.shader,
        ));
    }

    Ok(())
}
