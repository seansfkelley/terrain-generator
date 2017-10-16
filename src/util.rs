use gl;
use glm;
use std::boxed::Box;

pub fn assert_no_gl_error() {
    unsafe {
        let error = gl::GetError();
        if error != gl::NO_ERROR {
            panic!(format!("OpenGL error code 0x{:X}", error));
        }
    }
}

pub fn arrayify_mat4(mat: glm::Mat4) -> Box<[f32; 16]> {
    let mut array = [0f32; 16];

    for i in 0..4 {
        for j in 0..4 {
            array[i * 4 + j] = mat[i][j];
        }
    }

    Box::new(array)
}
