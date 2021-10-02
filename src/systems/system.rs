use crate::core::{Engine, EventManager};
use crate::game_world::world::World;
use std::collections::LinkedList;

#[derive(Debug, Clone, PartialEq)]
pub enum SystemType {
    RenderSystem,
    PhysicsSystem,
}

pub trait System {
    //WE might do some event subscriptions
    fn update(
        &mut self,
        world: &mut World,
        event_manager: &mut EventManager,
        engine: &mut Engine,
        delta_time: f32,
    );

    fn init(&mut self, world: &mut World, engine: &mut Engine) -> Result<(), String> {
        Ok(())
    }


    fn name(&self) -> String;
}

pub struct Systems {
    pub systems: LinkedList<Box<dyn System>>,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            systems: LinkedList::new(),
        }
    }
}
