// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Darkens (or tints) the edges. `roundness` 1 is a circle, 0 follows the view's shape;
// `smoothness` is how far in the falloff starts.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    var p = view_uv(uv) - 0.5;
    p.x = p.x * mix(1.0, aspect(), clamp(roundness, 0.0, 1.0));
    let d = length(p) * 1.4142;
    let t = smoothstep(0.35 + (1.0 - smoothness) * 0.35, 1.25, d);
    return vec4<f32>(mix(color.rgb, to_linear(tint), clamp(t * intensity, 0.0, 1.0)), color.a);
}
