use nalgebra::{Isometry3, Vector3};
use nphysics3d::material::MaterialHandle;
use nphysics3d::object::{BodyStatus, DefaultBodyHandle, DefaultColliderHandle};

pub struct Components {
    pub renderables: Vec<Option<RenderComponent>>,
    pub positionable: Vec<Option<TransformComponent>>,
    pub physics: Vec<Option<PhysicsComponent>>,
    pub highlightable: Vec<Option<HighlightComponent>>,
}

impl Components {
    pub fn new(capacity: usize) -> Self {
        Self {
            renderables: Vec::with_capacity(capacity),
            positionable: Vec::with_capacity(capacity),
            physics: Vec::with_capacity(capacity),
            highlightable: Vec::with_capacity(capacity),
        }
    }

    pub fn create_entry(&mut self) {
        self.renderables.push(None);
        self.positionable.push(None);
        self.physics.push(None);
        self.highlightable.push(None);
    }
}

#[derive(Debug)]
pub struct HighlightComponent {
    pub color: [f32; 3],
}

#[derive(Debug)]
pub struct RenderComponent {
    pub should_update: bool,
    pub mesh_label: String,
    pub shader_label: String,
    pub textures: Vec<String>,
}

impl RenderComponent {
    pub fn new(mesh_label: String, shader_label: String) -> Self {
        //let (vertex_data, indices) = Self::process_mesh(mesh);

        Self {
            should_update: true,
            mesh_label,
            shader_label,
            textures: vec![],
        }
    }

    //TODO(teddy): To be move the render system
}

#[derive(Debug)]
pub struct TransformComponent {
    pub position: Isometry3<f32>,
    pub scale: f32,
}

impl TransformComponent {
    pub fn new(translation: Vector3<f32>, rotation: Vector3<f32>, scale: f32) -> Self {
        Self {
            position: Isometry3::new(translation, rotation),
            scale,
        }
    }
}

pub struct PhysicsComponent {
    pub rigid_handle: Option<DefaultBodyHandle>,
    pub collider_handle: Option<DefaultColliderHandle>,
    pub material_handle: MaterialHandle<f32>,
    pub mass: f32,
    pub gravity: bool,
    pub status: BodyStatus,
    pub velocity: Vector3<f32>,
}

impl PhysicsComponent {
    pub fn new(
        mass: f32,
        gravity: bool,
        status: BodyStatus,
        initial_velocity: Vector3<f32>,
        material_handle: MaterialHandle<f32>,
    ) -> Self {
        Self {
            rigid_handle: None,
            collider_handle: None,
            material_handle,
            mass,
            gravity,
            status,
            velocity: initial_velocity,
        }
    }
}
