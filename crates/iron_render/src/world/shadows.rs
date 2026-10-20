//! Shadow maps, drawn once a frame from the main view and shared by every view that
//! frame (other cameras, mirrors, probe faces, render textures).
//!
//! - The sun (the first directional light with shadows) gets 1 to 4 cascades: slices of
//!   the main camera's view out to the shadow distance, each fitted with a sphere and an
//!   orthographic map, moved only in whole texels so edges don't crawl. One cascade
//!   covers a sphere in front of the camera instead.
//! - Spot lights get a perspective map each and point lights a cube (six 90° maps), in a
//!   second texture array at half the resolution, nearest lights first, up to
//!   [`MAX_LOCAL_LAYERS`] layers.
//!
//! The shader finds a light's matrices by index ([`ShadowSlot::first`]): cascades first,
//! then local layers.

use iron_core::glam::{Mat4, Vec3, Vec4Swizzles};

use super::{Light, LightKind, Shadows, WorldCamera, cull, environment};

/// The most cascades the sun's map has.
pub const MAX_CASCADES: u32 = 4;
/// The most layers spot (1 each) and point (6 each) lights share.
pub const MAX_LOCAL_LAYERS: u32 = 24;

/// A light casting shadows this frame and where its matrices are.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShadowSlot {
    pub light: Light,
    pub shadows: Shadows,
    /// Its first matrix: 0 for the sun (its cascades), else `MAX_CASCADES` + its first
    /// local layer.
    pub first: u32,
    /// For local lights: a shadow texel's size in metres at 1 m from the light (the
    /// normal offset grows with distance).
    pub texel_at_1m: f32,
}

/// The frame's shadow maps: who casts, and the matrices to draw and read them with.
#[derive(Clone, Debug, Default)]
pub(crate) struct ShadowPlan {
    pub slots: Vec<ShadowSlot>,
    /// The sun's cascades (0: no sun shadows).
    pub cascades: u32,
    /// Per cascade: a texel's size in metres.
    pub cascade_texels: [f32; 4],
    /// `MAX_CASCADES` slots for the sun (unused ones identity), then local layers.
    pub matrices: Vec<Mat4>,
    /// How many local layers are used.
    pub local_layers: u32,
}

impl ShadowPlan {
    /// The slot of a light (matched by value: views share their light lists).
    pub fn slot(&self, light: &Light) -> Option<&ShadowSlot> {
        self.slots.iter().find(|s| s.light == *light)
    }
}

/// Settings the plan follows (from the renderer's quality).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlanSettings {
    pub distance: f32,
    pub resolution: u32,
    pub local_resolution: u32,
    pub cascades: u32,
}

/// Which lights cast shadows this frame, as seen from `camera` with `lights`.
pub(crate) fn plan(camera: &WorldCamera, lights: &[Light], s: PlanSettings) -> ShadowPlan {
    let mut plan = ShadowPlan {
        matrices: vec![Mat4::IDENTITY; MAX_CASCADES as usize],
        ..Default::default()
    };
    let casting = |l: &&Light| l.shadows.is_some_and(|sh| sh.strength > 0.0);
    if let Some(sun) = lights
        .iter()
        .filter(casting)
        .find(|l| l.kind == LightKind::Directional)
    {
        let count = s.cascades.clamp(1, MAX_CASCADES);
        let fitted = cascades(camera, sun.direction, s.distance, count, s.resolution);
        for (i, (m, texel)) in fitted.iter().enumerate() {
            plan.matrices[i] = *m;
            plan.cascade_texels[i] = *texel;
        }
        plan.cascades = count;
        plan.slots.push(ShadowSlot {
            light: *sun,
            shadows: sun.shadows.expect("casting"),
            first: 0,
            texel_at_1m: 0.0,
        });
    }
    // Local lights reaching the view, nearest first.
    let frustum = cull::Frustum::from_view_proj(camera.view_proj);
    let mut locals: Vec<&Light> = lights
        .iter()
        .filter(casting)
        .filter(|l| l.kind != LightKind::Directional)
        .filter(|l| frustum.sphere(l.position, l.range))
        .collect();
    locals.sort_by(|a, b| {
        a.position
            .distance_squared(camera.position)
            .total_cmp(&b.position.distance_squared(camera.position))
    });
    let res = s.local_resolution.max(1) as f32;
    for l in locals {
        let near = 0.05;
        let far = l.range.max(near + 0.1);
        let (faces, fov): (Vec<(Vec3, Vec3)>, f32) = match l.kind {
            LightKind::Spot { outer_degrees, .. } => {
                let dir = l.direction.normalize_or(Vec3::NEG_Z);
                let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                // A little wider than the cone, so its soft edge has a map under it.
                let fov = (outer_degrees * 1.1 + 2.0).clamp(2.0, 170.0);
                (vec![(dir, up)], fov)
            }
            _ => ((0..6).map(environment::face_camera).collect(), 90.0),
        };
        let layers = faces.len() as u32;
        if plan.local_layers + layers > MAX_LOCAL_LAYERS {
            continue;
        }
        let proj = iron_core::glam::camera::rh::proj::directx::perspective(
            fov.to_radians(),
            1.0,
            near,
            far,
        );
        let first = MAX_CASCADES + plan.local_layers;
        for (forward, up) in faces {
            let view = iron_core::glam::camera::rh::view::look_at_mat4(
                l.position,
                l.position + forward,
                up,
            );
            plan.matrices.push(proj * view);
        }
        plan.local_layers += layers;
        plan.slots.push(ShadowSlot {
            light: *l,
            shadows: l.shadows.expect("casting"),
            first,
            texel_at_1m: 2.0 * (fov.to_radians() / 2.0).tan() / res,
        });
    }
    plan
}

/// The sun's cascade matrices and texel sizes for `camera`, out to `distance`.
fn cascades(
    camera: &WorldCamera,
    direction: Vec3,
    distance: f32,
    count: u32,
    resolution: u32,
) -> Vec<(Mat4, f32)> {
    let dir = direction.normalize_or(Vec3::NEG_Y);
    let res = resolution.max(1) as f32;
    if count <= 1 {
        // One map: a sphere `distance` across in front of the camera.
        let radius = (distance / 2.0).max(0.5);
        let center = camera.position + camera.forward * radius;
        return vec![(fit(center, radius, dir, res), 2.0 * radius / res)];
    }
    // The view's edges, from its near corners to its far corners.
    let inv = camera.view_proj.inverse();
    let eye = camera.position;
    let fwd = camera.forward;
    let edges: Vec<(Vec3, Vec3)> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|&(x, y)| {
            let at = |z: f32| {
                let p = inv * iron_core::glam::Vec4::new(x, y, z, 1.0);
                p.xyz() / p.w
            };
            (at(0.0), at(1.0))
        })
        .collect();
    let depth = |p: Vec3| (p - eye).dot(fwd);
    let near = edges
        .iter()
        .map(|e| depth(e.0))
        .fold(f32::MAX, f32::min)
        .max(0.05);
    let far_edge = edges.iter().map(|e| depth(e.1)).fold(0.0f32, f32::max);
    let far = distance.min(far_edge).max(near + 0.1);
    // Practical splits: halfway between even and logarithmic.
    let split = |i: u32| {
        let t = i as f32 / count as f32;
        let log = near * (far / near).powf(t);
        let even = near + (far - near) * t;
        0.5 * log + 0.5 * even
    };
    let point_at = |e: &(Vec3, Vec3), d: f32| {
        let (dn, df) = (depth(e.0), depth(e.1));
        let t = if (df - dn).abs() < 1e-6 {
            0.0
        } else {
            ((d - dn) / (df - dn)).clamp(0.0, 1.0)
        };
        e.0.lerp(e.1, t)
    };
    (0..count)
        .map(|i| {
            let (d0, d1) = (split(i), split(i + 1));
            let points: Vec<Vec3> = edges
                .iter()
                .flat_map(|e| [point_at(e, d0), point_at(e, d1)])
                .collect();
            let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;
            let radius = points
                .iter()
                .map(|p| p.distance(center))
                .fold(0.0f32, f32::max);
            // Rounded up, so small camera turns don't change the size (and shimmer).
            let radius = ((radius * 16.0).ceil() / 16.0).max(0.5);
            (fit(center, radius, dir, res), 2.0 * radius / res)
        })
        .collect()
}

/// An orthographic light view of a sphere, moved only in whole texels.
fn fit(center: Vec3, radius: f32, dir: Vec3, res: f32) -> Mat4 {
    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    // Far enough back to catch casters outside the sphere (tall things, hills).
    let back = (radius * 3.0).max(30.0);
    let eye = center - dir * back;
    let view = iron_core::glam::camera::rh::view::look_at_mat4(eye, center, up);
    let texel = 2.0 * radius / res;
    let at = view.transform_point3(center);
    let snap = Vec3::new(
        (at.x / texel).round() * texel - at.x,
        (at.y / texel).round() * texel - at.y,
        0.0,
    );
    let proj = iron_core::glam::camera::rh::proj::directx::orthographic(
        -radius,
        radius,
        -radius,
        radius,
        0.01,
        back + radius * 2.0,
    );
    proj * Mat4::from_translation(snap) * view
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ALL_LAYERS, Color};

    fn light(kind: LightKind, position: Vec3) -> Light {
        Light {
            kind,
            color: [1.0; 3],
            position,
            direction: Vec3::new(0.0, -1.0, 0.2).normalize(),
            range: 10.0,
            shadows: Some(Shadows {
                soft: true,
                strength: 1.0,
            }),
            culling_mask: ALL_LAYERS,
            volumetric: 1.0,
        }
    }

    // M5-SHD-01
    #[test]
    fn cascades_cover_the_view_and_local_lights_share_a_layer_budget() {
        let camera = WorldCamera::perspective(
            iron_core::glam::camera::rh::view::look_at_mat4(
                Vec3::new(0.0, 2.0, 10.0),
                Vec3::ZERO,
                Vec3::Y,
            )
            .inverse(),
            60.0,
            0.1,
            500.0,
            (1600, 900),
            Color::rgb(0.0, 0.0, 0.0),
        );
        let settings = PlanSettings {
            distance: 60.0,
            resolution: 2048,
            local_resolution: 1024,
            cascades: 4,
        };
        let mut lights = vec![light(LightKind::Directional, Vec3::ZERO)];
        // Five point lights (6 layers each) in view: only four fit in 24 layers.
        for i in 0..5 {
            lights.push(light(LightKind::Point, Vec3::new(i as f32, 1.0, 0.0)));
        }
        lights.push(light(
            LightKind::Spot {
                inner_degrees: 20.0,
                outer_degrees: 30.0,
            },
            Vec3::new(0.0, 3.0, 5.0),
        ));
        let plan = plan(&camera, &lights, settings);
        assert_eq!(plan.cascades, 4);
        // Cascades grow with distance, and each covers a point at the middle of its slice.
        assert!(plan.cascade_texels.windows(2).all(|w| w[0] <= w[1]));
        for (i, d) in [0.5f32, 4.0, 15.0, 45.0].iter().enumerate() {
            let p = camera.position + camera.forward * *d;
            let ndc = plan.matrices[i].project_point3(p);
            assert!(ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0, "cascade {i}");
        }
        // The spot (nearest the camera) and three points: 1 + 18 layers; the rest don't fit.
        assert_eq!(plan.local_layers, 1 + 6 * 3);
        assert_eq!(plan.slots.len(), 1 + 4);
        assert_eq!(plan.slots[1].first, MAX_CASCADES);
        assert!(matches!(plan.slots[1].light.kind, LightKind::Spot { .. }));
        assert_eq!(plan.matrices.len() as u32, MAX_CASCADES + plan.local_layers);
        assert!(plan.slot(&lights[0]).is_some_and(|s| s.first == 0));
        // A point light's face for +x sees a point to its +x.
        let point = plan.slots[2];
        let p = point.light.position + Vec3::X * 2.0;
        let ndc = plan.matrices[point.first as usize].project_point3(p);
        assert!(ndc.x.abs() < 1e-3 && ndc.y.abs() < 1e-3 && ndc.z > 0.0 && ndc.z < 1.0);
    }
}
