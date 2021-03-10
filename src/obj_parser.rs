use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use nalgebra::{Point2, Point3, Point4};

#[derive(Debug)]
pub enum ParseError {
    IOError(std::io::Error),
    Internal(String),
}

pub trait Obj {
    fn from(data: Data) -> Self;
}

#[derive(Debug)]
pub struct NormalObj {
    pub vertices: Vec<Point4<f32>>,
    pub normals: Vec<Point3<f32>>,
    pub indices: Vec<u32>,
}
#[derive(Debug)]
pub struct TexturedObj {
    pub vertices: Vec<Point4<f32>>,
    pub normals: Vec<Point3<f32>>,
    pub text_cords: Vec<Point2<f32>>,
    pub indices: Vec<u32>,
}

impl Obj for NormalObj {
    fn from(data: Data) -> Self {
        Self {
            vertices: data.vertices,
            normals: data.normals,
            indices: data.indices,
        }
    }
}

impl Obj for TexturedObj {
    fn from(data: Data) -> Self {
        Self {
            vertices: data.vertices,
            normals: data.normals,
            indices: data.indices,
            text_cords: data.text_cords,
        }
    }
}

#[derive(Debug)]
pub struct Data {
    vertices: Vec<Point4<f32>>,
    text_cords: Vec<Point2<f32>>,
    normals: Vec<Point3<f32>>,
    indices: Vec<u32>,
}

pub fn load_obj<T>(source: &str) -> Result<T, ParseError>
where
    T: Obj,
{
    let obj_file = File::open(source).expect(format!("Unable to open file {}", source).as_str());
    let file_content = BufReader::new(obj_file);

    let data = match parse_file(file_content) {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    // println!("{:#?}", data);
    Ok(T::from(data))
}

fn parse_file<T: BufRead>(file_content: T) -> Result<Data, ParseError> {
    let mut vertices: Vec<Point4<f32>> = vec![];
    let mut raw_texture_cords: Vec<Point2<f32>> = vec![];
    let mut raw_normals: Vec<Point3<f32>> = vec![];
    let mut raw_indices: Vec<[u32; 3]> = vec![];

    let lex_result = lex(file_content, |prefix, args| {
        match prefix {
            //Vertices
            "v" => match *args.as_slice() {
                [x, y, z, w] => {
                    let vertice = Point4::new(
                        x.parse().unwrap(),
                        y.parse().unwrap(),
                        z.parse().unwrap(),
                        w.parse().unwrap(),
                    );

                    vertices.push(vertice);
                    Ok(())
                }

                [x, y, z] => {
                    let vertice = Point4::new(
                        x.parse().unwrap(),
                        y.parse().unwrap(),
                        z.parse().unwrap(),
                        1.0,
                    );

                    vertices.push(vertice);
                    Ok(())
                }

                _ => Err(ParseError::Internal(String::from(
                    "V: Invalid number of arguments",
                ))),
            },

            //Texture Coordinates
            "vt" => match *args.as_slice() {
                [x, y] => {
                    raw_texture_cords.push(Point2::new(x.parse().unwrap(), y.parse().unwrap()));
                    Ok(())
                }

                _ => Err(ParseError::Internal(String::from(
                    "VT: Invalid number of arguments",
                ))),
            },

            //Normals
            "vn" => match *args.as_slice() {
                [x, y, z] => {
                    raw_normals.push(Point3::new(
                        x.parse().unwrap(),
                        y.parse().unwrap(),
                        z.parse().unwrap(),
                    ));
                    Ok(())
                }
                _ => Err(ParseError::Internal(String::from("VN: Invalid arguments"))),
            },

            //Indices
            "f" => {
                let indices = args
                    .iter()
                    .map(|s| s.split("/").collect())
                    .collect::<Vec<Vec<&str>>>();

                for index in indices {
                    match *index.as_slice() {
                        [vertex, text_cord, normal] => {
                            let parse = |s: &str| {
                                if s.is_empty() {
                                    0
                                } else {
                                    s.parse().unwrap()
                                }
                            };

                            raw_indices.push([parse(vertex), parse(text_cord), parse(normal)]);
                        }

                        _ => {
                            return Err(ParseError::Internal(String::from("F: Invalid arguments")))
                        }
                    }
                }
                Ok(())
            }
            // _ => Err(ParseError::Internal(String::from("Invalid prefix")))
            _ => Ok(()),
        }
    });

    if let Err(error) = lex_result {
        return Err(error);
    }

    // dbg!(&raw_normals);
    //Process the mesh
    let mut text_cords: Vec<Point2<f32>> = vec![Point2::origin(); vertices.len()];
    let mut normals: Vec<Point3<f32>> = vec![Point3::origin(); vertices.len()];
    let mut indices: Vec<u32> = vec![];

    for indice in raw_indices {
        indices.push(indice[0]);

        if indice[1] != 0 && !text_cords.is_empty() {
            text_cords[(indice[0] - 1) as usize] = raw_texture_cords[(indice[1] - 1) as usize];
        }

        if indice[2] != 0 {
            normals[(indice[0] - 1) as usize] = raw_normals[(indice[2] - 1) as usize]
        }
    }

    Ok(Data {
        vertices,
        text_cords,
        normals,
        indices: indices.into_iter().map(|x| x - 1).collect(),
    })
}

fn lex<T, F>(content: T, mut callback: F) -> Result<(), ParseError>
where
    T: BufRead,
    F: FnMut(&str, Vec<&str>) -> Result<(), ParseError>,
{
    let mut multi_line = String::new();
    for line in content.lines() {
        let line_content = match line {
            Ok(l) => l,
            Err(err) => return Err(ParseError::IOError(err)),
        };

        if line_content.starts_with("#") {
            //Ignore a comment
            continue;
        }

        if line_content.ends_with('\\') {
            multi_line.push_str(&line_content[..line_content.len() - 1]);
            multi_line.push(' ');
            continue;
        }

        multi_line.push_str(&*line_content.into_boxed_str());

        let mut words = multi_line.split_whitespace();

        let prefix = words.next().unwrap();
        let args = words.map(|s| s).collect::<Vec<&str>>();

        callback(prefix, args)?;
        multi_line.clear();
    }

    Ok(())
}
