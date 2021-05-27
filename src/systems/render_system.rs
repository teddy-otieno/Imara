use std::collections::HashMap;

use nalgebra::Vector3;

use super::system::{System, SystemType};
use crate::core::{Engine, Event, EventManager, EventType};
use crate::game_world::components::TransformComponent;
use crate::game_world::world::{EntityID, MeshType, World};
use crate::renderer::draw::*;

macro_rules! border_shader {
    () => {
        String::from("highlight_shader")
    };
}
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
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::ClearStencil(0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT)
        };

        //Check for new created entities

        // dbg!(&event_manager.get_engine_events());
        for event in event_manager.get_engine_events().clone().into_iter() {
            match event.event_type {
                EventType::EntityCreated(id) => {
                    //If the object was already loaded skip
                    if self.normal_objects.contains_key(&id)
                        || self.textured_objects.contains_key(&id)
                    {
                        if event.is_pending_for(SystemType::RenderSystem) {
                            event_manager.remove_pending(event.id, SystemType::RenderSystem);
                        }
                        continue;
                    }

                    let render_component = match world.components.renderables[id].as_ref() {
                        Some(comp) => comp,
                        None => continue,
                    };

                    let mesh_label = &render_component.mesh_label;

                    let mesh_data = match world.resources.mesh_data.try_read() {
                        Ok(mesh_container) => mesh_container,

                        Err(_) => {
                            if !event.is_pending_for(SystemType::RenderSystem) {
                                event_manager.add_pending(event, SystemType::RenderSystem);
                            }

                            continue;
                        }
                    };

                    if let Some(some_mesh) = mesh_data.get(mesh_label) {
                        match &**some_mesh {
                            Some(mesh) => match mesh {
                                MeshType::Textured(obj) => {
                                    let _render_object = unsafe { init_textured_object(&obj) };
                                }

                                MeshType::Normal(obj) => {
                                    let render_object = unsafe { init_normal_object(&obj) };

                                    if let Some(_) = self.normal_objects.insert(id, render_object) {
                                        panic!("Weird, looks render object for this entity exists.")
                                    };

                                    if event.is_pending_for(SystemType::RenderSystem) {
                                        event_manager
                                            .remove_pending(event.id, SystemType::RenderSystem);
                                    }
                                }
                            },

                            None => {
                                if !event.is_pending_for(SystemType::RenderSystem) {
                                    event_manager.add_pending(event, SystemType::RenderSystem);
                                }

                                continue;
                            }
                        }
                    } else {
                        eprintln!("Looks like mesh of id {} was not loaded ", mesh_label);
                        continue;
                    }
                }

                EventType::EntityRemoved(id) => {
                    //Free VBOs and others

                    let render_component = match world.components.renderables[id].as_mut() {
                        Some(comp) => comp,
                        None => continue,
                    };

                    let mesh_label = &render_component.mesh_label;

                    let mesh_data = match world.resources.mesh_data.try_read() {
                        Ok(mesh_container) => mesh_container,

                        Err(_) => {
                            event_manager.add_pending(event, SystemType::RenderSystem);
                            continue;
                        }
                    };

                    if let Some(some_mesh) = mesh_data.get(mesh_label) {
                        if let Some(mesh) = &**some_mesh {
                            match mesh {
                                MeshType::Textured(_obj) => {
                                    remove_textured_object(
                                        id,
                                        self.textured_objects.remove(&id).unwrap(),
                                    );
                                }

                                MeshType::Normal(_obj) => {
                                    remove_normal_object(
                                        id,
                                        self.normal_objects.remove(&id).unwrap(),
                                    );
                                }
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
                        if let Some(higlight_component) = &world.components.highlightable[*i] {
                            gl::StencilFunc(gl::ALWAYS, 1, 0xFF);
                            gl::StencilMask(0xFF);

                            let draw_params = || {
                                // gl::Enable(gl::DEPTH_TEST);
                                // gl::StencilFunc(gl::ALWAYS, 1, 0xFF);
                                // gl::StencilMask(0xFF);
                            };

                            gl::Enable(gl::DEPTH_TEST);
                            gl::StencilFunc(gl::ALWAYS, 1, 0xFF);
                            gl::StencilMask(0xFF);

                            draw_normal_object(
                                &world,
                                &render_component.shader_label,
                                &engine.camera,
                                render_object,
                                &transform_component,
                                &engine.dir_lights,
                                draw_params,
                            )
                            .unwrap();

                            let scaled_transform = TransformComponent::new(
                                transform_component.position.translation.vector,
                                Vector3::y(),
                                1.1,
                            );

                            //let scaled_shader = &world.resources.shaders[&border_shader!()];
                            let scaled_params = || {
                                // gl::StencilFunc(gl::EQUAL, 1, 0xFF);
                                // gl::StencilMask(0x00);
                                // gl::Disable(gl::DEPTH_TEST);
                            };

                            //Drawing scaled version of the object
                            gl::StencilFunc(gl::NOTEQUAL, 1, 0xFF);
                            gl::StencilMask(0x00);
                            gl::Disable(gl::DEPTH_TEST);
                            draw_normal_object(
                                &world,
                                &border_shader!(),
                                &engine.camera,
                                render_object,
                                &scaled_transform,
                                &engine.dir_lights,
                                scaled_params,
                            )
                            .unwrap();
                        } else {
                            let draw_params = || {
                                gl::Enable(gl::CULL_FACE);
                                gl::Enable(gl::DEPTH_TEST);
                                gl::DepthFunc(gl::LESS);
                            };

                            draw_normal_object(
                                &world,
                                &render_component.shader_label,
                                &engine.camera,
                                render_object,
                                &transform_component,
                                &engine.dir_lights,
                                draw_params,
                            )
                            .unwrap()
                        }
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
