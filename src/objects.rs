use std::vec::Vec;
use std::mem::size_of;
use std::ptr;
use gl;
use gl::types::*;
use glm;
use num_traits::identities::One;
use wavefront_obj::{ obj, mtl };
use util::assert_no_gl_error;

use shaders;
use util;

pub struct RenderableObject<'a> {
    object: obj::Object,
    material: Option<mtl::MtlSet>,
    program: &'a shaders::Program,
    initialized: bool,
    vao: GLuint,
    indices: i32,
}

impl <'a> RenderableObject<'a> {
    pub fn new(object: obj::Object, material: Option<mtl::MtlSet>, program: &shaders::Program) -> RenderableObject {
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

            let flattened_vertices: Vec<GLfloat> = self
                .object
                .vertices
                .iter()
                .flat_map(|v| vec![ v.x as GLfloat, v.y as GLfloat, v.z as GLfloat ].into_iter())
                .collect();

            let flattened_triangle_indices: Vec<u32> = self
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

            self.indices = flattened_triangle_indices.len() as i32;

            unsafe {
                // create the VAO for this object
                gl::GenVertexArrays(1, &mut self.vao);
                gl::BindVertexArray(self.vao);
                assert_no_gl_error();

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

                // set current array data buffer and fill it with vertex "color" data (reusing position for now for test)
                let mut v_color_buffer: GLuint = 0;
                gl::GenBuffers(1, &mut v_color_buffer);
                gl::BindBuffer(gl::ARRAY_BUFFER, v_color_buffer);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (flattened_vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                    flattened_vertices.as_ptr() as *const _,
                    gl::STATIC_DRAW);

                let fragment_color_attrib = self.program.get_attrib("in_FragmentColor") as GLuint;
                gl::EnableVertexAttribArray(fragment_color_attrib);
                gl::VertexAttribPointer(
                    fragment_color_attrib,
                    3,
                    gl::FLOAT,
                    gl::FALSE as GLboolean,
                    0,
                    ptr::null());

                // lastly, tell OpenGL about the indices (that must be correlated for all buffers!)
                let mut index_buffer: GLuint = 0;
                gl::GenBuffers(1, &mut index_buffer);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (flattened_triangle_indices.len() * size_of::<u32>()) as GLsizeiptr,
                    flattened_triangle_indices.as_ptr() as *const _,
                    gl::STATIC_DRAW);
                assert_no_gl_error();
            }
        }
    }
}
