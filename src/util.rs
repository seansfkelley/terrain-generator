use gl;
use glm;
use std::boxed::Box;

pub fn assert_no_gl_error() {
    unsafe {
        let error = gl::GetError();
        if error != gl::NO_ERROR {
            let name = match error {
                gl::INVALID_ENUM => "invalid enum",
                gl::INVALID_VALUE => "invalid value",
                gl::INVALID_OPERATION => "invalid operation",
                gl::STACK_OVERFLOW => "stack overflow",
                gl::STACK_UNDERFLOW => "stack underflow",
                gl::OUT_OF_MEMORY => "out of memory",
                _ => panic!("unknown error code while handling OpenGL error: {}", error),
            };
            panic!(format!("OpenGL error code 0x{:X}: {}", error, name));
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
