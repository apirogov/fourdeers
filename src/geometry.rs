//! Geometry primitives for polyhedron clipping and convex hull computation

use nalgebra::Vector3;

const VERTEX_MERGE_EPS_SQ: f32 = 1e-6;

pub(crate) struct VertexDedup {
    pub(crate) vertices: Vec<Vector3<f32>>,
}

impl VertexDedup {
    pub(crate) const fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    pub(crate) fn find_or_add(&mut self, v: Vector3<f32>) -> usize {
        for (i, existing) in self.vertices.iter().enumerate() {
            if (existing - v).norm_squared() < VERTEX_MERGE_EPS_SQ {
                return i;
            }
        }
        self.vertices.push(v);
        self.vertices.len() - 1
    }
}

pub(crate) struct ConvexPolyhedron {
    pub vertices: Vec<Vector3<f32>>,
    pub edges: Vec<[usize; 2]>,
}

#[allow(clippy::float_cmp)]
#[allow(clippy::similar_names)]
pub(crate) fn convex_hull_2d_indexed(points: &[(f32, f32)]) -> Vec<usize> {
    let n = points.len();
    if n < 3 {
        return (0..n).collect();
    }
    let mut start = 0;
    for i in 1..n {
        if points[i].0 < points[start].0
            || (points[i].0 == points[start].0 && points[i].1 < points[start].1)
        {
            start = i;
        }
    }
    let mut hull = Vec::new();
    let mut current = start;
    loop {
        hull.push(current);
        let mut next = 0;
        for i in 0..n {
            if i == current {
                continue;
            }
            if next == current {
                next = i;
                continue;
            }
            let oc_x = points[i].0 - points[current].0;
            let oc_y = points[i].1 - points[current].1;
            let on_x = points[next].0 - points[current].0;
            let on_y = points[next].1 - points[current].1;
            let cross = oc_x * on_y - oc_y * on_x;
            if cross > 0.0 {
                next = i;
            } else if cross.abs() < 1e-10 {
                let d_i = oc_x * oc_x + oc_y * oc_y;
                let d_n = on_x * on_x + on_y * on_y;
                if d_i > d_n {
                    next = i;
                }
            }
        }
        current = next;
        if current == start {
            break;
        }
        if hull.len() > n {
            break;
        }
    }
    hull
}

pub(crate) fn convex_hull_2d(pts: &[eframe::egui::Pos2]) -> Vec<eframe::egui::Pos2> {
    let coords: Vec<(f32, f32)> = pts.iter().map(|p| (p.x, p.y)).collect();
    let indices = convex_hull_2d_indexed(&coords);
    indices.into_iter().map(|i| pts[i]).collect()
}

pub(crate) fn clip_polyhedron_by_plane(
    poly: &ConvexPolyhedron,
    plane_point: Vector3<f32>,
    plane_normal: Vector3<f32>,
) -> ConvexPolyhedron {
    if poly.vertices.is_empty() {
        return ConvexPolyhedron {
            vertices: Vec::new(),
            edges: Vec::new(),
        };
    }
    let distances: Vec<f32> = poly
        .vertices
        .iter()
        .map(|v| (v - plane_point).dot(&plane_normal))
        .collect();
    let is_inside = |i: usize| distances[i] >= 0.0;
    let mut dedup = VertexDedup::new();
    let mut new_edges: Vec<[usize; 2]> = Vec::new();
    let mut crossing_points: Vec<Vector3<f32>> = Vec::new();
    for &[i, j] in &poly.edges {
        let ci = is_inside(i);
        let cj = is_inside(j);
        if ci && cj {
            let ni = dedup.find_or_add(poly.vertices[i]);
            let nj = dedup.find_or_add(poly.vertices[j]);
            if ni != nj {
                new_edges.push([ni, nj]);
            }
        } else if ci != cj {
            let d_i = distances[i];
            let d_j = distances[j];
            let t = d_i / (d_i - d_j);
            let intersection = poly.vertices[i] + (poly.vertices[j] - poly.vertices[i]) * t;
            let ix = dedup.find_or_add(intersection);
            crossing_points.push(intersection);
            if ci {
                let ni = dedup.find_or_add(poly.vertices[i]);
                if ni != ix {
                    new_edges.push([ni, ix]);
                }
            } else {
                let nj = dedup.find_or_add(poly.vertices[j]);
                if nj != ix {
                    new_edges.push([ix, nj]);
                }
            }
        }
    }
    if crossing_points.len() >= 3 {
        let normal = plane_normal.normalize();
        let (u, v) = if normal.z.abs() < 0.9 {
            let u = normal.cross(&Vector3::z()).normalize();
            let v = normal.cross(&u).normalize();
            (u, v)
        } else {
            let u = normal.cross(&Vector3::x()).normalize();
            let v = normal.cross(&u).normalize();
            (u, v)
        };
        let pts_2d: Vec<(f32, f32)> = crossing_points
            .iter()
            .map(|p| {
                let d = *p - plane_point;
                (d.dot(&u), d.dot(&v))
            })
            .collect();
        let hull_idx = convex_hull_2d_indexed(&pts_2d);
        for w in hull_idx.windows(2) {
            let a = dedup.find_or_add(crossing_points[w[0]]);
            let b = dedup.find_or_add(crossing_points[w[1]]);
            if a != b {
                new_edges.push([a, b]);
            }
        }
        if hull_idx.len() >= 3 {
            let a = dedup
                .find_or_add(crossing_points[*hull_idx.last().expect("hull has >= 3 elements")]);
            let b = dedup.find_or_add(crossing_points[hull_idx[0]]);
            if a != b {
                new_edges.push([a, b]);
            }
        }
    }
    ConvexPolyhedron {
        vertices: dedup.vertices,
        edges: new_edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_polyhedron() -> ConvexPolyhedron {
        let vertices = vec![
            Vector3::new(-1.0, -1.0, -1.0),
            Vector3::new(1.0, -1.0, -1.0),
            Vector3::new(1.0, 1.0, -1.0),
            Vector3::new(-1.0, 1.0, -1.0),
            Vector3::new(-1.0, -1.0, 1.0),
            Vector3::new(1.0, -1.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(-1.0, 1.0, 1.0),
        ];
        let edges = vec![
            [0, 1],
            [1, 2],
            [2, 3],
            [3, 0],
            [4, 5],
            [5, 6],
            [6, 7],
            [7, 4],
            [0, 4],
            [1, 5],
            [2, 6],
            [3, 7],
        ];
        ConvexPolyhedron { vertices, edges }
    }

    #[test]
    fn test_convex_hull_square() {
        let pts = vec![
            eframe::egui::Pos2::new(0.0, 0.0),
            eframe::egui::Pos2::new(1.0, 0.0),
            eframe::egui::Pos2::new(1.0, 1.0),
            eframe::egui::Pos2::new(0.0, 1.0),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 4);
        for pt in &pts {
            assert!(hull.contains(pt), "hull should contain {:?}", pt);
        }
    }

    #[test]
    fn test_convex_hull_with_interior_points() {
        let pts = vec![
            eframe::egui::Pos2::new(0.0, 0.0),
            eframe::egui::Pos2::new(2.0, 0.0),
            eframe::egui::Pos2::new(2.0, 2.0),
            eframe::egui::Pos2::new(0.0, 2.0),
            eframe::egui::Pos2::new(1.0, 1.0),
            eframe::egui::Pos2::new(0.5, 0.5),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(
            hull.len(),
            4,
            "interior points should be excluded from hull"
        );
        assert!(
            !hull.contains(&eframe::egui::Pos2::new(1.0, 1.0)),
            "interior point should not be in hull"
        );
        assert!(
            !hull.contains(&eframe::egui::Pos2::new(0.5, 0.5)),
            "interior point should not be in hull"
        );
    }

    #[test]
    fn test_convex_hull_triangle() {
        let pts = vec![
            eframe::egui::Pos2::new(0.0, 0.0),
            eframe::egui::Pos2::new(1.0, 0.0),
            eframe::egui::Pos2::new(0.5, 1.0),
            eframe::egui::Pos2::new(0.5, 0.3),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 3);
    }

    #[test]
    fn test_convex_hull_collinear() {
        let pts = vec![
            eframe::egui::Pos2::new(0.0, 0.0),
            eframe::egui::Pos2::new(1.0, 0.0),
            eframe::egui::Pos2::new(2.0, 0.0),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(
            hull.len(),
            2,
            "collinear points should produce a degenerate hull"
        );
    }

    #[test]
    fn test_clip_polyhedron_preserves_fully_inside() {
        let cube = unit_cube_polyhedron();
        let plane_point = Vector3::new(-2.0, 0.0, 0.0);
        let plane_normal = Vector3::new(1.0, 0.0, 0.0);
        let result = clip_polyhedron_by_plane(&cube, plane_point, plane_normal);
        assert_eq!(result.vertices.len(), 8, "cube should be fully preserved");
        assert_eq!(result.edges.len(), 12, "all edges should be preserved");
    }

    #[test]
    fn test_clip_polyhedron_empties_fully_outside() {
        let cube = unit_cube_polyhedron();
        let plane_point = Vector3::new(1.5, 0.0, 0.0);
        let plane_normal = Vector3::new(1.0, 0.0, 0.0);
        let result = clip_polyhedron_by_plane(&cube, plane_point, plane_normal);
        assert!(
            result.vertices.is_empty(),
            "cube entirely outside should be empty"
        );
    }

    #[test]
    fn test_clip_polyhedron_half_cube() {
        let cube = unit_cube_polyhedron();
        let plane_point = Vector3::new(0.0, 0.0, 0.0);
        let plane_normal = Vector3::new(1.0, 0.0, 0.0);
        let result = clip_polyhedron_by_plane(&cube, plane_point, plane_normal);
        assert!(
            result.vertices.len() >= 6,
            "half-cube should have >= 6 vertices, got {}",
            result.vertices.len()
        );
        assert!(
            result.edges.len() >= 8,
            "half-cube should have >= 8 edges, got {}",
            result.edges.len()
        );
        for v in &result.vertices {
            assert!(
                v.x >= -1e-6,
                "all vertices should have x >= 0, got x={}",
                v.x
            );
        }
    }

    #[test]
    fn test_clip_polyhedron_by_plane_diagonal() {
        let cube = unit_cube_polyhedron();
        let plane_point = Vector3::new(0.0, 0.0, 0.0);
        let plane_normal = Vector3::new(1.0, 1.0, 0.0).normalize();
        let result = clip_polyhedron_by_plane(&cube, plane_point, plane_normal);
        assert!(
            result.vertices.len() >= 4,
            "diagonal clip should produce >= 4 vertices, got {}",
            result.vertices.len()
        );
        assert!(
            result.edges.len() >= 6,
            "diagonal clip should produce >= 6 edges, got {}",
            result.edges.len()
        );
    }
}
