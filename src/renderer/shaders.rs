use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::ptr::null;

#[derive(Debug)]
pub enum ShaderError {
    VertexError(String),
    FragmentError(String),
    GeometryError(String),
}

pub unsafe fn create_shader(
    vertex: String,
    fragment: String,
    geometric: Option<String>,
) -> Result<u32, ShaderError> {
    let get_contents = |source: String| {
        let mut contents = String::new();
        let mut file = match File::open(source) {
            Ok(f) => f,
            Err(err) => return Err(err),
        };
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(err) => return Err(err),
        };

        Ok(CString::new(contents).expect("Unable to load C String"))
    };

    let vertex_string = get_contents(vertex).unwrap();
    let fragment_string = get_contents(fragment).unwrap();

    let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
    gl::ShaderSource(
        vertex_shader,
        1,
        &(vertex_string.as_ptr() as *const i8) as *const *const i8,
        null(),
    );
    gl::CompileShader(vertex_shader);
    let mut sucess: i32 = 0;
    let mut info_log: Vec<i8> = vec![0; 1028];
    gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut sucess as *mut i32);
    if sucess == 0 {
        gl::GetShaderInfoLog(
            vertex_shader,
            1028,
            null::<i32>() as *mut i32,
            info_log.as_mut_ptr(),
        );

        let message = info_log
            .iter()
            .filter(|s| **s != 0)
            .map(|s| *s as u8)
            .collect();
        return Err(ShaderError::VertexError(
            String::from_utf8(message).unwrap(),
        ));
    }

    let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
    gl::ShaderSource(
        fragment_shader,
        1,
        &(fragment_string.as_ptr() as *const i8) as *const *const i8,
        null(),
    );
    gl::CompileShader(fragment_shader);
    gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut sucess as *mut i32);
    if sucess == 0 {
        gl::GetShaderInfoLog(
            fragment_shader,
            1028,
            null::<i32>() as *mut i32,
            info_log.as_mut_ptr() as *mut i8,
        );
        let message = info_log
            .iter()
            .filter(|s| **s != 0)
            .map(|s| *s as u8)
            .collect();
        return Err(ShaderError::FragmentError(
            String::from_utf8(message).unwrap(),
        ));
    }

    let geo_shader = match geometric {
        Some(source) => {
            let geo_string = get_contents(source).unwrap();
            let geo_shader = gl::CreateShader(gl::GEOMETRY_SHADER);
            gl::ShaderSource(
                geo_shader,
                1,
                &(geo_string.as_ptr() as *const i8) as *const *const i8,
                null(),
            );
            gl::CompileShader(geo_shader);
            gl::GetShaderiv(geo_shader, gl::COMPILE_STATUS, &mut sucess as *mut i32);
            if sucess == 0 {
                gl::GetShaderInfoLog(
                    geo_shader,
                    1028,
                    null::<i32> as *mut i32,
                    info_log.as_mut_ptr() as *mut i8,
                );

                let message = info_log
                    .iter()
                    .filter(|s| **s != 0)
                    .map(|s| *s as u8)
                    .collect();
                return Err(ShaderError::GeometryError(
                    String::from_utf8(message).unwrap(),
                ));
            }

            geo_shader
        }

        None => 0,
    };

    let shader_program = gl::CreateProgram();
    gl::AttachShader(shader_program, vertex_shader);
    gl::AttachShader(shader_program, fragment_shader);
    if geo_shader > 0 {
        gl::AttachShader(shader_program, geo_shader);
    }
    gl::LinkProgram(shader_program);

    gl::DeleteShader(vertex_shader);
    gl::DeleteShader(fragment_shader);
    if geo_shader > 0 {
        gl::DeleteShader(geo_shader);
    }

    Ok(shader_program)
}
