use std::fmt;
use nalgebra::{Matrix4, Vector3, Vector4};
use std::collections::LinkedList;

use crate::core::ViewPortDimensions;

pub fn get_at_index<T>(list: &LinkedList<T>, index: usize) -> Option<&T> {
    for (i, el) in list.iter().enumerate() {
        if i == index {
            return Some(el);
        }
    }
    None
}

#[derive(Debug, Copy, Clone)]
pub struct Cords<T: fmt::Debug> {
    pub(crate) x: T,
    pub(crate) y: T,
}

//TODO(Teddy) Maybe will include the object's local vector space for rotation of markers
//This function will be used to generate cordinates for screen markers
#[inline]
pub fn compute_world_space_to_screen_space(
    screen_dimensions: ViewPortDimensions,
    object_world_position: &Vector3<f32>,
    view_matrix: &Matrix4<f32>,
    perspective_matrix: &Matrix4<f32>,
) -> Cords<f32> {
    let position_to_vec4 = Vector4::new(
        object_world_position.x,
        object_world_position.y,
        object_world_position.z,
        1.0,
    );

    let mut world_position_mapped_to_screen_position: Vector4<f32> = (perspective_matrix * view_matrix) * position_to_vec4;
    world_position_mapped_to_screen_position = world_position_mapped_to_screen_position;

    let screen_cords = world_position_mapped_to_screen_position.xy() / world_position_mapped_to_screen_position.z;

    let ViewPortDimensions { width, height } = screen_dimensions;

    let x = screen_cords.x;
    let y = screen_cords.y;

    let cord_x = (x + 1.0) * (width as f32 / 2.0);
    let cord_y = (y - 1.0) * (height as f32 / -2.0);

    Cords{x: cord_x, y: cord_y}
}
