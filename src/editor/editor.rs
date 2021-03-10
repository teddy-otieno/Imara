use crate::game_world::world::World;

pub struct Editor;

impl Editor {
    pub fn new() -> Self {
        Self {}
    }
}

pub fn update_editor(_editor: &mut Editor, _world: &mut World) {}
