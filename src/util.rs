use glm;
use std::boxed::Box;

pub fn arrayify_mat4(mat: glm::Mat4) -> Box<[f32; 16]> {
    let mut array = [0f32; 16];

    for i in 0..4 {
        for j in 0..4 {
            array[i * 4 + j] = mat[i][j];
        }
    }

    Box::new(array)
}
