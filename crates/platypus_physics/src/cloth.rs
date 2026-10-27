// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Cloth (Unity's `Cloth`): a mesh's vertices as particles, simulated with position-based
//! dynamics (Verlet integration, then constraint passes). Edges keep their length
//! (stretching), pairs across shared edges keep theirs (bending), pinned vertices stay
//! where the object holds them, and spheres and capsules push vertices out of them.
//!
//! Positions are in world space; [`Cloth::local_positions`] gives them back in the
//! object's space for drawing with the object's matrix.

use std::collections::HashMap;

use platypus_core::glam::{Mat4, Vec3};

/// How a cloth behaves (Unity's `Cloth` settings).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothSettings {
    /// 0 (rubbery) to 1 (keeps its edges' lengths).
    pub stretching_stiffness: f32,
    /// 0 (folds freely) to 1 (resists folding).
    pub bending_stiffness: f32,
    /// 0..1: how much of its motion it loses per second.
    pub damping: f32,
    /// Acceleration on every vertex (gravity plus `externalAcceleration`).
    pub acceleration: Vec3,
    /// Up to this much more, varying (wind gusts; Unity's `randomAcceleration`).
    pub random_acceleration: Vec3,
    /// 0..1: how much sliding along colliders slows it.
    pub friction: f32,
    /// Constraint passes per step: more is stiffer and slower.
    pub iterations: u32,
}

impl Default for ClothSettings {
    fn default() -> Self {
        Self {
            stretching_stiffness: 1.0,
            bending_stiffness: 0.3,
            damping: 0.1,
            acceleration: Vec3::new(0.0, -9.81, 0.0),
            random_acceleration: Vec3::ZERO,
            friction: 0.5,
            iterations: 8,
        }
    }
}

/// Something a cloth can't pass through, in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClothCollider {
    Sphere {
        center: Vec3,
        radius: f32,
    },
    /// A segment from `a` to `b`, swept by a sphere of `radius`.
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
}

impl ClothCollider {
    /// Where `p` is pushed to (outside, plus a small margin), if it's inside.
    fn push_out(&self, p: Vec3, margin: f32) -> Option<Vec3> {
        let (closest, radius) = match *self {
            ClothCollider::Sphere { center, radius } => (center, radius),
            ClothCollider::Capsule { a, b, radius } => {
                let ab = b - a;
                let t = if ab.length_squared() > 0.0 {
                    ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (a + ab * t, radius)
            }
        };
        let d = p - closest;
        let r = radius + margin;
        let len = d.length();
        if len >= r {
            return None;
        }
        let dir = if len > 1e-6 { d / len } else { Vec3::Y };
        Some(closest + dir * r)
    }
}

#[derive(Clone, Copy, Debug)]
struct Link {
    a: usize,
    b: usize,
    rest: f32,
}

/// A simulated cloth.
#[derive(Clone, Debug)]
pub struct Cloth {
    /// The mesh's vertices in the object's space as it was set up (pinned ones stay here).
    rest: Vec<Vec3>,
    /// Which mesh vertex each particle is (welded: vertices at the same spot share one).
    vertex_particle: Vec<usize>,
    /// Each particle's first vertex, in its rest position.
    particle_rest: Vec<Vec3>,
    pinned: Vec<bool>,
    positions: Vec<Vec3>,
    previous: Vec<Vec3>,
    stretch: Vec<Link>,
    bend: Vec<Link>,
    pub settings: ClothSettings,
    pub colliders: Vec<ClothCollider>,
    /// A small random generator for gusts (xorshift).
    seed: u32,
    time: f32,
}

impl Cloth {
    /// A cloth from a mesh (object-space vertices and triangles) placed at `world`,
    /// with `pinned(vertex position)` choosing the vertices held in place.
    pub fn new(
        vertices: &[Vec3],
        indices: &[u32],
        world: Mat4,
        pinned: impl Fn(Vec3) -> bool,
    ) -> Self {
        // Weld vertices at the same spot (seams in UVs or normals split them).
        let mut key_particle: HashMap<[i32; 3], usize> = HashMap::new();
        let mut vertex_particle = Vec::with_capacity(vertices.len());
        let mut particle_rest = Vec::new();
        for v in vertices {
            let key = (*v * 1e4).round().as_ivec3().to_array();
            let p = *key_particle.entry(key).or_insert_with(|| {
                particle_rest.push(*v);
                particle_rest.len() - 1
            });
            vertex_particle.push(p);
        }
        let positions: Vec<Vec3> = particle_rest
            .iter()
            .map(|v| world.transform_point3(*v))
            .collect();
        let pins = particle_rest.iter().map(|v| pinned(*v)).collect();
        // Edges once each, and the far corners of triangles sharing an edge (bending).
        let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for t in indices.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| vertex_particle[i as usize]);
            for (x, y, other) in [(a, b, c), (b, c, a), (c, a, b)] {
                if x == y {
                    continue;
                }
                edges.entry((x.min(y), x.max(y))).or_default().push(other);
            }
        }
        let rest_len = |a: usize, b: usize| particle_rest[a].distance(particle_rest[b]);
        let mut stretch = Vec::new();
        let mut bend = Vec::new();
        let mut bent: std::collections::HashSet<(usize, usize)> = Default::default();
        let mut keys: Vec<_> = edges.keys().copied().collect();
        keys.sort_unstable();
        for (a, b) in keys {
            stretch.push(Link {
                a,
                b,
                rest: rest_len(a, b),
            });
            let others = &edges[&(a, b)];
            if let [c, d] = others[..]
                && c != d
                && bent.insert((c.min(d), c.max(d)))
            {
                bend.push(Link {
                    a: c,
                    b: d,
                    rest: rest_len(c, d),
                });
            }
        }
        Self {
            rest: vertices.to_vec(),
            vertex_particle,
            particle_rest,
            pinned: pins,
            previous: positions.clone(),
            positions,
            stretch,
            bend,
            settings: ClothSettings::default(),
            colliders: Vec::new(),
            seed: 0x9e37_79b9,
            time: 0.0,
        }
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// One step of `dt` seconds with the object at `world` (pinned vertices follow it).
    pub fn step(&mut self, dt: f32, world: Mat4) {
        if dt <= 0.0 {
            return;
        }
        self.time += dt;
        let s = self.settings;
        // Gusts change smoothly: a slow wave times a little noise.
        let gust = 0.5 + 0.5 * (self.time * 1.7).sin() * (self.time * 0.6).cos();
        let noise = Vec3::new(self.random(), self.random(), self.random()) * 0.3;
        let accel = s.acceleration + s.random_acceleration * (gust + noise);
        let keep = (1.0 - s.damping.clamp(0.0, 1.0)).powf(dt);
        for i in 0..self.positions.len() {
            if self.pinned[i] {
                let p = world.transform_point3(self.particle_rest[i]);
                self.previous[i] = p;
                self.positions[i] = p;
                continue;
            }
            let p = self.positions[i];
            let velocity = (p - self.previous[i]) * keep;
            self.previous[i] = p;
            self.positions[i] = p + velocity + accel * dt * dt;
        }
        let iterations = s.iterations.max(1);
        for _ in 0..iterations {
            Self::solve(
                &mut self.positions,
                &self.pinned,
                &self.stretch,
                s.stretching_stiffness,
            );
            Self::solve(
                &mut self.positions,
                &self.pinned,
                &self.bend,
                s.bending_stiffness,
            );
        }
        // Colliders last: nothing ends a step inside one. Friction keeps a share of the
        // movement along the surface from carrying over.
        let margin = 0.01;
        for i in 0..self.positions.len() {
            if self.pinned[i] {
                continue;
            }
            for c in &self.colliders {
                if let Some(out) = c.push_out(self.positions[i], margin) {
                    let moved = out - self.previous[i];
                    self.positions[i] = out;
                    self.previous[i] += moved * s.friction.clamp(0.0, 1.0);
                }
            }
        }
    }

    /// Moves linked particles towards their rest distance, by `stiffness` (0..1).
    fn solve(positions: &mut [Vec3], pinned: &[bool], links: &[Link], stiffness: f32) {
        let k = stiffness.clamp(0.0, 1.0);
        if k <= 0.0 {
            return;
        }
        for l in links {
            let (wa, wb) = (
                if pinned[l.a] { 0.0 } else { 1.0 },
                if pinned[l.b] { 0.0 } else { 1.0 },
            );
            let w = wa + wb;
            if w == 0.0 {
                continue;
            }
            let d = positions[l.b] - positions[l.a];
            let len = d.length();
            if len < 1e-9 {
                continue;
            }
            let correction = d * ((len - l.rest) / len) * k / w;
            positions[l.a] += correction * wa;
            positions[l.b] -= correction * wb;
        }
    }

    /// Each mesh vertex's position in the object's space (for drawing with its matrix).
    pub fn local_positions(&self, world: Mat4) -> Vec<[f32; 3]> {
        let inverse = world.inverse();
        self.vertex_particle
            .iter()
            .map(|&p| inverse.transform_point3(self.positions[p]).to_array())
            .collect()
    }

    /// Particles' world positions.
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    /// The mesh's vertices in object space when it was set up.
    pub fn rest(&self) -> &[Vec3] {
        &self.rest
    }

    pub fn particle_count(&self) -> usize {
        self.positions.len()
    }

    /// Whether particle `i` is pinned.
    pub fn is_pinned(&self, i: usize) -> bool {
        self.pinned[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An n × n grid in the xz plane, 1 m across, centred on the origin.
    fn grid(n: usize) -> (Vec<Vec3>, Vec<u32>) {
        let mut v = Vec::new();
        for z in 0..=n {
            for x in 0..=n {
                v.push(Vec3::new(
                    x as f32 / n as f32 - 0.5,
                    0.0,
                    z as f32 / n as f32 - 0.5,
                ));
            }
        }
        let w = (n + 1) as u32;
        let mut idx = Vec::new();
        for z in 0..n as u32 {
            for x in 0..n as u32 {
                let a = z * w + x;
                idx.extend([a, a + w, a + 1, a + 1, a + w, a + w + 1]);
            }
        }
        (v, idx)
    }

    // M5-CLO-01
    #[test]
    fn a_cloth_hangs_from_its_pins_keeping_its_size() {
        let (v, idx) = grid(10);
        // Held along one edge (z = -0.5), the grid lying flat at 2 m up.
        let world = Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0));
        let mut cloth = Cloth::new(&v, &idx, world, |p| p.z < -0.49);
        cloth.settings.iterations = 20;
        for _ in 0..240 {
            cloth.step(1.0 / 60.0, world);
        }
        // Pinned particles stayed; the free edge hangs about a metre below them.
        let pinned_y: Vec<f32> = (0..cloth.particle_count())
            .filter(|&i| cloth.is_pinned(i))
            .map(|i| cloth.positions()[i].y)
            .collect();
        assert_eq!(pinned_y.len(), 11);
        assert!(pinned_y.iter().all(|y| (y - 2.0).abs() < 1e-5));
        let lowest = cloth
            .positions()
            .iter()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min);
        assert!((0.9..1.15).contains(&lowest), "lowest {lowest}");
        // Edges kept (nearly) their length.
        let worst = cloth
            .stretch
            .iter()
            .map(|l| (cloth.positions[l.a].distance(cloth.positions[l.b]) / l.rest - 1.0).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 0.05, "stretched {worst}");
        // Moving the object carries the pins with it.
        let moved = Mat4::from_translation(Vec3::new(1.0, 2.0, 0.0));
        cloth.step(1.0 / 60.0, moved);
        let p = (0..cloth.particle_count())
            .find(|&i| cloth.is_pinned(i))
            .unwrap();
        assert!((cloth.positions()[p].x - (cloth.particle_rest[p].x + 1.0)).abs() < 1e-5);
        // Back in the object's space: a value per mesh vertex.
        assert_eq!(cloth.local_positions(moved).len(), v.len());
    }

    // M5-CLO-01
    #[test]
    fn a_falling_cloth_drapes_over_a_sphere_without_passing_through() {
        let (v, idx) = grid(16);
        let world = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.0),
            platypus_core::glam::Quat::IDENTITY,
            Vec3::new(0.0, 1.5, 0.0),
        );
        let mut cloth = Cloth::new(&v, &idx, world, |_| false);
        let sphere = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 0.6,
        };
        cloth.colliders.push(sphere);
        for _ in 0..180 {
            cloth.step(1.0 / 60.0, world);
        }
        for p in cloth.positions() {
            assert!(p.length() >= 0.6, "inside the sphere at {p}");
        }
        // The middle rests on top; the corners hang below it.
        let top = cloth
            .positions()
            .iter()
            .map(|p| p.y)
            .fold(f32::MIN, f32::max);
        let low = cloth
            .positions()
            .iter()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min);
        assert!((0.6..0.7).contains(&top), "top {top}");
        assert!(low < 0.2, "lowest {low}");
        // Welding: a mesh with a seam (duplicated vertices) makes one particle per spot.
        let (mut v2, mut idx2) = grid(1);
        v2.push(v2[1]);
        idx2[2] = 4;
        let seam = Cloth::new(&v2, &idx2, Mat4::IDENTITY, |_| false);
        assert_eq!(seam.particle_count(), 4);
        assert_eq!(seam.local_positions(Mat4::IDENTITY).len(), 5);
        // A capsule pushes out sideways.
        let capsule = ClothCollider::Capsule {
            a: Vec3::ZERO,
            b: Vec3::Y,
            radius: 0.5,
        };
        let out = capsule.push_out(Vec3::new(0.1, 0.5, 0.0), 0.0).unwrap();
        assert!((out - Vec3::new(0.5, 0.5, 0.0)).length() < 1e-5);
        assert!(capsule.push_out(Vec3::new(2.0, 0.5, 0.0), 0.0).is_none());
    }
}
