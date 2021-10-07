use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::{c_void, CString};

use nalgebra::Vector3;

use super::system::{System, SystemType};
use crate::core::{Engine, Event, EventManager, EventType, ViewPortDimensions, bind_texture};
use crate::game_world::components::TransformComponent;
use crate::game_world::world::{EntityID, MeshType, World};
use crate::renderer::draw::*;

#[macro_export]
macro_rules! border_shader {
    () => {
        String::from("highlight_shader")
    };
}

#[macro_export]
macro_rules! SCREEN_SHADER {
    () => {
        String::from("screen_shader")
    };
}

pub struct Renderer {
    normal_objects: HashMap<EntityID, RenderObject>,
    textured_objects: HashMap<EntityID, RenderObject>,
    screen_vao: Option<u32>,
    screen_shader_program: Option<u32>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            normal_objects: HashMap::new(),
            textured_objects: HashMap::new(),
            screen_vao: None,
            screen_shader_program: None
        }
    }

    unsafe fn draw_entities(&mut self, engine_ptr: *mut Engine, world: &mut World) {
        let engine = engine_ptr.as_mut().unwrap();

        gl::BindFramebuffer(gl::FRAMEBUFFER, engine.scene_render_object.frame_buffer);
        //gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::ClearColor(0.1, 0.1, 0.1, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Enable(gl::DEPTH_TEST);

        for i in world.entities.iter() {
            let render_component = match &world.components.renderables[*i] {
                Some(component) => component,
                None => continue,
            };

            let transform_component = match &world.components.positionable[*i] {
                Some(component) => component,
                None => continue,
            };

            if !render_component.should_update {
                continue;
            }

            if render_component.textures.len() == 0 {
                let render_object = match self.normal_objects.get(i) {
                    Some(object) => object,
                    None => continue,
                };

                if let Some(_) = &world.components.highlightable[*i] {
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
            }
        }

        let ViewPortDimensions {width, height} = engine.camera.view_port;

        let mut texture_data: Vec<u8> = Vec::with_capacity((width * height * 1000).try_into().unwrap());
        gl::ReadPixels(0, 0, width, height, gl::RGB, gl::UNSIGNED_BYTE, texture_data.as_mut_ptr() as *mut c_void);
        //println!("{:?}", texture_data.len());
    }

    fn handle_system_events(&mut self, event_manager: &mut EventManager, world: &mut World) {

        for event in event_manager.get_engine_events().clone().into_iter() {
            match event.event_type {
                EventType::EntityCreated(id) => {
                    //If the object was already loaded skip
                    if self.normal_objects.contains_key(&id) || self.textured_objects.contains_key(&id) {
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
    }
}

impl System for Renderer {
    fn name(&self) -> String {
        String::from("Renderer")
    }


    fn init(&mut self, world: &mut World, _engine: &mut Engine) -> Result<(), String> {
        let shader_name = SCREEN_SHADER!();

        let resources = world.resources.shaders.read().unwrap();
        let screen_shader = match resources.get(&shader_name) {

            Some(id) => {
                if let Some(shader_id) = id {
                    *shader_id
                } else {
                    return Err(String::from("Failed to load the screen shader"));
                }
            }

            None => return Err(String::from("Failed to load the screen shader"))
        };

        let vertices = vec![
            -1.0,   1.0,    0.0,    1.0,
            -1.0,   -1.0,    0.0,    0.0,
             1.0,    -1.0,    1.0,    0.0,

            -1.0,     1.0,    0.0,    1.0,
             1.0,    -1.0,   1.0,    0.0, 
             1.0,     1.0,    1.0,    1.0f32,
        ];


        //let vertices: Vec<f32> = raw_vertices.iter().map(|a| a * 0.5).collect();


        let mut vao = 0;
        let mut vbo = 0;

        let size_of_float: i32 = std::mem::size_of::<f32>().try_into().unwrap();

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER, (size_of_float as usize  * vertices.len()).try_into().unwrap(), vertices.as_ptr() as *mut c_void, gl::STATIC_DRAW);

            //Note(teddy) Enable attribute for the screen vertext cords
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 4 * size_of_float, 0 as *const c_void);
            gl::EnableVertexAttribArray(0);

            gl::BindVertexArray(0);
        }

        //TODO(teddy) not sure if I should bind vbo to the object
        self.screen_vao = Some(vao);
        self.screen_shader_program = Some(screen_shader);

        Ok(())

    }

    fn update(
        &mut self,
        world: &mut World,
        event_manager: &mut EventManager,
        engine: &mut Engine,
        _delta_time: f32,
    ) {


        self.handle_system_events(event_manager, world);

        unsafe {
            self.draw_entities(engine, world);
            draw_ui(engine);

            //Note(teddy) I guess the texturing is not working
            //Note(teddy) Drawing the screen shadee
            //Using the sceen texture
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
            gl::Clear(gl::COLOR_BUFFER_BIT);



            //FIXME(teddy) Fix\ff
            if let Some(vao) = self.screen_vao {
                gl::BindVertexArray(vao);

                gl::Disable(gl::DEPTH_TEST);
                gl::Disable(gl::STENCIL_TEST);

                let program  = match self.screen_shader_program {
                    Some(id) => {
                        gl::UseProgram(id);
                        id
                    }
                    _ => panic!(),
                };
                bind_texture(&engine.scene_render_object, 0, program, "scene_shader");
                bind_texture(engine.ui_render_object.as_ref().unwrap(), 1, program, "ui_texture");
                gl::DrawArrays(gl::TRIANGLES, 0, 6);

                gl::BindVertexArray(0);
            }
        }
    }
}



//TODO(teddy) Draw on a seperate frame buffer
unsafe fn draw_ui(engine: *mut Engine) {
    let eng = engine.as_mut().unwrap();
    let ui_frame_buffer = eng.ui_render_object.as_ref().unwrap().frame_buffer;

    gl::BindFramebuffer(gl::FRAMEBUFFER, ui_frame_buffer);
    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    gl::Enable(gl::DEPTH_TEST);

    //TODO(Teddy) Do all the buffer clearing operations

    if let Some(view) = &mut eng.get_ui_tree().unwrap().root {
        match view.update(engine.as_ref().unwrap()) {
            Ok(_) => (),
            Err(_) => println!("A view failed to update"),
        }
    }
}
