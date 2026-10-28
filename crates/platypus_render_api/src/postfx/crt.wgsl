// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// An old CRT screen: a bulging glass (corners cut off), dark scanlines `line_height` pixels
// apart and a red / green / blue phosphor mask across the columns.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let flat_uv = view_uv(uv);
    var p = flat_uv * 2.0 - 1.0;
    p = p * (1.0 + curvature * dot(p, p) * 0.25);
    let q = p * 0.5 + 0.5;
    if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    var c = scene(fx.rect.xy + q * fx.rect.zw).rgb;
    let row = flat_uv.y * fx.size.y * fx.rect.w;
    c = c * (1.0 - scanlines * (0.5 + 0.5 * cos(row * 6.2831853 / max(line_height, 1.0))));
    var m = vec3<f32>(1.0 - mask);
    m[u32(uv.x * fx.size.x) % 3u] = 1.0;
    c = c * m * (1.0 + mask * 0.5);
    return vec4<f32>(c, color.a);
}
