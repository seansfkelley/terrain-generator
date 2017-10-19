use std::vec::Vec;
use std::mem::size_of;
use std::ptr;
use std::collections::{ HashMap, HashSet };
use multimap;
use gl;
use gl::types::*;
use glm;
use num_traits::identities::One;
use wavefront_obj::{ obj, mtl };
use util::assert_no_gl_error;

use shaders;
use util;

trait Flattenable {
    fn flatten(&self) -> Vec<GLfloat>;
}

impl Flattenable for obj::Vertex {
    fn flatten(&self) -> Vec<GLfloat> {
        vec![self.x as f32, self.y as f32, self.z as f32]
    }
}

impl Flattenable for mtl::Color {
    fn flatten(&self) -> Vec<GLfloat> {
        vec![self.r as f32, self.g as f32, self.b as f32]
    }
}

impl Flattenable for glm::Vec3 {
    fn flatten(&self) -> Vec<GLfloat> {
        vec![self.x, self.y, self.z]
    }
}

lazy_static! {
    static ref DEFAULT_MATERIAL: mtl::Material = mtl::Material {
        name: "default material".to_owned(),
        specular_coefficient: 0.0,
        color_ambient: mtl::Color { r: 0.5, g: 0.5, b: 0.5 },
        color_diffuse: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_specular: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_emissive: Option::None,
        optical_density: Option::None,
        alpha: 1.0,
        illumination: mtl::Illumination::Ambient,
        uv_map: Option::None,
    };
}

#[derive(Debug)]
struct RenderableChunk<'a> {
    material: &'a mtl::Material,
    vertices: Vec<glm::Vec3>,
    normals: Vec<glm::Vec3>,
    triangles: Vec<obj::Primitive>,
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
        let model_view_projection = projection * view * model;

        let v_array = util::arrayify_mat4(view);
        let m_array = util::arrayify_mat4(model);
        let mvp_array = util::arrayify_mat4(model_view_projection);
        let light_position = [2f32, 3f32, -7f32];

        unsafe {
            gl::UseProgram(self.program.name);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatMvp"), 1, gl::FALSE, &*mvp_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatV"), 1, gl::FALSE, &*v_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatM"), 1, gl::FALSE, &*m_array as *const f32);
            gl::Uniform3f(self.program.get_uniform("u_LightPosition_WorldSpace"), light_position[0], light_position[1], light_position[2]);
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.indices, gl::UNSIGNED_INT, ptr::null());
        }
    }

    fn split_into_renderable_chunks(&self) -> Vec<RenderableChunk> {
        let converted_vertices: Vec<glm::Vec3> = self
            .object
            .vertices
            .iter()
            .map(|v| glm::vec3(v.x as f32, v.y as f32, v.z as f32))
            .collect();

        let mut vertex_normal_pairs: Vec<(usize, glm::Vec3)> = self
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
                                let normal = glm::normalize(glm::cross(converted_vertices[v2] - converted_vertices[v1], converted_vertices[v3] - converted_vertices[v1]));
                                vec![(v1, normal), (v2, normal), (v3, normal)]
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
            })
            // pseudocode
            .collect::<multimap::MultiMap<usize, glm::Vec3>>()
            .iter_all()
            .map(|(v_index, normals)| {
                (*v_index, normals.iter().fold(glm::vec3(0.0, 0.0, 0.0), |acc, &n| acc + n))
            })
            .collect();

        vertex_normal_pairs.sort_by(|p1, p2| p1.0.cmp(&p2.0));

        // TODO: Sometimes, normals will be givn to us in the input file.
        let all_vertex_normals: Vec<glm::Vec3> = vertex_normal_pairs
            .iter()
            .map(|p| p.1)
            .collect();

        self
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

                let mut new_index_counter: usize = 0;
                let old_to_new_index_mapping: HashMap<usize, usize> = g.shapes
                    .iter()
                    .flat_map(|s| {
                        match s.primitive {
                            obj::Primitive::Triangle(
                                (v1, _, _),
                                (v2, _, _),
                                (v3, _, _),
                            ) => {
                                vec![v1, v2, v3].into_iter()
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
                    .collect::<HashSet<usize>>()
                    .iter()
                    .map(|i| {
                        let indices = (*i, new_index_counter);
                        new_index_counter += 1;
                        indices
                    })
                    .collect();

                let mut sorted_reverse_mapping: Vec<(usize, usize)> = old_to_new_index_mapping
                    .keys()
                    .map(|i| (old_to_new_index_mapping[i], *i))
                    .collect();

                sorted_reverse_mapping.sort_by(|p1, p2| p1.0.cmp(&p2.0));
                let new_to_old_index_mapping: Vec<usize> = sorted_reverse_mapping
                    .iter()
                    .map(|p| p.1)
                    .collect();

                let vertices: Vec<glm::Vec3> = new_to_old_index_mapping
                    .iter()
                    .map(|i| converted_vertices[*i])
                    .collect();

                let normals: Vec<glm::Vec3> = new_to_old_index_mapping
                    .iter()
                    .map(|i| all_vertex_normals[*i])
                    .collect();

                let triangles: Vec<obj::Primitive> = g.shapes
                    .iter()
                    .map(|s| {
                        match s.primitive {
                            obj::Primitive::Triangle(
                                (v1, t1, n1),
                                (v2, t2, n2),
                                (v3, t3, n3),
                            ) => {
                                let (new_v1, new_v2, new_v3) = (
                                    old_to_new_index_mapping[&v1],
                                    old_to_new_index_mapping[&v2],
                                    old_to_new_index_mapping[&v3],
                                );
                                obj::Primitive::Triangle {
                                    0: (new_v1, t1, n1),
                                    1: (new_v2, t2, n2),
                                    2: (new_v3, t3, n3),
                                }
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
                    .collect();

                RenderableChunk {
                    material: material,
                    vertices: vertices,
                    normals: normals,
                    triangles: triangles,
                }
            })
            .collect()
    }

    fn lazy_init(&mut self) {
        if !self.initialized {
            self.initialized = true;

            unsafe {
                // create the VAO for this object
                gl::GenVertexArrays(1, &mut self.vao);
                gl::BindVertexArray(self.vao);
                assert_no_gl_error();
            }

            let triangle_count: i32;

            {
                let chunks = self.split_into_renderable_chunks();

                let mut flattened_vertices: Vec<GLfloat> = vec![];
                let mut flattened_normals: Vec<GLfloat> = vec![];
                let mut flattened_color_ambient: Vec<GLfloat> = vec![];
                let mut flattened_color_diffuse: Vec<GLfloat> = vec![];
                let mut indices: Vec<u32> = vec![];

                let mut offset = 0;
                for c in chunks {
                    for i in 0..c.vertices.len() {
                        let mut v = c.vertices[i].flatten();
                        flattened_vertices.append(&mut v);

                        let mut n = c.normals[i].flatten();
                        flattened_normals.append(&mut n);

                        let mut c_ambient = c.material.color_ambient.flatten();
                        flattened_color_ambient.append(&mut c_ambient);

                        // TODO: Respect illumination type, which may not specify diffuse.
                        let mut c_diffuse = c.material.color_diffuse.flatten();
                        flattened_color_diffuse.append(&mut c_diffuse);
                    }

                    for t in c.triangles {
                        match t {
                            obj::Primitive::Triangle(
                                (v1, _, _),
                                (v2, _, _),
                                (v3, _, _),
                            ) => {
                                indices.push(v1 as u32 + offset);
                                indices.push(v2 as u32 + offset);
                                indices.push(v3 as u32 + offset);
                            },
                            _ => { panic!("unexpected non-triangle in triangle list"); },
                        }
                    }

                    offset += c.vertices.len() as u32;
                }

                // assign vertex position data
                unsafe {
                    let mut v_position_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut v_position_buffer);
                    gl::BindBuffer(gl::ARRAY_BUFFER, v_position_buffer);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (flattened_vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                        flattened_vertices.as_ptr() as *const _,
                        gl::STATIC_DRAW);
                    assert_no_gl_error();

                    let position_attrib_location = self.program.get_attrib("in_VertexPosition") as GLuint;
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

                // assign vertex normal data
                unsafe {
                    let mut v_normal_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut v_normal_buffer);
                    gl::BindBuffer(gl::ARRAY_BUFFER, v_normal_buffer);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (flattened_vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                        flattened_vertices.as_ptr() as *const _,
                        gl::STATIC_DRAW);
                    assert_no_gl_error();

                    let position_attrib_location = self.program.get_attrib("in_VertexNormal") as GLuint;
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

                // assign vertex ambient color
                unsafe {
                    let mut v_color_ambient_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut v_color_ambient_buffer);
                    gl::BindBuffer(gl::ARRAY_BUFFER, v_color_ambient_buffer);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (flattened_color_ambient.len() * size_of::<GLfloat>()) as GLsizeiptr,
                        flattened_color_ambient.as_ptr() as *const _,
                        gl::STATIC_DRAW);

                    let fragment_color_attrib = self.program.get_attrib("in_ColorAmbient") as GLuint;
                    gl::EnableVertexAttribArray(fragment_color_attrib);
                    gl::VertexAttribPointer(
                        fragment_color_attrib,
                        3,
                        gl::FLOAT,
                        gl::FALSE as GLboolean,
                        0,
                        ptr::null());
                }

                // assign vertex diffuse color
                unsafe {
                    let mut v_color_diffuse_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut v_color_diffuse_buffer);
                    gl::BindBuffer(gl::ARRAY_BUFFER, v_color_diffuse_buffer);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (flattened_color_diffuse.len() * size_of::<GLfloat>()) as GLsizeiptr,
                        flattened_color_diffuse.as_ptr() as *const _,
                        gl::STATIC_DRAW);

                    let fragment_color_attrib = self.program.get_attrib("in_ColorDiffuse") as GLuint;
                    gl::EnableVertexAttribArray(fragment_color_attrib);
                    gl::VertexAttribPointer(
                        fragment_color_attrib,
                        3,
                        gl::FLOAT,
                        gl::FALSE as GLboolean,
                        0,
                        ptr::null());
                }

                // specify vertex index mapping
                unsafe {
                    let mut index_buffer: GLuint = 0;
                    gl::GenBuffers(1, &mut index_buffer);
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer);
                    gl::BufferData(
                        gl::ELEMENT_ARRAY_BUFFER,
                        (indices.len() * size_of::<u32>()) as GLsizeiptr,
                        indices.as_ptr() as *const _,
                        gl::STATIC_DRAW);
                    assert_no_gl_error();
                }

                triangle_count = indices.len() as i32;
            }

            self.indices = triangle_count;
        }
    }
}
