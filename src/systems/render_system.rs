use std::collections::HashMap;

use super::system::System;
use crate::core::{Engine, Event, EventManager};
use crate::game_world::world::{EntityID, MeshType, World};
use crate::renderer::draw::*;

pub struct Renderer {
    normal_objects: HashMap<EntityID, RenderObject>,
    textured_objects: HashMap<EntityID, RenderObject>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            normal_objects: HashMap::new(),
            textured_objects: HashMap::new(),
        }
    }
}

impl System for Renderer {
    fn name(&self) -> String {
        String::from("Renderer")
    }

    fn update(
        &mut self,
        world: &mut World,
        event_manager: &mut EventManager,
        engine: &mut Engine,
        _delta_time: f32,
    ) {
        unsafe { gl::ClearColor(0.1, 0.1, 0.1, 1.0) };
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT) };

        //Check for new created entities

        // dbg!(&event_manager.get_engine_events());
        for event in event_manager.get_engine_events().iter() {
            match event {
                Event::EntityCreated(id) => {
                    let render_component = match world.components.renderables[*id].as_ref() {
                        Some(comp) => comp,
                        None => continue,
                    };

                    let mesh_id = render_component.mesh_id;

                    if let Some(mesh) = world.resources.mesh_data.get(mesh_id) {
                        match mesh {
                            MeshType::Textured(obj) => {
                                let _render_object = unsafe { init_textured_object(&obj) };
                            }

                            MeshType::Normal(obj) => {
                                let render_object = unsafe { init_normal_object(&obj) };

                                if let Some(_) = self.normal_objects.insert(*id, render_object) {
                                    panic!("Weird, looks render object for this entity exists.")
                                };
                            }
                        }
                    } else {
                        eprintln!("Looks like mesh of id {} was not loaded ", mesh_id);
                        continue;
                    }
                }

                Event::EntityRemoved(id) => {
                    //Free VBOs and others

                    let render_component = match world.components.renderables[*id].as_mut() {
                        Some(comp) => comp,
                        None => continue,
                    };

                    let mesh_id = render_component.mesh_id;

                    if let Some(mesh) = world.resources.mesh_data.get(mesh_id) {
                        match mesh {
                            MeshType::Textured(_obj) => {
                                remove_textured_object(
                                    *id,
                                    self.textured_objects.remove(id).unwrap(),
                                );
                            }

                            MeshType::Normal(_obj) => {
                                remove_normal_object(*id, self.normal_objects.remove(id).unwrap());
                            }
                        }
                    }
                }

                _ => (),
            }
        }

        //Dr
        for i in world.entities.iter() {
            let render_component = match &world.components.renderables[*i] {
                Some(component) => component,
                None => continue,
            };

            let transform_component = match &world.components.positionable[*i] {
                Some(component) => component,
                None => continue,
            };

            if render_component.should_update {
                if render_component.textures.len() == 0 {
                    let render_object = match self.normal_objects.get(i) {
                        Some(object) => object,
                        None => continue,
                    };

                    unsafe {
                        draw_normal_object(
                            &world,
                            render_component.shader_id,
                            &engine.camera,
                            render_object,
                            &transform_component,
                            &engine.dir_lights,
                            &world.components.highlightable[*i],
                        )
                        .unwrap()
                    };
                }
            }
        }

        //TODO(teddy) Split this ui updates to a seperate thread

        unsafe {
            draw_ui(engine);
        }
    }
}

//TODO(teddy) Draw on a seperate frame buffer
unsafe fn draw_ui(engine: *mut Engine) {
    let eng = engine.as_mut().unwrap();
    // for view in eng.ui_view.iter_mut() {
    //     view.update(engine.as_mut().unwrap()).unwrap();
    // }

    if let Some(view) = &mut eng.get_ui_tree().unwrap().root {
        match view.update(engine.as_ref().unwrap()) {
            Ok(_) => (),
            Err(_) => println!("A view failed to update"),
        }
    }
}
