use std::vec::Vec;
use std::mem::size_of;
use std::ptr;
use std::collections::HashMap;
use gl;
use gl::types::*;
use glm;
use num_traits::identities::One;
use wavefront_obj::{ obj, mtl };
use util::assert_no_gl_error;

use shaders;
use util;

lazy_static! {
    static ref DEFAULT_MATERIAL: mtl::Material = mtl::Material {
        name: "default material".to_owned(),
        specular_coefficient: 0.0,
        color_ambient: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_diffuse: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_specular: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_emissive: Option::None,
        optical_density: Option::None,
        alpha: 1.0,
        illumination: mtl::Illumination::Ambient,
        uv_map: Option::None,
    };
}

fn flatten_color(color: mtl::Color) -> Vec<f32> {
    return vec![color.r as f32, color.g as f32, color.b as f32];
}

struct RenderableChunk<'a> {
    material: &'a mtl::Material,
    vertices: Vec<obj::Vertex>,
    triangles: Vec<obj::Shape>,
}

pub struct RenderableObject<'a> {
    object: obj::Object,
    material: Option<HashMap<String, mtl::Material>>,
    program: &'a shaders::Program,
    initialized: bool,
    vao: GLuint,
    indices: i32,
}

impl <'a> RenderableObject<'a> {
    pub fn new(object: obj::Object, material: Option<HashMap<String, mtl::Material>>, program: &shaders::Program) -> RenderableObject {
        RenderableObject {
            object: object,
            material: material,
            program: program,
            initialized: false,
            vao: 0,
            indices: 0,
        }
    }

    pub fn render(&mut self, view: glm::Mat4, projection: glm::Mat4) {
        self.lazy_init();

        let model = glm::Mat4::one();
        let mvp = projection * view * model;
        let mvp_array = util::arrayify_mat4(mvp);

        unsafe {
            gl::UseProgram(self.program.name);
            gl::UniformMatrix4fv(self.program.get_uniform("mvp"), 1, gl::FALSE, &*mvp_array as *const f32);
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.indices, gl::UNSIGNED_INT, ptr::null());
        }
    }

    fn lazy_init(&mut self) {
        if !self.initialized {
            self.initialized = true;

            let mut renderable_chunks: Vec<RenderableChunk> = self
                .object
                .geometry
                .iter()
                .map(|g| {
                    let material;
                    // Nested options are pretty sad. :/
                    if self.material.is_some() && g.material_name.is_some() {
                        material = self.material.as_ref().unwrap().get(g.material_name.as_ref().unwrap()).unwrap_or(&DEFAULT_MATERIAL);
                    } else {
                        material = &DEFAULT_MATERIAL;
                    }
                    RenderableChunk {
                        material: material,
                        vertices: vec![],
                        triangles: vec![],
                    }
                    // g
                    //     .shapes
                    //     .iter()
                    //     .flat_map(move |s| {
                    //         let g1 = g.clone();
                    //         match s.primitive {
                    //             obj::Primitive::Triangle(
                    //                 (v1, _, _),
                    //                 (v2, _, _),
                    //                 (v3, _, _),
                    //             ) => {
                    //                 // we'll want to duplicate vertices that appear in two different materials, eventually
                    //                 let color = match g1.material_name {
                    //                     Some(name) => material.get(&name).unwrap().color_ambient,
                    //                     None => DEFAULT_COLOR,
                    //                 };
                    //                 vec![(v1, color), (v2, color), (v3, color)].into_iter()
                    //             },
                    //             _ => { panic!("got non-triangle primitive"); },
                    //         }
                    //     })
                })
                .collect();

            unsafe {
                // create the VAO for this object
                gl::GenVertexArrays(1, &mut self.vao);
                gl::BindVertexArray(self.vao);
                assert_no_gl_error();
            }

            {
                let flattened_vertices: Vec<GLfloat> = self
                    .object
                    .vertices
                    .iter()
                    .flat_map(|v| vec![ v.x as GLfloat, v.y as GLfloat, v.z as GLfloat ].into_iter())
                    .collect();

                unsafe {
                    // set current array data buffer and fill it with vertex position data
                    let mut v_position_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut v_position_buffer);
                    gl::BindBuffer(gl::ARRAY_BUFFER, v_position_buffer);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (flattened_vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                        flattened_vertices.as_ptr() as *const _,
                        gl::STATIC_DRAW);
                    assert_no_gl_error();

                    // find the location of the position argument in the shader and tell OpenGL that the currently bound array points to it in triples
                    let position_attrib_location = self.program.get_attrib("in_Position") as GLuint;
                    gl::EnableVertexAttribArray(position_attrib_location);
                    gl::VertexAttribPointer(
                        position_attrib_location,
                        3,
                        gl::FLOAT,
                        gl::FALSE as GLboolean,
                        0,
                        ptr::null());
                    assert_no_gl_error();
                }
            }

            // match self.material {
            //     Some(ref material) => {
            //         let mut ordered_vertex_colors: Vec<(usize, mtl::Color)> = self
            //             .object
            //             .geometry
            //             .iter()
            //             .flat_map(|g| {
            //                 g
            //                     .shapes
            //                     .iter()
            //                     .flat_map(move |s| {
            //                         let g1 = g.clone();
            //                         match s.primitive {
            //                             obj::Primitive::Triangle(
            //                                 (v1, _, _),
            //                                 (v2, _, _),
            //                                 (v3, _, _),
            //                             ) => {
            //                                 // we'll want to duplicate vertices that appear in two different materials, eventually
            //                                 let color = match g1.material_name {
            //                                     Some(name) => material.get(&name).unwrap().color_ambient,
            //                                     None => DEFAULT_COLOR,
            //                                 };
            //                                 vec![(v1, color), (v2, color), (v3, color)].into_iter()
            //                             },
            //                             _ => { panic!("got non-triangle primitive"); },
            //                         }
            //                     })
            //             })
            //             .collect::<Vec<(usize, mtl::Color)>>();

            //         ordered_vertex_colors.sort_by(|t0, t1| t0.0.cmp(&t1.0));

            //         let flattened_vertex_colors: Vec<GLfloat> = ordered_vertex_colors
            //             .iter()
            //             .flat_map(|t| flatten_color(t.1).into_iter())
            //             .collect();

            //         unsafe {
            //             // set current array data buffer and fill it with vertex "color" data
            //             let mut v_color_buffer: GLuint = 0;
            //             gl::GenBuffers(1, &mut v_color_buffer);
            //             gl::BindBuffer(gl::ARRAY_BUFFER, v_color_buffer);
            //             gl::BufferData(
            //                 gl::ARRAY_BUFFER,
            //                 (flattened_vertex_colors.len() * size_of::<GLfloat>()) as GLsizeiptr,
            //                 flattened_vertex_colors.as_ptr() as *const _,
            //                 gl::STATIC_DRAW);

            //             let fragment_color_attrib = self.program.get_attrib("in_FragmentColor") as GLuint;
            //             gl::EnableVertexAttribArray(fragment_color_attrib);
            //             gl::VertexAttribPointer(
            //                 fragment_color_attrib,
            //                 3,
            //                 gl::FLOAT,
            //                 gl::FALSE as GLboolean,
            //                 0,
            //                 ptr::null());
            //         }
            //     },
            //     None => {
            //         // TODO: Some sane default scheme?
            //         info!("no material provided, will render default");
            //     }
            // }

            {
                let vertex_indices: Vec<u32> = self
                    .object
                    .geometry
                    .iter()
                    .flat_map(|g| {
                        g
                            .shapes
                            .iter()
                            .flat_map(|s| {
                                match s.primitive {
                                    obj::Primitive::Triangle(
                                        (v1, _, _),
                                        (v2, _, _),
                                        (v3, _, _),
                                    ) => {
                                        vec![v1 as u32, v2 as u32, v3 as u32].into_iter()
                                    },
                                    _ => { panic!("got non-triangle primitive"); },
                                }
                            })
                    })
                    .collect();

                self.indices = vertex_indices.len() as i32;

                unsafe {
                    // lastly, tell OpenGL about the indices (that must be correlated for all buffers!)
                    let mut index_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut index_buffer);
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer);
                    gl::BufferData(
                        gl::ELEMENT_ARRAY_BUFFER,
                        (vertex_indices.len() * size_of::<u32>()) as GLsizeiptr,
                        vertex_indices.as_ptr() as *const _,
                        gl::STATIC_DRAW);
                    assert_no_gl_error();
                }
            }
        }
    }
}
