use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::{c_void, CString};
use std::time::Instant;

use nalgebra::Vector3;

use super::system::{System, SystemType};
use crate::core::{Engine, EventManager, Camera, EventType, Light, ViewPortDimensions, bind_texture, Event};
use crate::game_world::components::{TransformComponent, RenderComponent};
use crate::game_world::world::{EntityID, MeshType, World};
use crate::logs::{LogManager, Logable};
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

struct RenderSystemLogObject { 
    text: String
}

impl Logable for RenderSystemLogObject {
    fn to_string(&self) -> String {
        self.text.clone()
    }
}

type ComponentsForRender<'a> = (EntityID, &'a RenderComponent, &'a TransformComponent);

impl World {
    //TODO(teddy) construct an iterator
    fn get_render_components(&self) -> Vec<ComponentsForRender> {
        let mut render_components = vec![];
        for entity in &self.entities {
            let render = match self.components.renderables.get(*entity) {
                Some(comp) if comp.is_some() => comp.as_ref().unwrap(),
                _ => continue
            };

            let tranform = match self.components.positionable.get(*entity) {
                Some(comp) if comp.is_some() => comp.as_ref().unwrap(),
                _ => continue
            };

            render_components.push((*entity, render, tranform))
        }
        render_components
    }

    fn get_render_component(&self, id: EntityID) -> Option<&RenderComponent> {
        match self.components.renderables.get(id) { //TODO(teddy) Refactor
            Some(comp) => comp.as_ref(),
            None => None
        }
    }
}

struct HighlightReferences<'a> {
    world: &'a World,
    shader_label: &'a String,
    camera: &'a Camera,
    transform: &'a TransformComponent,
    light: &'a Light,
    object: &'a RenderObject
}

#[inline]
unsafe fn draw_with_highlight(data: HighlightReferences) {
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
        data.world,
        data.shader_label,
        data.camera,
        data.object,
        data.transform,
        data.light,
        draw_params,
        )
        .unwrap();

    let scaled_transform = TransformComponent::new(
        data.transform.position.translation.vector,
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
        &data.world,
        &border_shader!(),
        &data.camera,
        data.object,
        &data.transform,
        &data.light,
        scaled_params,
        )
        .unwrap();
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

        if world.entities.len() == 0 {
            return;
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, engine.scene_render_object.frame_buffer);
        //gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::ClearColor(0.1, 0.1, 0.1, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Enable(gl::DEPTH_TEST);

        for (i, render_component, transform_component) in world.get_render_components() {
            let render_object = match self.normal_objects.get(&i) {
                Some(object) => object,
                None => continue,
            };

            if render_component.highlight.is_none() {
                let draw_params = || {
                    gl::Enable(gl::CULL_FACE);
                    gl::Enable(gl::DEPTH_TEST);
                    gl::DepthFunc(gl::LESS);
                };

                draw_normal_object(
                    &world,
                    &render_component.shader_label,
                    &engine.camera      ,
                    render_object,
                    &transform_component,
                    &engine.dir_lights,
                    draw_params,
                )
                .unwrap();
                continue;
            }

            draw_with_highlight(HighlightReferences { 
                world: &world, 
                shader_label: &render_component.shader_label, 
                camera: &engine.camera, 
                transform: &transform_component, 
                light: &engine.dir_lights, 
                object: &render_object
            });
        }

        let ViewPortDimensions {width, height} = engine.camera.view_port;

        let mut texture_data: Vec<u8> = Vec::with_capacity((width * height * 1000).try_into().unwrap());
        gl::ReadPixels(0, 0, width, height, gl::RGB, gl::UNSIGNED_BYTE, texture_data.as_mut_ptr() as *mut c_void);
        //println!("{:?}", texture_data.len());
    }

    fn allocate_entity(
        &mut self, 
        event: Event, 
        id: EntityID, 
        event_manager: &mut EventManager, 
        mesh: &Option<MeshType>
    ) -> Result<(), String>{
        let mesh_type = match mesh {
            Some(e) => e,
            None => {
                if !event.is_pending_for(SystemType::RenderSystem) {
                    event_manager.add_pending(event, SystemType::RenderSystem);
                }
                return Err(format!(""));
            }
        };

        match mesh_type {
            MeshType::Textured(obj) => {
                let _render_object = unsafe { init_textured_object(&obj) };
            },
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
        };
        Ok(())

    }

    fn handle_entity_creation(
        &mut self, 
        id: EntityID, 
        event: Event, 
        event_manager: &mut EventManager, 
        world: &mut World
    ) -> Result<(), String> {
        if (self.normal_objects.contains_key(&id) || self.textured_objects.contains_key(&id)) 
            && event.is_pending_for(SystemType::RenderSystem) {
            return Err(format!(""));
        }
        let mesh_label = match world.get_render_component(id) {
            Some(comp) => &comp.mesh_label,
            None => return Err(format!("Component was not found")) ,
        };

        match world.resources.try_read() {
            Ok(res) if res.mesh_data.contains_key(mesh_label) =>
                self.allocate_entity(event, id, event_manager, &res.mesh_data[mesh_label].mesh_type),
            Err(_) => {
                if !event.is_pending_for(SystemType::RenderSystem) {
                    event_manager.add_pending(event, SystemType::RenderSystem);
                }
                Err(format!(""))
            },
            _ => Err(format!(""))
        }
    }

    fn free_objects(&mut self, id: EntityID, mesh: &Option<MeshType>) -> Result <(), String> {
        if let Some(mesh_type) = mesh {
            match mesh_type {
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

            Ok(())
        } else {
            Err("".to_string())
        }
    }

    fn remove_entity(&mut self, id: EntityID, event: Event, event_manager: &mut EventManager, world: &mut World) -> Result<(), String> {
        let mesh_label = match world.components.renderables[id].as_mut() {
            Some(comp) => &comp.mesh_label,
            None => return Err("".to_string()),
        };

        match world.resources.try_read() {
            Ok(res) if res.mesh_data.contains_key(mesh_label) => self.free_objects(id, &res.mesh_data[mesh_label].mesh_type),
            Err(_) => {
                event_manager.add_pending(event, SystemType::RenderSystem);
                Err("".to_string())
            }
            _ => Err("".to_string())
        }
    }

    fn handle_system_events(&mut self, event_manager: &mut EventManager, world: &mut World) {

        for event in event_manager.get_engine_events().clone().into_iter() {
            match event.event_type {
                EventType::EntityCreated(id) => {
                    self.handle_entity_creation(id, event, event_manager, world);
                }

                EventType::EntityRemoved(id) => {
                    self.remove_entity(id, event, event_manager, world);
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

    fn init(&mut self, world: &mut World, engine: &mut Engine) -> Result<(), String> {
        let shader_name = SCREEN_SHADER!();

        let resources = &world.resources.read().unwrap().shaders;
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
            let instant = Instant::now();
            self.draw_entities(engine, world);
            draw_ui(engine, &mut engine.log_manager);
            let time = instant.elapsed().as_millis();

            let log_manager = &mut engine.log_manager;
            log_manager.add_log((
                format!("render_system"), 
                Box::new(RenderSystemLogObject{text: format!("RENDER_SYSTEM: {} ms", time)})
            ));
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
unsafe fn draw_ui(engine: *mut Engine, log_manager: *mut LogManager) {
    let eng = engine.as_mut().unwrap();
    let ui_frame_buffer = eng.ui_render_object.as_ref().unwrap().frame_buffer;

    gl::BindFramebuffer(gl::FRAMEBUFFER, ui_frame_buffer);
    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    gl::Enable(gl::DEPTH_TEST);

    log_manager.as_ref().unwrap().update_ui_logs_view(eng);
    //TODO(Teddy) Do all the buffer clearing operations

    if let Some(view) = &mut eng.get_ui_tree().unwrap().root {
        match view.update(engine.as_ref().unwrap()) {
            Ok(_) => (),
            Err(_) => println!("A view failed to update"),
        }
    }
}
