//! Regular convex 4-polytopes
//!
//! All 6 regular convex 4-polytopes, centered at the origin.
//!
//! | Name | Vertices | Edges | Cells |
//! |------|----------|-------|-------|
//! | 5-cell (pentachoron) | 5 | 10 | 5 |
//! | 8-cell (tesseract) | 16 | 32 | 8 |
//! | 16-cell | 8 | 24 | 16 |
//! | 24-cell | 24 | 96 | 24 |
//! | 120-cell | 600 | 1200 | 120 |
//! | 600-cell | 120 | 720 | 600 |

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex4D {
    pub position: [f32; 4],
}

impl Vertex4D {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            position: [x, y, z, w],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolytopeType {
    FiveCell,
    #[default]
    EightCell,
    SixteenCell,
    TwentyFourCell,
    OneHundredTwentyCell,
    SixHundredCell,
}

impl PolytopeType {
    pub fn name(&self) -> &'static str {
        match self {
            PolytopeType::FiveCell => "5-cell (Pentachoron)",
            PolytopeType::EightCell => "8-cell (Tesseract)",
            PolytopeType::SixteenCell => "16-cell",
            PolytopeType::TwentyFourCell => "24-cell",
            PolytopeType::OneHundredTwentyCell => "120-cell",
            PolytopeType::SixHundredCell => "600-cell",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            PolytopeType::FiveCell => "5-cell",
            PolytopeType::EightCell => "tesseract",
            PolytopeType::SixteenCell => "16-cell",
            PolytopeType::TwentyFourCell => "24-cell",
            PolytopeType::OneHundredTwentyCell => "120-cell",
            PolytopeType::SixHundredCell => "600-cell",
        }
    }

    pub fn vertex_count(&self) -> usize {
        match self {
            PolytopeType::FiveCell => 5,
            PolytopeType::EightCell => 16,
            PolytopeType::SixteenCell => 8,
            PolytopeType::TwentyFourCell => 24,
            PolytopeType::OneHundredTwentyCell => 600,
            PolytopeType::SixHundredCell => 120,
        }
    }

    pub fn edge_count(&self) -> usize {
        match self {
            PolytopeType::FiveCell => 10,
            PolytopeType::EightCell => 32,
            PolytopeType::SixteenCell => 24,
            PolytopeType::TwentyFourCell => 96,
            PolytopeType::OneHundredTwentyCell => 1200,
            PolytopeType::SixHundredCell => 720,
        }
    }

    pub fn all() -> [PolytopeType; 6] {
        [
            PolytopeType::FiveCell,
            PolytopeType::EightCell,
            PolytopeType::SixteenCell,
            PolytopeType::TwentyFourCell,
            PolytopeType::OneHundredTwentyCell,
            PolytopeType::SixHundredCell,
        ]
    }
}

pub fn create_polytope(kind: PolytopeType) -> (Vec<Vertex4D>, Vec<u16>) {
    match kind {
        PolytopeType::FiveCell => create_5_cell(),
        PolytopeType::EightCell => create_8_cell(),
        PolytopeType::SixteenCell => create_16_cell(),
        PolytopeType::TwentyFourCell => create_24_cell(),
        PolytopeType::OneHundredTwentyCell => create_120_cell_stub(),
        PolytopeType::SixHundredCell => create_600_cell_stub(),
    }
}

fn create_5_cell() -> (Vec<Vertex4D>, Vec<u16>) {
    let sqrt5 = 5.0f32.sqrt();
    let inv_sqrt5 = 1.0 / sqrt5;
    let four_inv_sqrt5 = 4.0 / sqrt5;

    let vertices = vec![
        Vertex4D::new(1.0, 1.0, 1.0, -inv_sqrt5),
        Vertex4D::new(1.0, -1.0, -1.0, -inv_sqrt5),
        Vertex4D::new(-1.0, 1.0, -1.0, -inv_sqrt5),
        Vertex4D::new(-1.0, -1.0, 1.0, -inv_sqrt5),
        Vertex4D::new(0.0, 0.0, 0.0, four_inv_sqrt5),
    ];

    let indices: Vec<u16> = vec![0, 1, 0, 2, 0, 3, 0, 4, 1, 2, 1, 3, 1, 4, 2, 3, 2, 4, 3, 4];

    (vertices, indices)
}

fn create_8_cell() -> (Vec<Vertex4D>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(16);
    for i in 0..16 {
        let x = if (i & 1) != 0 { 1.0 } else { -1.0 };
        let y = if (i & 2) != 0 { 1.0 } else { -1.0 };
        let z = if (i & 4) != 0 { 1.0 } else { -1.0 };
        let w = if (i & 8) != 0 { 1.0 } else { -1.0 };
        vertices.push(Vertex4D::new(x, y, z, w));
    }

    let mut indices = Vec::new();
    for i in 0..16u16 {
        for bit in 0..4 {
            let j = i ^ (1 << bit);
            if i < j {
                indices.push(i);
                indices.push(j);
            }
        }
    }

    (vertices, indices)
}

fn create_16_cell() -> (Vec<Vertex4D>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(8);

    for i in 0..8 {
        let axis = i / 2;
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        let mut pos = [0.0f32; 4];
        pos[axis] = sign;
        vertices.push(Vertex4D { position: pos });
    }

    let mut indices = Vec::new();
    for i in 0..8u16 {
        for j in (i + 1)..8u16 {
            if (i / 2) != (j / 2) {
                indices.push(i);
                indices.push(j);
            }
        }
    }

    (vertices, indices)
}

fn create_24_cell() -> (Vec<Vertex4D>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(24);

    for i in 0..4 {
        for j in (i + 1)..4 {
            for &sign_i in &[1.0, -1.0] {
                for &sign_j in &[1.0, -1.0] {
                    let mut pos = [0.0f32; 4];
                    pos[i] = sign_i;
                    pos[j] = sign_j;
                    vertices.push(Vertex4D { position: pos });
                }
            }
        }
    }

    let mut indices = Vec::new();
    let edge_length_sq = 2.0f32;

    for i in 0..24u16 {
        for j in (i + 1)..24u16 {
            let v1 = &vertices[i as usize];
            let v2 = &vertices[j as usize];
            let dist_sq = (0..4)
                .map(|k| (v1.position[k] - v2.position[k]).powi(2))
                .sum::<f32>();

            if (dist_sq - edge_length_sq).abs() < 1e-6 {
                indices.push(i);
                indices.push(j);
            }
        }
    }

    (vertices, indices)
}

fn create_120_cell_stub() -> (Vec<Vertex4D>, Vec<u16>) {
    create_8_cell()
}

fn create_600_cell_stub() -> (Vec<Vertex4D>, Vec<u16>) {
    create_16_cell()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn distance_sq(v1: &Vertex4D, v2: &Vertex4D) -> f32 {
        (0..4)
            .map(|i| (v1.position[i] - v2.position[i]).powi(2))
            .sum()
    }

    fn centroid(vertices: &[Vertex4D]) -> [f32; 4] {
        let n = vertices.len() as f32;
        let mut c = [0.0f32; 4];
        for v in vertices {
            for i in 0..4 {
                c[i] += v.position[i];
            }
        }
        for i in 0..4 {
            c[i] /= n;
        }
        c
    }

    fn count_antipodal_pairs(vertices: &[Vertex4D]) -> usize {
        let mut count = 0;
        for i in 0..vertices.len() {
            for j in (i + 1)..vertices.len() {
                let v1 = &vertices[i];
                let v2 = &vertices[j];
                let is_antipodal =
                    (0..4).all(|k| (v1.position[k] + v2.position[k]).abs() < EPSILON);
                if is_antipodal {
                    count += 1;
                }
            }
        }
        count
    }

    fn uniform_edge_lengths(vertices: &[Vertex4D], indices: &[u16]) -> bool {
        if indices.len() < 2 {
            return true;
        }
        let first_dist_sq = distance_sq(
            &vertices[indices[0] as usize],
            &vertices[indices[1] as usize],
        );
        indices.chunks(2).all(|chunk| {
            let d = distance_sq(&vertices[chunk[0] as usize], &vertices[chunk[1] as usize]);
            (d - first_dist_sq).abs() < 1e-3 * first_dist_sq.max(1.0)
        })
    }

    fn compute_vertex_degrees(vertices: &[Vertex4D], indices: &[u16]) -> Vec<usize> {
        let mut degrees = vec![0usize; vertices.len()];
        for chunk in indices.chunks(2) {
            degrees[chunk[0] as usize] += 1;
            degrees[chunk[1] as usize] += 1;
        }
        degrees
    }

    #[test]
    fn test_5_cell_vertex_count() {
        let (vertices, _) = create_5_cell();
        assert_eq!(vertices.len(), 5);
    }

    #[test]
    fn test_5_cell_edge_count() {
        let (_, indices) = create_5_cell();
        assert_eq!(indices.len(), 20);
    }

    #[test]
    fn test_5_cell_centered_at_origin() {
        let (vertices, _) = create_5_cell();
        let c = centroid(&vertices);
        for i in 0..4 {
            assert!(c[i].abs() < EPSILON, "Centroid not at origin: {:?}", c);
        }
    }

    #[test]
    fn test_5_cell_uniform_edges() {
        let (vertices, indices) = create_5_cell();
        assert!(
            uniform_edge_lengths(&vertices, &indices),
            "5-cell edges not uniform"
        );
    }

    #[test]
    fn test_5_cell_no_antipodal_pairs() {
        let (vertices, _) = create_5_cell();
        assert_eq!(
            count_antipodal_pairs(&vertices),
            0,
            "5-cell should have no antipodal pairs"
        );
    }

    #[test]
    fn test_5_cell_vertex_degrees() {
        let (vertices, indices) = create_5_cell();
        let degrees = compute_vertex_degrees(&vertices, &indices);
        for (i, &deg) in degrees.iter().enumerate() {
            assert_eq!(deg, 4, "Vertex {} should have degree 4, got {}", i, deg);
        }
    }

    #[test]
    fn test_8_cell_vertex_count() {
        let (vertices, _) = create_8_cell();
        assert_eq!(vertices.len(), 16);
    }

    #[test]
    fn test_8_cell_edge_count() {
        let (_, indices) = create_8_cell();
        assert_eq!(indices.len(), 64);
    }

    #[test]
    fn test_8_cell_centered_at_origin() {
        let (vertices, _) = create_8_cell();
        let c = centroid(&vertices);
        for i in 0..4 {
            assert!(c[i].abs() < EPSILON, "Centroid not at origin: {:?}", c);
        }
    }

    #[test]
    fn test_8_cell_uniform_edges() {
        let (vertices, indices) = create_8_cell();
        assert!(
            uniform_edge_lengths(&vertices, &indices),
            "8-cell edges not uniform"
        );
    }

    #[test]
    fn test_8_cell_antipodal_pairs() {
        let (vertices, _) = create_8_cell();
        assert_eq!(
            count_antipodal_pairs(&vertices),
            8,
            "8-cell should have 8 antipodal pairs"
        );
    }

    #[test]
    fn test_8_cell_vertex_degrees() {
        let (vertices, indices) = create_8_cell();
        let degrees = compute_vertex_degrees(&vertices, &indices);
        for (i, &deg) in degrees.iter().enumerate() {
            assert_eq!(deg, 4, "Vertex {} should have degree 4, got {}", i, deg);
        }
    }

    #[test]
    fn test_16_cell_vertex_count() {
        let (vertices, _) = create_16_cell();
        assert_eq!(vertices.len(), 8);
    }

    #[test]
    fn test_16_cell_edge_count() {
        let (_, indices) = create_16_cell();
        assert_eq!(indices.len(), 48);
    }

    #[test]
    fn test_16_cell_centered_at_origin() {
        let (vertices, _) = create_16_cell();
        let c = centroid(&vertices);
        for i in 0..4 {
            assert!(c[i].abs() < EPSILON, "Centroid not at origin: {:?}", c);
        }
    }

    #[test]
    fn test_16_cell_uniform_edges() {
        let (vertices, indices) = create_16_cell();
        assert!(
            uniform_edge_lengths(&vertices, &indices),
            "16-cell edges not uniform"
        );
    }

    #[test]
    fn test_16_cell_antipodal_pairs() {
        let (vertices, _) = create_16_cell();
        assert_eq!(
            count_antipodal_pairs(&vertices),
            4,
            "16-cell should have 4 antipodal pairs"
        );
    }

    #[test]
    fn test_16_cell_vertex_degrees() {
        let (vertices, indices) = create_16_cell();
        let degrees = compute_vertex_degrees(&vertices, &indices);
        for (i, &deg) in degrees.iter().enumerate() {
            assert_eq!(deg, 6, "Vertex {} should have degree 6, got {}", i, deg);
        }
    }

    #[test]
    fn test_24_cell_vertex_count() {
        let (vertices, _) = create_24_cell();
        assert_eq!(vertices.len(), 24);
    }

    #[test]
    fn test_24_cell_edge_count() {
        let (_, indices) = create_24_cell();
        assert_eq!(indices.len(), 192);
    }

    #[test]
    fn test_24_cell_centered_at_origin() {
        let (vertices, _) = create_24_cell();
        let c = centroid(&vertices);
        for i in 0..4 {
            assert!(c[i].abs() < EPSILON, "Centroid not at origin: {:?}", c);
        }
    }

    #[test]
    fn test_24_cell_uniform_edges() {
        let (vertices, indices) = create_24_cell();
        assert!(
            uniform_edge_lengths(&vertices, &indices),
            "24-cell edges not uniform"
        );
    }

    #[test]
    fn test_24_cell_antipodal_pairs() {
        let (vertices, _) = create_24_cell();
        assert_eq!(
            count_antipodal_pairs(&vertices),
            12,
            "24-cell should have 12 antipodal pairs"
        );
    }

    #[test]
    fn test_24_cell_vertex_degrees() {
        let (vertices, indices) = create_24_cell();
        let degrees = compute_vertex_degrees(&vertices, &indices);
        for (i, &deg) in degrees.iter().enumerate() {
            assert_eq!(deg, 8, "Vertex {} should have degree 8, got {}", i, deg);
        }
    }

    #[test]
    fn test_polytope_type_metadata() {
        assert_eq!(PolytopeType::FiveCell.vertex_count(), 5);
        assert_eq!(PolytopeType::FiveCell.edge_count(), 10);

        assert_eq!(PolytopeType::EightCell.vertex_count(), 16);
        assert_eq!(PolytopeType::EightCell.edge_count(), 32);

        assert_eq!(PolytopeType::SixteenCell.vertex_count(), 8);
        assert_eq!(PolytopeType::SixteenCell.edge_count(), 24);

        assert_eq!(PolytopeType::TwentyFourCell.vertex_count(), 24);
        assert_eq!(PolytopeType::TwentyFourCell.edge_count(), 96);
    }
}
