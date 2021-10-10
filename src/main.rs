extern crate gl;
extern crate glfw;
extern crate nalgebra;
extern crate nphysics3d;
#[macro_use]
extern crate memoffset;
extern crate freetype;
extern crate serde_json;

#[macro_use]
mod core;
mod logs;
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

use crate::core::{camera_behaviour, load_fonts, Engine, EventManager};
use editor::editor::{update_editor, Editor};
use game_world::world::{AssetSource, World};
use gl_bindings::Display;
use systems::physics::Physics;
use systems::render_system::Renderer;
use logs::Logable;

#[macro_use]
use systems::system::{System, Systems};
use ui::ui::init_ui;

fn main() {
    let display = gl_bindings::init_gl_window_context((1000, 600), "Imara");
    run(display);
}

macro_rules! default_shader {
    () => {
        String::from("default")
    };
}


fn run(display: Display) {
    let fonts = unsafe { load_fonts(12).unwrap() };

    let mut engine = Engine::new(display, fonts);
    let mut event_manager = EventManager::new();
    let mut world = World::new(&mut event_manager, &mut engine.log_manager);
    let mut systems = Systems::new();

    world.resources.add_resource(
        AssetSource::Shader(
            default_shader!(),
            String::from("vert.glsl"),
            String::from("frag.glsl"),
            None,
        ),
        false,
    );

    world.resources.add_resource(
        AssetSource::Shader(
            String::from("highlight_shader"),
            String::from("vert.glsl"),
            String::from("border_frag.glsl"),
            None,
        ),
        false,
    );


    world.resources.add_resource(
        AssetSource::Shader(
            SCREEN_SHADER!(),
            String::from("screen_vert.glsl"),
            String::from("screen_frag.glsl"),
            None
            ),
        false,
        );

    init_ui(&mut engine, &mut world).unwrap();

    //TODO(teddy) Issue will happen
    let mut editor = Editor::new(default_shader!());
    editor.init_editor_ui(&mut engine, &mut world);
    engine.ui_tree = Some(&mut editor.ui_tree);

    let render_system: Box<dyn System> = Box::new(Renderer::new());
    let physics_system: Box<dyn System> = Box::new(Physics::new());

    systems.systems.push_front(render_system);
    systems.systems.push_front(physics_system);

    {
        for system in systems.systems.iter_mut() {
            system.init(&mut world, &mut engine).unwrap();
        }
    }
    // I have to create and load a mesh
    //world.components.(RenderComponent::new())
    let mut frame_time: u128 = 0;
    let mut ticks: u128 = 0;

    unsafe {
        gl::Enable(gl::STENCIL_TEST);
        gl::StencilFunc(gl::NOTEQUAL, 1, 0xFF);
        gl::StencilOp(gl::KEEP, gl::KEEP, gl::REPLACE);
    }

    engine.log_manager.add_log((String::from("main"), Box::new(MainLoopLogObject{text: String::new()})));

    while !engine.display.window.should_close() {
        let time = Instant::now();
        engine.display.glfw.poll_events();
        event_manager.handle_events(glfw::flush_messages(&engine.display.events_receiver));
        engine.update(&mut event_manager);

        camera_behaviour(&mut engine);
        for system in systems.systems.iter_mut() {
            system.update(&mut world, &mut event_manager, &mut engine, 16.0);
        }

        update_editor(&mut editor, &mut engine, &mut world, &mut event_manager);

        engine.display.window.swap_buffers();
        event_manager.clear();
        frame_time += time.elapsed().as_nanos();
        ticks += 1;

        if frame_time >= 1000000000 {
            //println!("{} : Frames per second", ticks);
            let main_log = format!("Avg. Frame Time {} ms", frame_time / (ticks * 1_000_000));
            engine.log_manager.add_log((String::from("main"), Box::new(MainLoopLogObject{text: main_log})));
            frame_time = 0;
            ticks = 0;
        }
    }
}

struct MainLoopLogObject {
    text: String
}

impl Logable for MainLoopLogObject {
    fn to_string(&self) -> String  { self.text.clone() }
}
