extern crate gl;
extern crate glfw;
extern crate nalgebra;
extern crate nphysics3d;
#[macro_use]
extern crate memoffset;
extern crate freetype;
extern crate serde_json;

mod core;
mod editor;
mod game_world;
mod gl_bindings;
mod obj_parser;
mod renderer;
mod systems;
mod utils;
mod ui;

use std::time::Instant;

use glfw::Context;

use crate::core::{camera_behaviour, load_fonts, Engine, EventManager};
use editor::editor::{update_editor, Editor};
use game_world::world::{AssetSource};
use game_world::world::World;
use gl_bindings::Display;
use systems::physics::Physics;
use systems::render_system::Renderer;
use systems::system::{System, Systems};
use ui::ui::{init_ui, add_ui_element, TextView};
use crate::utils::Cords;

fn main() {
    let display = gl_bindings::init_gl_window_context((1000, 600), "Imara");
    run(display);
}

fn run(display: Display) {
    let fonts = unsafe { load_fonts() }.expect("Failed to load messages");

    let mut engine = Engine::new(display, fonts);
    let mut event_manager = EventManager::new();
    let ev_pointer: *const EventManager = &event_manager;
    let mut world = World::new(ev_pointer as *mut EventManager);
    let mut systems = Systems::new();
    let mut editor = Editor::new();


    // if let Err(_err) = load_level("level1", &mut world) {
    //     //Will do something
    //     println!("Level not found");
    // }


    /*
    {
        let id = world.create_entity();
        let mesh_id = world.resources.add_resource(AssetSource::Mesh((
            ObjType::Normal,
            String::from("landscape.obj"),
        )));

        //TODO(teddy) add a push method
        world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_id));
        world.components.positionable[id] =
            Some(TransformComponent::new(Vector3::new(0.0, -5.0, 0.0), 1.0));

        world.components.physics[id] = Some(PhysicsComponent::new(
            0.0,
            false,
            BodyStatus::Static,
            Vector3::zeros(),
            MaterialHandle::new(BasicMaterial::new(1.0, 0.8)),
        ));
    }

    {
        let id = world.create_entity();
        let mesh_id = world.resources.add_resource(AssetSource::Mesh((
            ObjType::Normal,
            String::from("sphere.obj"),
        )));

        //TODO(teddy) add a push method
        world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_id));
        world.components.positionable[id] = Some(TransformComponent::new(
            Vector3::new(100.0, 400.0, 0.0),
            1.0,
        ));

        world.components.physics[id] = Some(PhysicsComponent::new(
            1.0,
            false,
            BodyStatus::Dynamic,
            Vector3::zeros(),
            MaterialHandle::new(BasicMaterial::new(0.8, 0.8)),
        ));
    }

    {
        let id = world.create_entity();
        let mesh_id = world.resources.add_resource(AssetSource::Mesh((
            ObjType::Normal,
            String::from("cube.obj"),
        )));

        //TODO(teddy) add a push method
        world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_id));
        world.components.positionable[id] =
            Some(TransformComponent::new(Vector3::new(0.0, 400.0, 20.0), 1.0));

        world.components.physics[id] = Some(PhysicsComponent::new(
            10.4,
            false,
            BodyStatus::Dynamic,
            Vector3::zeros(),
            MaterialHandle::new(BasicMaterial::new(0.8, 0.8)),
        ));
    }

    {
        let id = world.create_entity();
        let mesh_id = world.resources.add_resource(AssetSource::Mesh((
            ObjType::Normal,
            String::from("suzanne.obj"),
        )));

        //TODO(teddy) add a push method
        world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_id));
        world.components.positionable[id] = Some(TransformComponent::new(
            Vector3::new(5.0, 400.0, -10.0),
            1.0,
        ));
        world.components.physics[id] = Some(PhysicsComponent::new(
            8.0,
            false,
            BodyStatus::Dynamic,
            Vector3::zeros(),
            MaterialHandle::new(BasicMaterial::new(1.0, 0.8)),
        ));
    }

    */

    init_ui(&mut engine, &mut world);

    let text_view = Box::new(TextView::new(String::from("Hello world"), Cords { x: 10.0, y: 10.0 }, 1.0, None));
    let text_view_1 = Box::new(TextView::new(String::from("Another hellow rodl"), Cords { x: 100.0, y: 100.0 }, 1.0, None));
    let text_view_2 = Box::new(TextView::new(String::from("This ui is working okay"), Cords { x: 200.0, y: 200.0 }, 1.0, None));
    add_ui_element(&mut engine, text_view);
    add_ui_element(&mut engine, text_view_1);
    add_ui_element(&mut engine, text_view_2);
    let render_system: Box<dyn System> = Box::new(Renderer::new());
    let physics_system: Box<dyn System> = Box::new(Physics::new());

    systems.systems.push_front(render_system);
    systems.systems.push_front(physics_system);
    // I have to create and load a mesh
    //world.components.(RenderComponent::new())
    let mut frame_time: u128 = 0;
    let mut ticks: u128 = 0;

    while !engine.display.window.should_close() {
        let time = Instant::now();
        engine.display.glfw.poll_events();
        event_manager.handle_events(glfw::flush_messages(&engine.display.events_receiver));
        engine.update(&mut world, &mut event_manager);

        update_editor(&mut editor, &mut world);

        camera_behaviour(&mut engine);
        for system in systems.systems.iter_mut() {
            system.update(&mut world, &mut event_manager, &mut engine, 16.0);
            //println!("Duration FOR {} System = {}", &system.name(), time.elapsed().as_millis());
        }

        engine.display.window.swap_buffers();

        event_manager.clear();
        frame_time += time.elapsed().as_nanos();
        ticks += 1;

        if frame_time >= 1000000000 {
            println!("Avg. Frame Time {} ns", frame_time / ticks);
            frame_time = 0;
            ticks = 0;
        }
    }
}
