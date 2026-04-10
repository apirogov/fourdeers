pub const SQRT_3: f32 = 1.732_050_8;

pub const TETRAHEDRON_BASE_VERTICES: [[f32; 3]; 4] = [
    [1.0, 1.0, 1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [1.0, -1.0, -1.0],
];

use nalgebra::Vector3;

#[must_use]
pub fn tetrahedron_vertices(scale: f32) -> [Vector3<f32>; 4] {
    let s = scale / SQRT_3;
    TETRAHEDRON_BASE_VERTICES.map(|[x, y, z]| Vector3::new(x * s, y * s, z * s))
}

#[must_use]
pub fn magnitude_4d(v: nalgebra::Vector4<f32>) -> f32 {
    (v.x.powi(2) + v.y.powi(2) + v.z.powi(2) + v.w.powi(2)).sqrt()
}

#[must_use]
pub fn component_weights(vector_4d: nalgebra::Vector4<f32>) -> Option<[f32; 4]> {
    let norm = magnitude_4d(vector_4d);
    if norm < 1e-6 {
        return None;
    }
    let normalized = vector_4d / norm;
    Some([
        normalized.x.abs(),
        normalized.y.abs(),
        normalized.z.abs(),
        normalized.w.abs(),
    ])
}

#[must_use]
pub fn compute_weighted_direction_3d(vector_4d: nalgebra::Vector4<f32>) -> Vector3<f32> {
    let Some(weights) = component_weights(vector_4d) else {
        return Vector3::zeros();
    };

    let mut result = Vector3::zeros();
    for (i, &weight) in weights.iter().enumerate() {
        let [x, y, z] = TETRAHEDRON_BASE_VERTICES[i];
        result += Vector3::new(x, y, z) * weight;
    }

    result
}
