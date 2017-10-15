use glm;

// TODO: Probably a faster way to implement this.
pub fn arrayify_mat4(mat: glm::Mat4) -> *const f32 {
    let mut array = Vec::new();

    for i in 0..4 {
        for j in 0..4 {
            array.push(mat[i][j]);
        }
    }

    return array.as_ptr();
}
