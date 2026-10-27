// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The light meter (`Light.illuminanceAt`, `Light.measureIlluminance`): how much light every light
//! sheds on a point, in lux with physical light units. The same sums as `js/engine`'s
//! `rendering.ts`, in the same order of operations.

use crate::{LightData, lights};
use platypus_math::V3;
use platypus_scene::api::scene_api;

fn dot(a: V3, b: V3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// What one light sheds on `point`, facing `normal` if given (head-on otherwise).
fn lux(l: &LightData, point: V3, normal: Option<V3>) -> f64 {
    if !scene_api::active(l.object) {
        return 0.0;
    }
    let forward = scene_api::rotation(l.object).rotate(V3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    });
    if l.r#type == "directional" {
        let cos = normal.map_or(1.0, |n| (-dot(n, forward)).max(0.0));
        return l.intensity * cos;
    }
    let p = scene_api::position(l.object);
    let (dx, dy, dz) = (point.x - p.x, point.y - p.y, point.z - p.z);
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    if d >= l.range || d == 0.0 {
        return 0.0;
    }
    // As the renderer: a smooth fade to zero at the range.
    let x = d / l.range;
    let fade = (1.0 - x * x * x * x) * (1.0 - x * x * x * x);
    // Lumens to candela, all round (HDRP's spots without reflector).
    let mut candela = l.intensity / (4.0 * std::f64::consts::PI);
    if l.r#type == "spot" {
        let c = (dx * forward.x + dy * forward.y + dz * forward.z) / d;
        let outer = ((l.spot_angle / 2.0) * std::f64::consts::PI / 180.0).cos();
        let inner = ((l.inner_spot_angle / 2.0) * std::f64::consts::PI / 180.0).cos();
        let u = ((c - outer) / (inner - outer).max(1e-4)).clamp(0.0, 1.0);
        candela *= u * u * (3.0 - 2.0 * u);
    }
    let cos = normal.map_or(1.0, |n| (-(n.x * dx + n.y * dy + n.z * dz) / d).max(0.0));
    ((candela * fade) / (d * d)) * cos
}

/// The sum over every light; with `shadows`, a light counts only if a physics ray from the point
/// reaches it.
pub(crate) fn illuminance(point: V3, normal: Option<V3>, shadows: bool) -> f64 {
    let mut sum = 0.0;
    for (_, l) in lights() {
        let mut lit = lux(&l, point, normal);
        if shadows && lit > 0.0 {
            let (dir, distance) = if l.r#type == "directional" {
                let f = scene_api::rotation(l.object).rotate(V3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                });
                (
                    V3 {
                        x: -f.x,
                        y: -f.y,
                        z: -f.z,
                    },
                    10000.0,
                )
            } else {
                let p = scene_api::position(l.object);
                let dir = V3 {
                    x: p.x - point.x,
                    y: p.y - point.y,
                    z: p.z - point.z,
                };
                (dir, dot(dir, dir).sqrt() - 0.05)
            };
            // Start a little off the point, so the surface it is on does not block it.
            let m = dot(dir, dir).sqrt();
            let len = if m > 0.0 { m } else { 1.0 };
            let from = V3 {
                x: point.x + (dir.x / len) * 0.02,
                y: point.y + (dir.y / len) * 0.02,
                z: point.z + (dir.z / len) * 0.02,
            };
            if platypus_physics_api::physics_api::raycast(from, dir, distance).0 != 0 {
                lit = 0.0;
            }
        }
        sum += lit;
    }
    sum
}
