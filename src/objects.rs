use std::vec::Vec;
use std::mem::size_of;
use std::ffi::CString;
use std::ptr;
use gl;
use gl::types::*;
use wavefront_obj::obj;
use util::assert_no_gl_error;

pub struct RenderableObject {
    object: obj::Object,
    program: GLuint,
    initialized: bool,
    vao: GLuint,
    indices: i32,
}

impl RenderableObject {
    pub fn new(object: obj::Object, program: GLuint) -> RenderableObject {
        RenderableObject {
            object: object,
            program: program,
            initialized: false,
            vao: 0,
            indices: 0,
        }
    }

    pub fn render(&mut self) {
        self.lazy_init();

        unsafe {
            // TODO: Should probably upload program myself, but don't want to do it redundantly.
            // Probably should create a Program object that acn be enabled/disabled.
            gl::UseProgram(self.program);
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
                // TODO: Is this necessary?
                // use the appropriate program so we can fetch information about it
                // gl::UseProgram(self.program);

                // create the VAO for this object
                gl::GenVertexArrays(1, &mut self.vao);
                gl::BindVertexArray(self.vao);

                // set current array data buffer and fill it with vertex position data
                let mut v_position_buffer: GLuint = 0;
                gl::GenBuffers(1, &mut v_position_buffer);
                gl::BindBuffer(gl::ARRAY_BUFFER, v_position_buffer);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (flattened_vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                    flattened_vertices.as_ptr() as *const _,
                    gl::STATIC_DRAW);

                // find the location of the position argument in the shader and tell OpenGL that the currently bound array points to it in triples
                let position_attrib_location = gl::GetAttribLocation(
                    self.program,
                    CString::new("in_Position").unwrap().as_ptr()) as GLuint;
                assert_ne!(position_attrib_location, -1i32 as u32, "position attribute not found or not enabled in program");

                gl::EnableVertexAttribArray(position_attrib_location);
                gl::VertexAttribPointer(
                    position_attrib_location,
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
