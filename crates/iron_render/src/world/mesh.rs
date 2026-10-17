//! Triangle meshes for the world pass: vertex data, and the primitives Unity has
//! (cube, sphere, capsule, cylinder, plane, quad) built in code.
//!
//! Meshes are right-handed, y up, with counter-clockwise front faces and unit sizes
//! like Unity's: a 1 m cube, a sphere of radius 0.5, a 2 m tall capsule, a 10 m plane.

use std::f32::consts::{PI, TAU};

/// A mesh's vertices (positions, normals, texture coordinates) and triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Three per triangle, counter-clockwise seen from the front.
    pub indices: Vec<u32>,
}

impl MeshData {
    fn push(&mut self, p: [f32; 3], n: [f32; 3], uv: [f32; 2]) -> u32 {
        self.positions.push(p);
        self.normals.push(n);
        self.uvs.push(uv);
        self.positions.len() as u32 - 1
    }

    /// Fills in missing normals (smooth, from the triangles) and texture coordinates.
    pub fn complete(mut self) -> Self {
        if self.uvs.len() != self.positions.len() {
            self.uvs = vec![[0.0, 0.0]; self.positions.len()];
        }
        if self.normals.len() != self.positions.len() {
            let mut normals = vec![[0.0f32; 3]; self.positions.len()];
            for t in self.indices.as_chunks::<3>().0 {
                let [a, b, c] = [t[0], t[1], t[2]]
                    .map(|i| self.positions.get(i as usize).copied().unwrap_or_default());
                let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for &i in t {
                    if let Some(slot) = normals.get_mut(i as usize) {
                        for k in 0..3 {
                            slot[k] += n[k];
                        }
                    }
                }
            }
            for n in &mut normals {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if len > 0.0 {
                    *n = [n[0] / len, n[1] / len, n[2] / len];
                }
            }
            self.normals = normals;
        }
        self
    }

    /// Whether every index is in range and there are whole triangles.
    pub fn is_valid(&self) -> bool {
        self.indices.len().is_multiple_of(3)
            && self.positions.len() == self.normals.len()
            && self.positions.len() == self.uvs.len()
            && self
                .indices
                .iter()
                .all(|&i| (i as usize) < self.positions.len())
    }
}

/// A built-in mesh by Unity's name: `cube`, `sphere`, `capsule`, `cylinder`, `plane`
/// (10 m, facing up) or `quad` (1 m, facing +z).
pub fn primitive(name: &str) -> Option<MeshData> {
    Some(match name {
        "cube" => cube(),
        "sphere" => sphere(0.5, 24, 16),
        "capsule" => capsule(0.5, 2.0, 24, 8),
        "cylinder" => cylinder(0.5, 2.0, 24),
        "plane" => grid_plane(10.0, 10),
        "quad" => quad(),
        _ => return None,
    })
}

fn cube() -> MeshData {
    let mut m = MeshData::default();
    // Each face: its normal and two in-plane axes (u right, v up as seen from outside).
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (n, u, v) in faces {
        let corner =
            |su: f32, sv: f32| [0, 1, 2].map(|k| n[k] * 0.5 + u[k] * su * 0.5 + v[k] * sv * 0.5);
        let base = m.positions.len() as u32;
        m.push(corner(-1.0, -1.0), n, [0.0, 1.0]);
        m.push(corner(1.0, -1.0), n, [1.0, 1.0]);
        m.push(corner(1.0, 1.0), n, [1.0, 0.0]);
        m.push(corner(-1.0, 1.0), n, [0.0, 0.0]);
        m.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    m
}

fn sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
    let mut m = MeshData::default();
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let phi = v * PI;
        for s in 0..=segments {
            let u = s as f32 / segments as f32;
            let theta = u * TAU;
            let n = [phi.sin() * theta.cos(), phi.cos(), -phi.sin() * theta.sin()];
            m.push(n.map(|c| c * radius), n, [u, v]);
        }
    }
    grid_indices(&mut m, 0, segments, rings);
    m
}

/// Triangles over a (segments+1)×(rows+1) grid of vertices starting at `base`.
fn grid_indices(m: &mut MeshData, base: u32, segments: u32, rows: u32) {
    let w = segments + 1;
    for r in 0..rows {
        for s in 0..segments {
            let a = base + r * w + s;
            let b = a + w;
            m.indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
}

fn capsule(radius: f32, height: f32, segments: u32, cap_rings: u32) -> MeshData {
    let mut m = MeshData::default();
    let half = (height / 2.0 - radius).max(0.0);
    // Top cap, then the bottom cap, joined by the cylinder band between them.
    let rows = cap_rings * 2 + 1;
    for r in 0..=rows {
        let (phi, offset) = if r <= cap_rings {
            (r as f32 / cap_rings as f32 * PI / 2.0, half)
        } else {
            ((r - 1) as f32 / cap_rings as f32 * PI / 2.0, -half)
        };
        for s in 0..=segments {
            let u = s as f32 / segments as f32;
            let theta = u * TAU;
            let n = [phi.sin() * theta.cos(), phi.cos(), -phi.sin() * theta.sin()];
            let p = [n[0] * radius, n[1] * radius + offset, n[2] * radius];
            m.push(p, n, [u, 0.5 - p[1] / height]);
        }
    }
    grid_indices(&mut m, 0, segments, rows);
    m
}

fn cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
    let mut m = MeshData::default();
    let h = height / 2.0;
    for (y, v) in [(h, 0.0), (-h, 1.0)] {
        for s in 0..=segments {
            let u = s as f32 / segments as f32;
            let theta = u * TAU;
            let n = [theta.cos(), 0.0, -theta.sin()];
            m.push([n[0] * radius, y, n[2] * radius], n, [u, v]);
        }
    }
    grid_indices(&mut m, 0, segments, 1);
    // Caps: a centre and a ring each.
    for (y, ny) in [(h, 1.0f32), (-h, -1.0)] {
        let center = m.push([0.0, y, 0.0], [0.0, ny, 0.0], [0.5, 0.5]);
        for s in 0..=segments {
            let theta = s as f32 / segments as f32 * TAU;
            let (c, sn) = (theta.cos(), -theta.sin());
            m.push(
                [c * radius, y, sn * radius],
                [0.0, ny, 0.0],
                [0.5 + c * 0.5, 0.5 + sn * 0.5],
            );
        }
        for s in 0..segments {
            let (a, b) = (center + 1 + s, center + 2 + s);
            if ny > 0.0 {
                m.indices.extend([center, a, b]);
            } else {
                m.indices.extend([center, b, a]);
            }
        }
    }
    m
}

fn grid_plane(size: f32, cells: u32) -> MeshData {
    let mut m = MeshData::default();
    for r in 0..=cells {
        for c in 0..=cells {
            let (u, v) = (c as f32 / cells as f32, r as f32 / cells as f32);
            m.push(
                [(u - 0.5) * size, 0.0, (v - 0.5) * size],
                [0.0, 1.0, 0.0],
                [u, v],
            );
        }
    }
    grid_indices(&mut m, 0, cells, cells);
    m
}

fn quad() -> MeshData {
    let mut m = MeshData::default();
    let n = [0.0, 0.0, 1.0];
    m.push([-0.5, -0.5, 0.0], n, [0.0, 1.0]);
    m.push([0.5, -0.5, 0.0], n, [1.0, 1.0]);
    m.push([0.5, 0.5, 0.0], n, [1.0, 0.0]);
    m.push([-0.5, 0.5, 0.0], n, [0.0, 0.0]);
    m.indices.extend([0, 1, 2, 0, 2, 3]);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Twice the signed volume enclosed (positive when triangles face outwards).
    fn volume(m: &MeshData) -> f32 {
        m.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.positions[i as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    // M5-MSH-01
    #[test]
    fn primitives_are_closed_outward_facing_and_unity_sized() {
        for name in ["cube", "sphere", "capsule", "cylinder", "plane", "quad"] {
            let m = primitive(name).unwrap();
            assert!(m.is_valid(), "{name}");
            for n in &m.normals {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-4, "{name} normal {n:?}");
            }
        }
        // Volumes (positive: faces wind outwards).
        let near = |a: f32, b: f32, tol: f32| (a - b).abs() < tol;
        assert!(near(volume(&cube()), 1.0, 1e-4));
        let sphere_v = 4.0 / 3.0 * PI * 0.125;
        assert!(near(volume(&primitive("sphere").unwrap()), sphere_v, 0.03));
        let capsule_v = PI * 0.25 * 1.0 + sphere_v;
        assert!(near(
            volume(&primitive("capsule").unwrap()),
            capsule_v,
            0.05
        ));
        assert!(near(
            volume(&primitive("cylinder").unwrap()),
            PI * 0.25 * 2.0,
            0.05
        ));
        assert!(primitive("teapot").is_none());

        // Missing normals are made from the triangles.
        let mut tri = MeshData {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            ..Default::default()
        };
        tri = tri.complete();
        assert!(tri.is_valid());
        assert_eq!(tri.normals[0], [0.0, 0.0, 1.0]);
    }
}
