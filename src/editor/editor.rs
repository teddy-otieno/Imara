use std::fs;
use std::path::Path;

use glfw::MouseButton;
use nalgebra::{Matrix4, Point3, Vector3};
use nphysics3d::material::{BasicMaterial, MaterialHandle};
use nphysics3d::object::{BodyStatus, DefaultBodyHandle, DefaultColliderHandle};

use crate::{core::{
    mouse_clicked, CastRayDat, CastedRay, Engine, Event, EventManager, EventType,
    ViewPortDimensions,
}, ui::ui::View};
use crate::game_world::components::*;
use crate::game_world::world::{AssetSource, ObjType, OBJ_ASSETS_DIR};
use crate::game_world::world::{ResourceResult, World};
use crate::ui::ui::{
    Orientation, SimpleUIContainer, TextView, UITree, ViewContainer, ViewPosition,
};
use crate::utils::compute_world_space_to_screen_space;

pub struct Editor {
    pub ui_tree: UITree,
    pub shader_label: String,
    pub selected_entity: Option<usize>,
}

impl Editor {
    pub fn new(shader_label: String) -> Self {
        Self {
            ui_tree: UITree::new(),
            shader_label,
            selected_entity: None,
        }
    }

    pub fn init_editor_ui(&mut self, engine: &mut Engine, world: &mut World) {
        let simpe_container_position = ViewPosition::new(0, 0);
        let mut simple_container = Box::new(SimpleUIContainer::new(
            String::from("simple_container").into_boxed_str(),
            None,
            simpe_container_position,
            Orientation::Vertical,
            10,
            1.0
        ));

        let log_container = Box::new(SimpleUIContainer::new(
            String::from("ui_log").into_boxed_str(),
            None,
            ViewPosition::new(0, 0),
            Orientation::Vertical,
            10,
            1.0
        ));

        let mut text_view = Box::new(TextView::new(
            String::from("text_1").into_boxed_str(),
            String::from("Objects"),
            ViewPosition { x: 10, y: 10 },
            1.0,
            10,
        ));
        text_view.get_view_object_mut().background_color = Box::new([0.6, 0.2, 0.2]);

        text_view.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
            let view_ref = view.as_mut().unwrap();
            view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
            view_ref.get_view_object_mut().background_color = Box::new([0.0, 0.4, 0.0])
        }));
        text_view.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
            let view_ref = view.as_mut().unwrap();
            view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
            view_ref.get_view_object_mut().background_color = Box::new([0.2, 0.2, 0.0])
        }));

        simple_container.add_child(text_view);

        let objs = load_list_of_obj_assets();

        for (i, name) in objs.into_iter().filter(|s| s.ends_with(".obj")).enumerate() {
            let mut asset_name_text_view = Box::new(TextView::new(
                format!("text_{}", name).into_boxed_str(),
                name.clone(),
                ViewPosition { x: 10, y: 10 },
                1.0,
                10,
            ));

            asset_name_text_view.get_view_object_mut().background_color = Box::new([0.2, 0.2, 0.2]);

            asset_name_text_view.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
                let view_ref = view.as_mut().unwrap();
                view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
                view_ref.get_view_object_mut().background_color = Box::new([0.0, 0.4, 0.0]);
            }));
            asset_name_text_view.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
                let view_ref = view.as_mut().unwrap();
                view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
                view_ref.get_view_object_mut().background_color = Box::new([0.2, 0.2, 0.2]);
            }));

            let shader = self.shader_label.clone();
            let world_ptr: *mut World = world;
            let engine_ptr: *mut Engine = engine;
            let self_ptr: *mut Self = self;

            asset_name_text_view.on_click = Some(Box::new(move |view: *mut TextView| {
                let self_ref = unsafe { self_ptr.as_mut().unwrap() };
                //Move the camera closer to the entity

                println!("ON CLICK CLICKED");
                let id = create_entity(world_ptr, engine_ptr, name.clone(), shader.clone());
                self_ref.selected_entity = Some(id);
            }));

            simple_container.add_child(asset_name_text_view);
        }

        let mut save_world = TextView::new("save".to_owned().into_boxed_str(), format!("Save world"), ViewPosition::zerod(), 1.0, 10);

        let world_ptr: *mut World = world;
        save_world.on_click = Some(Box::new( move |view: *mut TextView| unsafe {
            let world_ref = world_ptr.as_mut().unwrap();
            world_ref.save();
        }));

        let sep = TextView::new("logs".to_owned().into_boxed_str(), format!("------------------------------------------------------------------------"), ViewPosition::zerod(), 1.0, 10);
        let text_view = TextView::new("logs".to_owned().into_boxed_str(), format!("Logs"), ViewPosition::zerod(), 1.0, 10);
        simple_container.add_child(Box::new(save_world));
        simple_container.add_child(Box::new(sep));
        simple_container.add_child(Box::new(text_view));
        simple_container.add_child(log_container);
        self.ui_tree.root = Some(simple_container);
    }
}

static mut COUNTER: f32 = 0.0;

fn create_entity(
    world_ptr: *mut World,
    engine_ptr: *mut Engine,
    file_path: String,
    shader_label: String,
) -> usize {
    let world = unsafe { world_ptr.as_mut().unwrap() };
    let engine = unsafe { engine_ptr.as_mut().unwrap() };

    let id = world.create_entity();

    let words: Vec<&str> = file_path.split("/").collect();
    let mesh_id = match world.resources.add_resource(
        AssetSource::Mesh(ObjType::Normal, String::from(words[words.len() - 1])),
        true,
    ) {
        ResourceResult::Mesh(id) => id,
        _ => unreachable!(),
    };

    world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_label));
    world.components.positionable[id] = Some(TransformComponent::new(
        Vector3::new(0.0 + (5.0 * unsafe { COUNTER }), 0.0, 10.0),
        Vector3::new(0.0, 1.0, 0.0),
        1.0,
    ));

    world.components.physics[id] = Some(PhysicsComponent::new(
        1.0,
        false,
        BodyStatus::Static,
        Vector3::new(0.0, 0.0, 0.0),
        MaterialHandle::new(BasicMaterial::new(0.3, 0.8)),
    ));
    // world.components.highlightable[id] = Some(HighlightComponent{color: [0.0, 0.0, 0.0]});

    println!("{:#?}", world.components.positionable[id]);
    println!("Entity {} succesffuly loaded", file_path);
    unsafe { COUNTER += 1.0 };

    id
}

fn load_list_of_obj_assets() -> Vec<String> {
    let mut output = vec![];
    let directory = fs::read_dir(dbg!(Path::new(OBJ_ASSETS_DIR))).unwrap();
    for entry in directory {
        let dir = entry.unwrap().path();
        output.push(String::from(dir.to_str().unwrap()));
    }

    output
}

pub fn update_editor(
    editor: &mut Editor,
    engine: &mut Engine,
    world: &mut World,
    event_manager: &mut EventManager,
) {
    //TODO(teddy)
    //1. get the selected_entity and add higlight component
    //

    if mouse_clicked(engine, &MouseButton::Button3) {
        println!("Button event captured");
    }

    if let Some(id) = editor.selected_entity {
        let component = world.components.positionable[id].as_ref().unwrap();

        let camera = &engine.camera;
        let ViewPortDimensions{  width, height } = camera.view_port;

        //TODO(Teddy) fix tomorrow
        let result = compute_world_space_to_screen_space(
            ViewPortDimensions { width, height },
            &component.position.translation.vector,
            &camera.view(),
            &camera.perspective(),
        );

        if (result.x > 0.0 && result.x < width as f32)
            && (result.y > 0.0 && result.y < height as f32)
        {
            //TODO(teddy):
        }

        //Note(teddy) Draw a quad at that position
        handle_world_events(editor, engine, world, event_manager);
        unsafe { draw_transform_guides(&Vector3::new(0.0, 0.0, 0.0)) };
    }
}

fn handle_world_events(
    editor: &mut Editor,
    engine: &Engine,
    world: &mut World,
    event_manager: &mut EventManager,
) {
    for event in event_manager.get_engine_events() {
        match event.event_type {
            EventType::RayCasted(CastedRay { id: _, entity }) if entity.is_some() => {
                world.components.highlightable[entity.unwrap()] = Some(HighlightComponent {
                    color: [0.0, 1.0, 0.0],
                });
            }
            _ => (),
        }
    }
}

unsafe fn draw_transform_guides(position: &Vector3<f32>) {}
