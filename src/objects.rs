use std::vec::Vec;
use std::mem::size_of;
use std::{ ptr, path };
use std::collections::{ HashMap, HashSet };
use multimap;
use gl;
use gl::types::*;
use glm;
use image;
use image::{ GenericImage };
use num_traits::identities::One;
use wavefront_obj::{ obj, mtl };
use util::assert_no_gl_error;
use file;

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

impl Flattenable for glm::Vec2 {
    fn component_count()  -> u32 { 2 }

    fn append_components_to(&self, vector: &mut Vec<GLfloat>) {
        vector.push(self.x);
        vector.push(self.y);
    }
}

impl Flattenable for glm::Vec3 {
    fn component_count() -> u32 { 3 }

    fn append_components_to(&self, vector: &mut Vec<GLfloat>) {
        vector.push(self.x);
        vector.push(self.y);
        vector.push(self.z);
    }
}

impl Flattenable for mtl::Color {
    fn component_count() -> u32 { 3 }

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

pub struct LoadedMesh {
    vao: GLuint,
    texture_name: GLuint,
    index_count: GLint
}

pub struct RenderableObject<'a> {
    filename: String,
    program: &'a shaders::Program,
    meshes: Option<Vec<LoadedMesh>>,
}

fn load_obj_file<'a>(path: &path::Path) -> obj::ObjSet {
    obj::parse(file::read_file_contents(path)).unwrap()
}

fn mapify_mtl(mtl_set: mtl::MtlSet) -> HashMap<String, mtl::Material> {
    let mut map = HashMap::new();
    for m in mtl_set.materials {
        map.insert(m.name.clone(), m);
    }
    map
}

impl <'a> RenderableObject<'a> {
    pub fn new(filename: &str, program: &'a shaders::Program) -> RenderableObject<'a> {
        RenderableObject {
            filename: filename.to_owned(),
            program: program,
            meshes: Option::None,
        }
    }

    pub fn render(&mut self, view: glm::Mat4, projection: glm::Mat4) {
        if self.meshes.is_none() {
            self.meshes = Some(self.load_meshes());
        }

        let model = glm::Mat4::one();
        let model_view_projection = projection * view * model;

        let v_array = util::arrayify_mat4(view);
        let m_array = util::arrayify_mat4(model);
        let mvp_array = util::arrayify_mat4(model_view_projection);
        let light_position = [3f32, 4f32, 15f32];
        let light_color = [1f32, 1f32, 1f32];
        let light_power = 80f32;

        unsafe {
            gl::UseProgram(self.program.name);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatMvp"), 1, gl::FALSE, &*mvp_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatV"), 1, gl::FALSE, &*v_array as *const f32);
            gl::UniformMatrix4fv(self.program.get_uniform("u_MatM"), 1, gl::FALSE, &*m_array as *const f32);
            gl::Uniform3f(self.program.get_uniform("u_LightPosition_WorldSpace"), light_position[0], light_position[1], light_position[2]);
            gl::Uniform3f(self.program.get_uniform("u_LightColor"), light_color[0], light_color[1], light_color[2]);
            gl::Uniform1f(self.program.get_uniform("u_LightPower"), light_power);
            assert_no_gl_error();

            for m in self.meshes.as_ref().unwrap() {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, m.texture_name);
                gl::Uniform1i(self.program.get_uniform("u_TextureDiffuse"), 0);
                gl::BindVertexArray(m.vao);
                gl::DrawElements(gl::TRIANGLES, m.index_count, gl::UNSIGNED_INT, ptr::null());
                assert_no_gl_error();
            }

            // TODO: Cleanup: unbind program/textures/VAOs/etc.
        }
    }

    fn load_meshes(&self) -> Vec<LoadedMesh> {
        let p = path::Path::new(&self.filename);

        let obj_set = load_obj_file(p);
        let materials = obj_set.material_library
            .and_then(|mtl_name| Some(mapify_mtl(mtl::parse(file::read_file_contents(&*p.parent().unwrap().join(mtl_name))).unwrap())));

        obj_set
            .objects
            .into_iter()
            .filter(|o| o.vertices.len() > 0)
            .flat_map(|o| {
                self.load_mesh_for_object(&p, o, &materials).into_iter()
            })
            .collect::<Vec<LoadedMesh>>()
    }

    fn load_mesh_for_object(&self, p: &path::Path, o: obj::Object, materials: &Option<HashMap<String, mtl::Material>>) -> Vec<LoadedMesh> {
        let vertex_positions: Vec<glm::Vec3> = o
            .vertices
            .iter()
            .map(|v| glm::vec3(v.x as f32, v.y as f32, v.z as f32))
            .collect();

        let mut vertex_normal_pairs: Vec<(usize, glm::Vec3)> = o
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
                                // TODO: Normalize here, or later?
                                let normal = glm::cross(
                                    vertex_positions[v2] - vertex_positions[v1],
                                    vertex_positions[v3] - vertex_positions[v1]);
                                vec![(v1, normal), (v2, normal), (v3, normal)]
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
            })
            .collect::<multimap::MultiMap<usize, glm::Vec3>>()
            .iter_all()
            .map(|(v_index, normals)| {
                (*v_index, glm::normalize(normals.iter().fold(glm::vec3(0.0, 0.0, 0.0), |acc, &n| acc + n)))
            })
            .collect();

        // TODO: A trait for sortable iterators. Surprised this doesn't exist already.
        vertex_normal_pairs.sort_by(|p1, p2| p1.0.cmp(&p2.0));

        // TODO: Sometimes, normals will be given to us in the input file.
        // TODO: Smoothing groups, which probably have to be applied across objects...?
        let vertex_normals: Vec<glm::Vec3> = vertex_normal_pairs
            .iter()
            .map(|p| p.1)
            .collect();

        o
            .geometry
            .iter()
            .map(|g| {
                let material;
                // Nested options are pretty sad. :/
                if materials.is_some() && g.material_name.is_some() {
                    material = materials.as_ref().unwrap().get(g.material_name.as_ref().unwrap()).unwrap_or(&DEFAULT_MATERIAL);
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
                    .map(|i| vertex_positions[*i])
                    .collect();

                let normals: Vec<glm::Vec3> = new_to_old_index_mapping
                    .iter()
                    .map(|i| vertex_normals[*i])
                    .collect();

                let indices: Vec<GLuint> = g
                    .shapes
                    .iter()
                    .flat_map(|s| {
                        match s.primitive {
                            obj::Primitive::Triangle(
                                (v1, _, _),
                                (v2, _, _),
                                (v3, _, _),
                            ) => {
                                vec![
                                    old_to_new_index_mapping[&v1] as GLuint,
                                    old_to_new_index_mapping[&v2] as GLuint,
                                    old_to_new_index_mapping[&v3] as GLuint,
                                ].into_iter()
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
                    .collect();

                // TODO: I think the distortion is because these are sorted differently than the positions.
                let uvs: Vec<glm::Vec2> = g
                    .shapes
                    .iter()
                    .flat_map(|s| {
                        match s.primitive {
                            obj::Primitive::Triangle(
                                (_, t1, _),
                                (_, t2, _),
                                (_, t3, _),
                            ) => {
                                vec![t1, t2, t3]
                                    .into_iter()
                                    .map(|t| {
                                        match t {
                                            Some(index) => {
                                                let t_vertex = o.tex_vertices[index];
                                                // TODO: 1 - v?
                                                glm::vec2(t_vertex.u as f32, t_vertex.v as f32)
                                            },
                                            // TODO: Failure mode: an input specifies /some/ texture vertices, but ends up with zeroes elsewhere.
                                            None => glm::vec2(0f32, 0f32),
                                        }
                                    })
                            },
                            _ => { panic!("got non-triangle primitive"); },
                        }
                    })
                    .collect();

                let mut colors_ambient: Vec<mtl::Color> = vec![];
                let mut colors_diffuse: Vec<mtl::Color> = vec![];
                let mut colors_specular: Vec<mtl::Color> = vec![];
                let mut specular_exponents: Vec<GLfloat> = vec![];

                // TODO: Can do some kind of "repeat" instead of this silliness?
                for _ in 0..vertices.len() {
                    match material.illumination {
                        mtl::Illumination::Ambient => {
                            colors_ambient.push(material.color_ambient);
                            colors_diffuse.push(BLACK);
                            colors_specular.push(BLACK);
                            specular_exponents.push(1.0);
                        },
                        mtl::Illumination::AmbientDiffuse => {
                            colors_ambient.push(material.color_ambient);
                            colors_diffuse.push(material.color_diffuse);
                            colors_specular.push(BLACK);
                            specular_exponents.push(1.0);
                        },
                        mtl::Illumination::AmbientDiffuseSpecular => {
                            colors_ambient.push(material.color_ambient);
                            colors_diffuse.push(material.color_diffuse);
                            colors_specular.push(material.color_specular);
                            specular_exponents.push(material.specular_coefficient as GLfloat);
                        },
                    }
                }

                let mut texture: image::DynamicImage;
                match material.uv_map.as_ref() {
                    Some(texture_name) => {
                        texture = image::open(p.parent().unwrap().join(texture_name)).unwrap();
                    },
                    None => {
                        texture = image::DynamicImage::new_rgb8(1, 1);
                        texture.put_pixel(0, 0, image::Rgba([
                            (material.color_diffuse.r * 255f64) as u8,
                            (material.color_diffuse.g * 255f64) as u8,
                            (material.color_diffuse.b * 255f64) as u8,
                            255
                        ]));
                    },
                }

                let mut vao = 0;
                unsafe {
                    gl::GenVertexArrays(1, &mut vao);
                    gl::BindVertexArray(vao);
                    assert_no_gl_error();
                }

                self.create_array_buffer("in_VertexPosition", vertices);
                self.create_array_buffer("in_VertexNormal", normals);
                self.create_array_buffer("in_VertexUv", uvs);
                self.create_array_buffer("in_ColorAmbient", colors_ambient);
                self.create_array_buffer("in_ColorDiffuse", colors_diffuse);
                self.create_array_buffer("in_ColorSpecular", colors_specular);
                self.create_array_buffer("in_SpecularExponent", specular_exponents);

                let index_count = indices.len();
                self.create_element_array_buffer(indices);

                unsafe {
                    // TODO: Verify that this is "unbind".
                    gl::BindVertexArray(0);
                }

                let texture_name = self.create_texture_buffer(texture);

                LoadedMesh {
                    vao: vao,
                    texture_name: texture_name,
                    index_count: index_count as GLint,
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

    fn create_texture_buffer(&self, texture: image::DynamicImage) -> GLuint {
        let (width, height) = texture.dimensions();
        unsafe {
            let mut texture_buffer_name: GLuint = 0;
            gl::GenTextures(1, &mut texture_buffer_name);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture_buffer_name);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as GLint,
                width as GLsizei,
                height as GLsizei,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                // TODO: Literally no idea if this is right.
                (*(texture.to_rgb())).as_ptr() as *const _,
            );

            // TODO: Better resampling.
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);

            assert_no_gl_error();

            texture_buffer_name
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
}
