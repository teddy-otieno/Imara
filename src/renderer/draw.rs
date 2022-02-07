use std::convert::TryInto;
use std::ffi::c_void;
use std::ffi::CString;

use nalgebra::{Matrix4, Point3, Point4, Vector3};

use crate::core::{Camera, Engine, Light, ViewPortDimensions};
use crate::game_world::components::{TransformComponent};
use crate::game_world::world::World;
use crate::obj_parser::{NormalObj, TexturedObj};
use crate::utils::get_at_index;

#[derive(Debug)]
pub enum DrawError {
    ShaderNotFound(String),
    ShaderNotAvailable(String),
}

#[repr(C)]
#[derive(Debug)]
pub struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[repr(C)]
#[derive(Debug)]
pub struct TexturedVertex {
    position: Vec4,
    normal: Vec3,
    text_cords: [f32; 2],
}

#[repr(C)]
#[derive(Debug)]
pub struct NormalVertex {
    position: Vec3,
    normal: Vec3,
}

#[derive(Debug)]
pub struct RenderObject {
    pub vertex_buffer: u32,
    pub element_buffer: u32,
    pub vertex_array_object: u32,
    pub size_of_elements: i32,
}

pub unsafe fn init_normal_object(object: &NormalObj) -> RenderObject {
    let (vertices, indices) = process_normal_mesh(&object);

    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;

    //dbg!(&vertices);
    //dbg!(&indices);

    //panic!();
    gl::GenVertexArrays(1, &mut vao);
    gl::GenBuffers(1, &mut vbo);
    gl::GenBuffers(1, &mut ebo);

    gl::BindVertexArray(vao);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        (vertices.len() * std::mem::size_of::<NormalVertex>()) as isize,
        vertices.as_ptr().cast(),
        gl::STATIC_DRAW,
    );

    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        (indices.len() * std::mem::size_of::<u32>()) as isize,
        indices.as_ptr().cast(),
        gl::STATIC_DRAW,
    );

    gl::EnableVertexAttribArray(0);

    gl::VertexAttribPointer(
        0,
        3,
        gl::FLOAT,
        gl::FALSE,
        std::mem::size_of::<NormalVertex>().try_into().unwrap(),
        0 as *const c_void,
    );

    gl::EnableVertexAttribArray(1);
    gl::VertexAttribPointer(
        1,
        3,
        gl::FLOAT,
        gl::FALSE,
        std::mem::size_of::<NormalVertex>().try_into().unwrap(),
        offset_of!(NormalVertex, normal) as *const c_void,
    );

    //Note(teddy) break the vertex array binding
    gl::BindVertexArray(0);

    RenderObject {
        vertex_array_object: vao,
        vertex_buffer: vbo,
        element_buffer: ebo,
        size_of_elements: indices.len() as i32,
    }
}

pub unsafe fn init_textured_object(object: &TexturedObj) -> RenderObject {
    let (_vertices, _indices) = process_textured_mesh(&object);
    unimplemented!()
}

pub fn remove_normal_object(_id: usize, _object: RenderObject) {}

pub fn remove_textured_object(_id: usize, _object: RenderObject) {}

fn process_textured_mesh(obj: &TexturedObj) -> (Vec<TexturedVertex>, Vec<u32>) {
    let point4toslice = |point: &Point4<f32>| Vec4 {
        x: point.x,
        y: point.y,
        z: point.z,
        w: point.w,
    };

    let point3_to_slice = |point: &Point3<f32>| Vec3 {
        x: point.x,
        y: point.y,
        z: point.z,
    };

    assert!(obj.vertices.len() == obj.normals.len() && obj.vertices.len() == obj.text_cords.len(), true);

    let mut output_vertices = vec![];

    for i in 0..obj.vertices.len() {
        let new_vertex = TexturedVertex {
            position: point4toslice(&obj.vertices[i]),
            normal: point3_to_slice(&obj.normals[i]),
            text_cords: [obj.text_cords[i].x, obj.text_cords[i].y],
        };

        output_vertices.push(new_vertex);
    }

    (output_vertices, obj.indices.clone())
}

fn process_normal_mesh(obj: &NormalObj) -> (Vec<NormalVertex>, Vec<u32>) {
    let point4toslice = |point: &Point4<f32>| Vec3 {
        x: point.x,
        y: point.y,
        z: point.z,
    };

    let point3_to_slice = |point: &Point3<f32>| Vec3 {
        x: point.x,
        y: point.y,
        z: point.z,
    };

    assert!(obj.vertices.len() == obj.normals.len(), true);
    let mut output_vertices = vec![];

    for i in 0..obj.vertices.len() {
        let new_vertex = NormalVertex {
            position: point4toslice(&obj.vertices[i]),
            normal: point3_to_slice(&obj.normals[i]),
        };

        output_vertices.push(new_vertex);
    }

    (output_vertices, obj.indices.clone())
}

pub unsafe fn draw_normal_object<T>(
    world: &World,
    shader_label: &String,
    camera: &Camera,
    object: &RenderObject,
    transform: &TransformComponent,
    light: &Light,
    draw_params: T,
) -> Result<(), DrawError>
where
    T: FnOnce(),
{
    let resources = &world.resources.read().unwrap().shaders;

    let shader = match resources.get(shader_label) {
        Some(id) => {
            if let Some(shader_id) = id {
                *shader_id
            } else {
                //Shader is not available skip
                return Err(DrawError::ShaderNotAvailable(shader_label.clone()));
            }
        }
        None => return Err(DrawError::ShaderNotFound(shader_label.clone())),
    };

    let view_matrix: Matrix4<f32> = camera.view();
    let perspective_matrix: Matrix4<f32> = camera.perspective();
    let scale = transform.scale;
    let scale_matrix = Matrix4::new(
        scale, 0.0, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, 0.0, 1.0,
    );
    let model_matrix: Matrix4<f32> = transform.position.to_homogeneous() * scale_matrix;

    //TODO(teddy) precompute the transformation matrices then send

    let uniform_name = CString::new("view").unwrap();
    let perspective_name = CString::new("pers").unwrap();
    let model_name = CString::new("model").unwrap();
    let dir_light_direction_name = CString::new("dir_light.direction").unwrap();
    let dir_light_color_name = CString::new("dir_light.color").unwrap();
    let object_color_name = CString::new("color").unwrap();

    let view_mat_location = gl::GetUniformLocation(shader, uniform_name.as_ptr());
    let pers_mat_location = gl::GetUniformLocation(shader, perspective_name.as_ptr());
    let model_mat_location = gl::GetUniformLocation(shader, model_name.as_ptr());
    let dir_light_location = gl::GetUniformLocation(shader, dir_light_direction_name.as_ptr());
    let dir_light_color_location = gl::GetUniformLocation(shader, dir_light_color_name.as_ptr());
    let object_color_location = gl::GetUniformLocation(shader, object_color_name.as_ptr());

    gl::UseProgram(shader);

    gl::UniformMatrix4fv(
        view_mat_location,
        1,
        gl::FALSE,
        view_matrix.as_slice().as_ptr(),
    );
    gl::UniformMatrix4fv(
        pers_mat_location,
        1,
        gl::FALSE,
        perspective_matrix.as_slice().as_ptr(),
    );
    gl::UniformMatrix4fv(
        model_mat_location,
        1,
        gl::FALSE,
        model_matrix.as_slice().as_ptr(),
    );

    gl::Uniform3fv(dir_light_location, 1, light.direction.as_ptr());
    gl::Uniform3fv(dir_light_color_location, 1, light.color.as_ptr());

    //TODO(use objects color)
    let default_color = [0.7, 0.7, 0.7];
    gl::Uniform3fv(object_color_location, 1, default_color.as_ptr());
    gl::BindVertexArray(object.vertex_array_object);

    draw_params();
    gl::DrawElements(
        gl::TRIANGLES,
        object.size_of_elements,
        gl::UNSIGNED_INT,
        0 as *const c_void,
    );
    gl::BindVertexArray(0);
    Ok(())
}

//TODO(teddy) Remove the scale, A wrapper function will be use to load the specified font sizes
//Replace the scale with the font's pixel height
pub unsafe fn draw_text(
    text_vao: u32,
    text_vbo: u32,
    engine: &Engine,
    shader_id: u32,
    text: &str,
    mut x: f32,
    mut y: f32,
    scale: f32,
    color: &Vector3<f32>,
) {
    gl::Enable(gl::BLEND);
    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

    gl::UseProgram(shader_id);

    //Note(teddy) Since opengl's origin cords are at the bottom. We decrement the y with font_size
    //to accurately map the font cords to the screen
    let ViewPortDimensions { width, height } = engine.camera.view_port;

    y = height as f32 - y - engine.font_face.font_size as f32;

    let projection: Matrix4<f32> =
        Matrix4::new_orthographic(0.0, width as f32, 0.0, height as f32, -1.0, 1.0);

    //dbg!(projection);
    let projection_uniform_name = CString::new("projection").unwrap();
    let text_color_name = CString::new("text_color").unwrap();

    let projection_uniform_location =
        gl::GetUniformLocation(shader_id, projection_uniform_name.as_ptr());

    let text_color_uniform_location = gl::GetUniformLocation(shader_id, text_color_name.as_ptr());

    gl::UniformMatrix4fv(
        projection_uniform_location,
        1,
        gl::FALSE,
        projection.as_slice().as_ptr(),
    );
    gl::Uniform3f(text_color_uniform_location, color.x, color.y, color.z);
    gl::ActiveTexture(gl::TEXTURE0);
    gl::BindVertexArray(text_vao);

    for c in text.chars() {
        let character = &engine.font_face.chars[&c];
        let xposition = x + character.bearing.x as f32 * scale;
        let yposition = y - (character.size.y - character.bearing.y) as f32 * scale;

        let w: f32 = character.size.x as f32 * scale;
        let h: f32 = character.size.y as f32 * scale;

        let vertices: [[f32; 4]; 6] = [
            [xposition, yposition + h, 0.0, 0.0],
            [xposition, yposition, 0.0, 1.0],
            [xposition + w, yposition, 1.0, 1.0],
            [xposition, yposition + h, 0.0, 0.0],
            [xposition + w, yposition, 1.0, 1.0],
            [xposition + w, yposition + h, 1.0, 0.0],
        ];

        gl::BindTexture(gl::TEXTURE_2D, character.texture);

        gl::BindBuffer(gl::ARRAY_BUFFER, text_vbo);
        gl::BufferSubData(
            gl::ARRAY_BUFFER,
            0,
            (vertices.len() * 4 * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as *const c_void,
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);

        x += (character.advance >> 6) as f32 * scale;
    }

    gl::BindVertexArray(0);
    gl::BindTexture(gl::TEXTURE_2D, 0);
}

//Note(teddy) Draw any quad
pub unsafe fn draw_quad(
    quad_vao: u32,
    quad_vbo: u32,
    z_position: f32,
    (x, y): (f32, f32),
    (h, w): (f32, f32),
) {
    let vertices: [[f32; 3]; 6] = [
        [x, y + h, z_position],
        [x, y, z_position],
        [x + w, y, z_position],
        [x, y + h, z_position],
        [x + w, y, z_position],
        [x + w, y + h, z_position],
    ];

    gl::BindVertexArray(quad_vao);
    gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
    gl::BufferSubData(
        gl::ARRAY_BUFFER,
        0,
        (vertices.len() * 3 * std::mem::size_of::<f32>()) as isize,
        vertices.as_ptr() as *const c_void,
    );

    //Note(teddy) Unbinding the quad_vbo
    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::DrawArrays(gl::TRIANGLES, 0, 6);

    gl::BindVertexArray(0);
}

pub unsafe fn draw_quad_with_default_shader(
    engine: &Engine,
    quad_vao: u32,
    quad_vbo: u32,
    z_position: f32,
    (x, y): (f32, f32),
    (h, w): (f32, f32),
    color: &[f32; 3],
) {
    use crate::ui::ui::UI_QUAD_SHADER_ID;

    let program = UI_QUAD_SHADER_ID;
    gl::UseProgram(program);

    let ViewPortDimensions {width, height} = engine.camera.view_port;

    let projection: Matrix4<f32> =
        Matrix4::new_orthographic(0.0, width as f32, 0.0, height as f32, -1.0, 1.0);
    let color_uniform_name = CString::new("quad_color").unwrap();
    let projection_name = CString::new("projection").unwrap();

    let color_uniform_location = gl::GetUniformLocation(program, color_uniform_name.as_ptr());
    let projection_location = gl::GetUniformLocation(program, projection_name.as_ptr());

    gl::Uniform3fv(color_uniform_location, 1, color.as_ptr());
    gl::UniformMatrix4fv(
        projection_location,
        1,
        gl::FALSE,
        projection.as_slice().as_ptr(),
    );

    gl::Enable(gl::BLEND);
    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(gl::LESS);

    draw_quad(
        quad_vao,
        quad_vbo,
        z_position,
        (x, height as f32 - y),
        (h, w),
    );
}
