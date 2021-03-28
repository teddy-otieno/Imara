use nalgebra::{Point3, Vector3};
use ncollide3d::pipeline::object::CollisionGroups;
use ncollide3d::shape::{Ball, ShapeHandle, TriMesh};

use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodySet, DefaultColliderSet, RigidBodyDesc,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};

use super::system::System;
use crate::core::{CastedRay, Engine, Event, EventManager};
use crate::game_world::world::{MeshType, World};

pub struct Physics {
    mechanical_world: DefaultMechanicalWorld<f32>,
    geometrical_world: DefaultGeometricalWorld<f32>,
    bodies: DefaultBodySet<f32>,
    colliders: DefaultColliderSet<f32>,
    joint_constraints: DefaultJointConstraintSet<f32>,
    force_generators: DefaultForceGeneratorSet<f32>,
}

impl Physics {
    pub fn new() -> Self {
        Self {
            mechanical_world: DefaultMechanicalWorld::new(Vector3::new(0.0, -9.81, 0.0)),
            geometrical_world: DefaultGeometricalWorld::new(),
            bodies: DefaultBodySet::new(),
            colliders: DefaultColliderSet::new(),
            joint_constraints: DefaultJointConstraintSet::new(),
            force_generators: DefaultForceGeneratorSet::new(),
        }
    }

    #[inline]
    fn handle_physics_events(&mut self, world: &mut World, _event_manager: &mut EventManager) {
        for entity in world.entities.iter() {
            let physics_component = match world.components.physics[*entity].as_ref() {
                Some(component) => component,
                None => continue,
            };

            let transform_component = match world.components.positionable[*entity].as_mut() {
                Some(component) => component,
                None => continue,
            };

            let rigid_body = self
                .bodies
                .rigid_body(physics_component.rigid_handle.unwrap())
                .unwrap();

            // let collider = self
            //     .colliders
            //     .get(physics_component.collider_handle.unwrap())
            //     .unwrap();

            transform_component.position = rigid_body.position().clone();
            //dbg!(&rigid_body.position());
            //dbg!(&collider.position());
            //panic!();
        }
    }

    #[inline]
    fn handle_world_events(&mut self, world: &mut World, event_manager: *mut EventManager) {
        for event in unsafe { &*event_manager }.get_engine_events() {
            match event {
                Event::EntityCreated(id) => {
                    let physics_component = match world.components.physics[*id].as_mut() {
                        Some(component) => component,
                        None => continue,
                    };
                    let transform_component = match world.components.positionable[*id].as_ref() {
                        Some(component) => component,
                        None => continue,
                    };

                    //Note(teddy) Creating rigid body object
                    let rigid_body = RigidBodyDesc::new()
                        .position(transform_component.position)
                        .mass(physics_component.mass)
                        .gravity_enabled(physics_component.gravity)
                        .status(physics_component.status)
                        .build();

                    let rigid_body_handle = self.bodies.insert(rigid_body);
                    physics_component.rigid_handle = Some(rigid_body_handle);

                    let shape = {
                        if let Some(render_component) = &world.components.renderables[*id] {
                            // construct a trimesh
                            let mesh_id = render_component.mesh_id;

                            let trimesh = match &world.resources.mesh_data[mesh_id] {
                                MeshType::Normal(obj) => {
                                    let mut indices = vec![];

                                    divide_indices(obj.indices.clone(), &mut indices);
                                    TriMesh::new(
                                        obj.vertices.iter().map(|p| p.xyz()).collect(),
                                        indices,
                                        None,
                                    )
                                }

                                MeshType::Textured(_obj) => {
                                    unimplemented!();
                                }
                            };

                            ShapeHandle::new(trimesh)
                        } else {
                            //Construct a ball, this entity could be a sensor

                            ShapeHandle::new(Ball::new(1.5))
                        }
                    };

                    let collider_body = ColliderDesc::new(shape)
                        //.ccd_enabled(true)
                        .margin(0.2)
                        .material(physics_component.material_handle.clone())
                        .build(BodyPartHandle(rigid_body_handle, 0));

                    let collider_handle = self.colliders.insert(collider_body);

                    physics_component.rigid_handle = Some(rigid_body_handle);
                    physics_component.collider_handle = Some(collider_handle);
                }

                Event::EntityRemoved(_id) => {}

                Event::CastRay(data) => {
                    let collider_groups = CollisionGroups::new();
                    let interferences = self.geometrical_world.interferences_with_ray(
                        &self.colliders,
                        &data.ray,
                        10000.0,
                        &collider_groups,
                    );

                    let mut ray_casted_event = CastedRay {
                        id: data.id,
                        entity: None,
                    };

                    let mut min = 100000.0;

                    for (id, _collider, intersection) in interferences {
                        for (entity_id, physics_component) in
                            world.components.physics.iter().enumerate()
                        {
                            //Note(teddy) This might result in undefined behaviour sooner or later
                            if let Some(component) = physics_component {
                                dbg!(&intersection);
                                match component.collider_handle {
                                    Some(handle) if handle == id => {
                                        if intersection.toi < min {
                                            ray_casted_event.entity = Some(entity_id);
                                            min = intersection.toi;
                                        }
                                    }

                                    _ => (),
                                }
                            }
                        }
                    }
                    unsafe { &mut *event_manager }
                        .add_engine_event(Event::RayCasted(ray_casted_event));
                }
                _ => (),
            }
        }
    }
}

impl System for Physics {
    fn name(&self) -> String {
        String::from("Physics")
    }

    fn update(
        &mut self,
        world: &mut World,
        event_manager: &mut EventManager,
        _engine: &mut Engine,
        _delta_time: f32,
    ) {
        self.handle_world_events(world, event_manager);

        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );

        self.handle_physics_events(world, event_manager);

        //Check is object has intersected with the camera view direction
    }
}

fn divide_indices(mut ind: Vec<u32>, result: &mut Vec<Point3<usize>>) {
    assert!(ind.len() % 3 == 0, true);

    if ind.len() == 3 {
        result.push(Point3::new(
            ind[0] as usize,
            ind[1] as usize,
            ind[2] as usize,
        ));
    } else {
        let next = ind.split_off(3);
        assert!(ind.len() == 3, true);
        result.push(Point3::new(
            ind[0] as usize,
            ind[1] as usize,
            ind[2] as usize,
        ));
        divide_indices(next, result);
    }
}

mod tests {
    use super::divide_indices;

    #[test]
    fn test_divide_indices() {
        let list_of_indices = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut result = Vec::with_capacity(list_of_indices.len() / 3);

        divide_indices(list_of_indices, &mut result);

        println!("{:?}", result);
        assert!(result.len() == 3, true);
    }
}
