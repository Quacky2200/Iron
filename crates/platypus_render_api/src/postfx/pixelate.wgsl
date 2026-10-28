// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Big square pixels of `size` screen pixels, sharp-edged, optionally with the colours cut to
// `levels` steps per channel (0 keeps them all).
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let s = max(size, 1.0);
    let origin = fx.rect.xy * fx.size.xy;
    let cell = floor((uv * fx.size.xy - origin) / s);
    var c = scene_px(vec2<i32>((cell + 0.5) * s + origin));
    if (levels >= 2.0) {
        let g = to_gamma(c.rgb);
        c = vec4<f32>(to_linear(floor(g * (levels - 1.0) + 0.5) / (levels - 1.0)), c.a);
    }
    return c;
}
