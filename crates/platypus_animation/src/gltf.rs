// SPDX-License-Identifier: Apache-2.0

//! Clips from a glTF file's animation channels (`Model.clips` in `js/engine/src/models.ts`): one
//! curve per component, paths by node names from the copy's root.

use crate::clip::{Clip, ClipId, World};
use crate::curve::{Curve, Keyframe, PointMode, WrapMode};

/// One channel: the object it moves (a path from the copy's root), what it moves
/// (`translation`, `rotation` or `scale`), how it interpolates (`linear`, `step` or `cubic`) and
/// its keys (cubic splines bring in-tangent, value, out-tangent per key).
pub struct Channel<'a> {
    pub path: &'a str,
    pub target: &'a str,
    pub interpolation: &'a str,
    pub times: &'a [f64],
    pub values: &'a [f64],
}

/// Rotation keys made sign-continuous: q and -q are the same turn, and exporters (Blender's)
/// sometimes switch sides from one key to the next. Blended component by component, a key pair
/// on opposite sides passes near zero between them, and normalised that is a wild pose for a
/// frame or two (a leg flung up mid-stride). Each key is flipped to its predecessor's side;
/// cubic keys (in-tangent, value, out-tangent) flip as a whole.
pub fn continuous_rotations(values: &mut [f64], cubic: bool) {
    let (stride, at) = if cubic { (12, 4) } else { (4, 0) };
    let mut i = stride;
    while i + stride <= values.len() {
        let dot: f64 = (0..4)
            .map(|c| values[i - stride + at + c] * values[i + at + c])
            .sum();
        if dot < 0.0 {
            for v in &mut values[i..i + stride] {
                *v = -*v;
            }
        }
        i += stride;
    }
}

/// A clip (looping) from a glTF animation's channels.
pub fn clip_from_channels(world: &mut World, name: &str, channels: &[Channel]) -> ClipId {
    let mut clip = Clip::new(name);
    clip.wrap_mode = WrapMode::Loop;
    let id = world.add_clip(clip);
    for ch in channels {
        let (property, size) = match ch.target {
            "translation" => ("localPosition", 3),
            "rotation" => ("localRotation", 4),
            _ => ("localScale", 3),
        };
        let cubic = ch.interpolation == "cubic";
        let mut values = ch.values.to_vec();
        if ch.target == "rotation" {
            continuous_rotations(&mut values, cubic);
        }
        for k in 0..size {
            let at = |i: usize| values.get(i).copied().unwrap_or(0.0);
            let curve = if cubic {
                // The file's tangents as they are.
                Curve::new(
                    ch.times
                        .iter()
                        .enumerate()
                        .map(|(i, &t)| Keyframe {
                            time: t,
                            in_tangent: at(i * 3 * size + k),
                            value: at((i * 3 + 1) * size + k),
                            out_tangent: at((i * 3 + 2) * size + k),
                        })
                        .collect(),
                )
            } else {
                let points: Vec<(f64, f64)> = ch
                    .times
                    .iter()
                    .enumerate()
                    .map(|(i, &t)| (t, at(i * size + k)))
                    .collect();
                let mode = if ch.interpolation == "step" {
                    PointMode::Step
                } else {
                    PointMode::Linear
                };
                Curve::from_points(&points, mode)
            };
            let curve = world.add_curve(curve);
            let axis = &"xyzw"[k..=k];
            world.set_curve(
                id,
                ch.path,
                "Transform",
                &format!("{property}.{axis}"),
                Some(curve),
            );
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-SKN-04
    #[test]
    fn rotation_keys_that_switch_sides_are_flipped_to_agree() {
        // A quarter turn about y, then the same turn written as -q, then back.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let mut v = vec![0.0, s, 0.0, s, 0.0, -s, 0.0, -s, 0.0, s, 0.0, s];
        continuous_rotations(&mut v, false);
        assert_eq!(v, [0.0, s, 0.0, s, 0.0, s, 0.0, s, 0.0, s, 0.0, s]);
        // Cubic keys flip with their tangents (in, value, out).
        let mut c = vec![0.0; 24];
        c[4..8].copy_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        c[12..24].copy_from_slice(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 2.0, 0.0, 0.0]);
        continuous_rotations(&mut c, true);
        assert_eq!(
            &c[12..24],
            &[-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -2.0, 0.0, 0.0]
        );
    }

    // M5-SKN-04
    #[test]
    fn sampling_between_keys_on_opposite_sides_keeps_the_turn() {
        let mut world = World::default();
        let s = std::f64::consts::FRAC_1_SQRT_2;
        // A half-turn's worth of keys about y; the second written on the other side.
        let values = [0.0, s, 0.0, s, 0.0, -s, 0.0, -s];
        let clip = clip_from_channels(
            &mut world,
            "turn",
            &[Channel {
                path: "Leg",
                target: "rotation",
                interpolation: "linear",
                times: &[0.0, 1.0],
                values: &values,
            }],
        );
        let c = world.clip(clip).expect("clip");
        assert_eq!(c.wrap_mode, WrapMode::Loop);
        let at = |prop: &str| {
            let b = c.bindings.iter().find(|b| b.property == prop).unwrap();
            world.curve(b.curve).unwrap().evaluate(0.5)
        };
        // Halfway it is still the same quarter turn (not near zero).
        assert!((at("localRotation.y") - s).abs() < 1e-9);
        assert!((at("localRotation.w") - s).abs() < 1e-9);
    }
}
