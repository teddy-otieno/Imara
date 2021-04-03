use std::fs;

use nalgebra::Vector3;

use crate::game_world::world::World;
use crate::ui::ui::{
    Orientation, SimpleUIContainer, TextView, UITree, ViewContainer, ViewPosition,
};
use crate::game_world::world::{ObjType, AssetSource};
use crate::game_world::components::{RenderComponent, TransformComponent};

pub struct Editor {
    pub ui_tree: UITree,
    pub shader_id: usize,
}

impl Editor {
    pub fn new(shader_id: usize) -> Self {
        Self {
            ui_tree: UITree::new(),
            shader_id: shader_id
        }
    }

    pub fn init_editor_ui(&mut self, world: &mut World) {
        let simpe_container_position = Some(ViewPosition::new(0, 0));
        let mut simple_container = Box::new(SimpleUIContainer::new(
            String::from("simple_container").into_boxed_str(),
            None,
            simpe_container_position,
            Orientation::Vertical,
        ));

        let mut text_view = Box::new(TextView::new(
            String::from("text_1").into_boxed_str(),
            String::from("Objects"),
            ViewPosition { x: 10, y: 10 },
            1.0,
            10,
        ));
        text_view.background_color = [0.6, 0.2, 0.2];

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

        simple_container.add_child(text_view);

        let objs = load_list_of_obj_assets();

        for name in objs.into_iter().filter(|s| s.ends_with(".obj")) {
            let mut asset_name_text_view = Box::new(TextView::new(
                String::from("text_1").into_boxed_str(),
                name.clone(),
                ViewPosition { x: 10, y: 10 },
                1.0,
                10,
            ));

            asset_name_text_view.background_color = [0.2, 0.2, 0.2];

            asset_name_text_view.on_hover = Some(Box::new(|view: *mut TextView| unsafe {
                let view_ref = view.as_mut().unwrap();
                view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
                view_ref.background_color = [0.0, 0.4, 0.0];
            }));
            asset_name_text_view.on_mouse_leave = Some(Box::new(|view: *mut TextView| unsafe {
                let view_ref = view.as_mut().unwrap();
                view_ref.color = Some(Vector3::new(1.0, 1.0, 1.0));
                view_ref.background_color = [0.2, 0.2, 0.2];
            }));

            let shader = self.shader_id;
            let world_ptr: *mut World = world;
            asset_name_text_view.on_click = Some(Box::new(move |view: *mut TextView| {
               create_entity(world_ptr, name.clone(), shader);
            }));

            simple_container.add_child(asset_name_text_view)
        }

        self.ui_tree.root = Some(simple_container);
    }
}

fn create_entity(world_ptr: *mut World, file_path: String, shader_id: usize) {
    let world = unsafe {world_ptr.as_mut().unwrap()};

    let id = world.create_entity();

    let words: Vec<&str> = file_path.split("\\").collect();
    let mesh_id = world.resources.add_resource(AssetSource::Mesh(ObjType::Normal, String::from(words[words.len() - 1])));

    world.components.renderables[id] = Some(RenderComponent::new(mesh_id, shader_id));
    world.components.positionable[id] =
        Some(TransformComponent::new(Vector3::new(0.0, -5.0, 0.0), Vector3::new(0.0, 1.0, 0.0), 1.0));


    println!("Entity {} succesffuly loaded", file_path);
}

fn load_list_of_obj_assets() -> Vec<String> {
    let mut output = vec![];
    let directory = fs::read_dir(".\\assets\\objects").unwrap();
    for entry in directory {
        let dir = entry.unwrap().path();
        output.push(String::from(dir.to_str().unwrap()));
    }

    output
}

pub fn update_editor(_editor: &mut Editor, _world: &mut World) {}
