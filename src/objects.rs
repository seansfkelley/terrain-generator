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
    fn component_count() -> u32;
    fn append_components_to(&self, vector: &mut Vec<GLfloat>);
}

impl Flattenable for GLfloat {
    fn component_count() -> u32 { 1 }

    fn append_components_to(&self, vector: &mut Vec<GLfloat>) {
        vector.push(*self);
    }
}

impl Flattenable for glm::Vec3 {
    fn component_count()  -> u32 { 3 }

    fn append_components_to(&self, vector: &mut Vec<GLfloat>) {
        vector.push(self.x);
        vector.push(self.y);
        vector.push(self.z);
    }
}

impl Flattenable for mtl::Color {
    fn component_count()  -> u32 { 3 }

    fn append_components_to(&self, vector: &mut Vec<GLfloat>) {
        vector.push(self.r as GLfloat);
        vector.push(self.g as GLfloat);
        vector.push(self.b as GLfloat);
    }
}

lazy_static! {
    static ref DEFAULT_MATERIAL: mtl::Material = mtl::Material {
        name: "default material".to_owned(),
        specular_coefficient: 0.0,
        color_ambient: mtl::Color { r: 0.5, g: 0.5, b: 0.5 },
        color_diffuse: mtl::Color { r: 0.5, g: 0.5, b: 0.5 },
        color_specular: mtl::Color { r: 1.0, g: 1.0, b: 1.0 },
        color_emissive: Option::None,
        optical_density: Option::None,
        alpha: 1.0,
        illumination: mtl::Illumination::AmbientDiffuse,
        uv_map: Option::None,
    };
}

static BLACK: mtl::Color = mtl::Color { r: 0.0, g: 0.0, b: 0.0 };

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
    loaded: bool,
    vao: GLuint,
    triangle_count: i32,
}

impl <'a> RenderableObject<'a> {
    pub fn new(object: obj::Object, material: Option<HashMap<String, mtl::Material>>, program: &shaders::Program) -> RenderableObject {
        RenderableObject {
            object: object,
            material: material,
            program: program,
            loaded: false,
            vao: 0,
            triangle_count: 0,
        }
    }

    pub fn render(&mut self, view: glm::Mat4, projection: glm::Mat4) {
        self.lazy_load_buffers();

        let model = glm::Mat4::one();
        let model_view_projection = projection * view * model;

        let v_array = util::arrayify_mat4(view);
        let m_array = util::arrayify_mat4(model);
        let mvp_array = util::arrayify_mat4(model_view_projection);
        let light_position = [3f32, 4f32, 7f32];
        let light_color = [1f32, 1f32, 1f32];
        let light_power = 50f32;

        unsafe {
            gl::UseProgram(self.program.name);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatMvp"), 1, gl::FALSE, &*mvp_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatV"), 1, gl::FALSE, &*v_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatM"), 1, gl::FALSE, &*m_array as *const f32);
            gl::Uniform3f(self.program.get_uniform("u_LightPosition_WorldSpace"), light_position[0], light_position[1], light_position[2]);
            gl::Uniform3f(self.program.get_uniform("u_LightColor"), light_color[0], light_color[1], light_color[2]);
            gl::Uniform1f(self.program.get_uniform("u_LightPower"), light_power);
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.triangle_count, gl::UNSIGNED_INT, ptr::null());
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

    fn create_array_buffer<T: Flattenable>(&self, attribute_name: &str, items: Vec<T>) {
        let mut flattened_items: Vec<GLfloat> = vec![];
        for i in items {
            i.append_components_to(&mut flattened_items);
        }

        unsafe {
            let mut array_buffer_name: GLuint = 0;
            gl::GenBuffers(1, &mut array_buffer_name);
            gl::BindBuffer(gl::ARRAY_BUFFER, array_buffer_name);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (flattened_items.len() * size_of::<GLfloat>()) as GLsizeiptr,
                flattened_items.as_ptr() as *const _,
                gl::STATIC_DRAW);
            assert_no_gl_error();

            let attribute_location = self.program.get_attrib(attribute_name) as GLuint;
            gl::EnableVertexAttribArray(attribute_location);
            gl::VertexAttribPointer(
                attribute_location,
                T::component_count() as GLint,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                0,
                ptr::null());
            assert_no_gl_error();
        }
    }

    fn create_element_array_buffer(&self, indices: Vec<u32>) {
        unsafe {
            let mut index_buffer_name: GLuint = 0;
            gl::GenBuffers(1, &mut index_buffer_name);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer_name);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * size_of::<u32>()) as GLsizeiptr,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW);
            assert_no_gl_error();
        }
    }

    fn lazy_load_buffers(&mut self) {
        if !self.loaded {
            self.loaded = true;

            unsafe {
                gl::GenVertexArrays(1, &mut self.vao);
                gl::BindVertexArray(self.vao);
                assert_no_gl_error();
            }

            let triangle_count: i32;

            {
                let chunks = self.split_into_renderable_chunks();

                let mut all_vertices: Vec<glm::Vec3> = vec![];
                let mut all_normals: Vec<glm::Vec3> = vec![];
                let mut all_colors_ambient: Vec<mtl::Color> = vec![];
                let mut all_colors_diffuse: Vec<mtl::Color> = vec![];
                let mut all_colors_specular: Vec<mtl::Color> = vec![];
                let mut all_specular_exponents: Vec<GLfloat> = vec![];
                let mut indices: Vec<u32> = vec![];

                let mut offset = 0;
                for c in chunks {
                    for i in 0..c.vertices.len() {
                        all_vertices.push(c.vertices[i]);
                        all_normals.push(c.normals[i]);
                        match c.material.illumination {
                            mtl::Illumination::Ambient => {
                                all_colors_ambient.push(c.material.color_ambient);
                                all_colors_diffuse.push(BLACK);
                                all_colors_specular.push(BLACK);
                                all_specular_exponents.push(1.0);
                            },
                            mtl::Illumination::AmbientDiffuse => {
                                all_colors_ambient.push(c.material.color_ambient);
                                all_colors_diffuse.push(c.material.color_diffuse);
                                all_colors_specular.push(BLACK);
                                all_specular_exponents.push(1.0);
                            },
                            mtl::Illumination::AmbientDiffuseSpecular => {
                                all_colors_ambient.push(c.material.color_ambient);
                                all_colors_diffuse.push(c.material.color_diffuse);
                                all_colors_specular.push(c.material.color_specular);
                                all_specular_exponents.push(c.material.specular_coefficient as GLfloat);
                            },
                        }
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

                self.create_array_buffer("in_VertexPosition", all_vertices);
                self.create_array_buffer("in_VertexNormal", all_normals);
                self.create_array_buffer("in_ColorAmbient", all_colors_ambient);
                self.create_array_buffer("in_ColorDiffuse", all_colors_diffuse);
                self.create_array_buffer("in_ColorSpecular", all_colors_specular);
                self.create_array_buffer("in_SpecularExponent", all_specular_exponents);

                triangle_count = indices.len() as i32;
                self.create_element_array_buffer(indices);
            }

            self.triangle_count = triangle_count;
        }
    }
}
