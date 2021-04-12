use std::fs;

use nalgebra::{Vector3, Matrix4, Point3};

use crate::utils::{compute_world_space_to_screen_space};
use crate::game_world::components::{RenderComponent, TransformComponent, HighlightComponent};
use crate::game_world::world::World;
use crate::core::{Engine, Event, ViewPortDimensions, EventManager, CastRayDat, CastedRay};
use crate::game_world::world::{AssetSource, ObjType};
use crate::ui::ui::{
    Orientation, SimpleUIContainer, TextView, UITree, ViewContainer, ViewPosition,
};

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

            let shader = self.shader_label.clone();
            let world_ptr: *mut World = world;
            let engine_ptr: *mut Engine = engine;
            let self_ptr: *mut Self = self;

            asset_name_text_view.on_click = Some(Box::new(move |view: *mut TextView| {

                let self_ref = unsafe {self_ptr.as_mut().unwrap()};
                //Move the camera closer to the entity
                let id = create_entity(world_ptr, engine_ptr, name.clone(), shader.clone());
                self_ref.selected_entity = Some(id);
            }));

            simple_container.add_child(asset_name_text_view)
        }

        self.ui_tree.root = Some(simple_container);
    }
}

static mut counter: f32 = 0.0;

fn create_entity(world_ptr: *mut World, engine_ptr: *mut Engine, file_path: String, shader_label: String) -> usize {
    let world = unsafe { world_ptr.as_mut().unwrap() };
    let engine = unsafe { engine_ptr.as_mut().unwrap() };

    let id = world.create_entity();

    let words: Vec<&str> = file_path.split("\\").collect();
    let mesh_id = world.resources.add_resource(AssetSource::Mesh(
        ObjType::Normal,
        String::from(words[words.len() - 1]),
    ));

    world.components.renderables[id] = Some(RenderComponent::new(mesh_id.unwrap(), shader_label));
    world.components.positionable[id] = Some(TransformComponent::new(
        Vector3::new(0.0 + (5.0 * unsafe {counter}), 0.0, 10.0),
        Vector3::new(0.0, 1.0, 0.0),
        1.0,
    ));

    world.components.highlightable[id] = Some(HighlightComponent{color: [0.0, 0.0, 0.0]});

    println!("{:#?}", world.components.positionable[id]);
    println!("Entity {} succesffuly loaded", file_path);
    unsafe {counter += 1.0};

    id
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

pub fn update_editor(editor: &mut Editor, engine: &mut Engine, world: &mut World, event_manager: &EventManager) {

    //TODO(teddy) 
    //1. get the selected_entity and add higlight component
    //

    if let Some(id) = editor.selected_entity {
        let component = world.components.positionable[id].as_ref().unwrap();

        let camera = &engine.camera;
        let (width, height) = camera.view_port;


        //TODO(Teddy) fix tomorrow
        let result = compute_world_space_to_screen_space(
            ViewPortDimensions{width, height}, 
            &component.position.translation.vector, 
            &camera.view(),
            &camera.perspective()
            );


        if (result.x > 0.0 && result.x < width as f32) && (result.y > 0.0 && result.y < height as f32) {
            //TODO(teddy):
        }

        //Note(teddy) Draw a quad at that position
        handle_world_events(editor, engine, world, event_manager);
        unsafe { draw_transform_guides(&Vector3::new(0.0,0.0,0.0)) };
    }
}

fn handle_world_events(editor: &mut Editor, engine: &Engine, world: &mut World, event_manager: &EventManager) {
    for event in event_manager.get_engine_events() {
        match event {
            Event::RayCasted(CastedRay { id: _, entity }) if entity.is_some() => {
                world.components.highlightable[entity.unwrap()] = Some(HighlightComponent {
                    color: [0.0, 1.0, 0.0],
                });
            }
            _ => (),
        }
    }
}

unsafe fn draw_transform_guides(position: &Vector3<f32>) {

}
