pub fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
    assert!((a - b).abs() < epsilon, "Expected {:.6}, got {:.6}", b, a);
}

pub fn assert_vec_approx_eq(a: [f32; 4], b: [f32; 4], epsilon: f32) {
    for i in 0..4 {
        assert_approx_eq(a[i], b[i], epsilon);
    }
}
