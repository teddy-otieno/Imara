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
mod ui;
mod utils;

use std::time::Instant;

use glfw::Context;
use nalgebra::Vector3;

use crate::core::{camera_behaviour, load_fonts, Engine, EventManager};
use crate::ui::ui::View;
use editor::editor::{update_editor, Editor};
use game_world::world::World;
use gl_bindings::Display;
use systems::physics::Physics;
use systems::render_system::Renderer;
use systems::system::{System, Systems};
use ui::ui::{
    init_ui, Orientation, SimpleUIContainer, TextView, ViewContainer, ViewDimens, ViewPosition,
};

fn main() {
    let display = gl_bindings::init_gl_window_context((1000, 600), "Imara");
    run(display);
}

fn run(display: Display) {
    let fonts = unsafe { load_fonts(12) }.expect("Failed to load messages");

    let mut engine = Engine::new(display, fonts);
    let mut event_manager = EventManager::new();
    let ev_pointer: *const EventManager = &event_manager;
    let mut world = World::new(ev_pointer as *mut EventManager);
    let mut systems = Systems::new();
    let mut editor = Editor::new();

    init_ui(&mut engine, &mut world).unwrap();

    let mut text_view = Box::new(TextView::new(
        String::from("text_1").into_boxed_str(),
        String::from("Hello world"),
        ViewPosition { x: 10, y: 10 },
        1.0,
        10,
    ));
    let mut text_view_1 = Box::new(TextView::new(
        String::from("text_2").into_boxed_str(),
        String::from("Hello world"),
        ViewPosition { x: 100, y: 100 },
        1.0,
        10,
    ));
    let mut text_view_2 = Box::new(TextView::new(
        String::from("text_3").into_boxed_str(),
        String::from("Hello world"),
        ViewPosition { x: 200, y: 200 },
        1.0,
        10,
    ));

    text_view.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
        view_ref.background_color = [0.0, 0.4, 0.0]
    }));
    text_view.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
        view_ref.background_color = [0.2, 0.2, 0.0]
    }));

    text_view_1.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(0.0, 1.0, 0.0));
    }));

    text_view_1.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
    }));

    text_view_2.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(0.0, 1.0, 0.0));
    }));
    text_view_2.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
        let view_ref = view.as_mut().unwrap();
        view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
    }));

    let simple_container_view_dimensions = Some(ViewDimens::new(1000, 600));
    let simpe_container_position = Some(ViewPosition::new(500, 300));
    let mut simple_container = Box::new(SimpleUIContainer::new(
        String::from("simple_container").into_boxed_str(),
        simple_container_view_dimensions,
        simpe_container_position,
        Orientation::Vertical,
    ));

    simple_container.add_child(text_view);
    simple_container.add_child(text_view_1);
    simple_container.add_child(text_view_2);

    // simple_container.remove_child("text_1");

    engine.ui_tree.root = Some(simple_container);
    // add_ui_element(&mut engine, text_view);
    // add_ui_element(&mut engine, text_view_1);
    // add_ui_element(&mut engine, text_view_2);

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
            println!("{} : Frames per second", ticks);
            // println!("Avg. Frame Time {} ns", frame_time / ticks);
            frame_time = 0;
            ticks = 0;
        }
    }
}

//TODO(teddy) Get rid of this junk code later

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
