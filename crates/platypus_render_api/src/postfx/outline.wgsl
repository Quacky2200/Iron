// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Ink outlines along silhouettes and creases, like a comic or Borderlands. Silhouettes are found
// as jumps in depth (the second difference of depth over a cross of neighbours, relative to the
// distance, so a slanted floor is not an edge), creases as turns in the normals rebuilt from
// depth. `fade_distance` thins the lines with distance; `color_threshold` also inks hard
// brightness edges (texture detail, shadows).
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    // The picture's own border has no neighbours (clamped reads would ink it).
    let border = pixel() * 2.0;
    if (any(uv < border) || any(uv > vec2<f32>(1.0) - border)) {
        return color;
    }
    let d0 = scene_depth(uv);
    var t = max(thickness, 0.25);
    if (fade_distance > 0.0) {
        t = t * mix(1.0, 0.25, clamp(d0 / fade_distance, 0.0, 1.0));
    }
    let o = pixel() * t;
    let ua = uv + vec2<f32>(o.x, 0.0);
    let ub = uv - vec2<f32>(o.x, 0.0);
    let uc = uv + vec2<f32>(0.0, o.y);
    let ud = uv - vec2<f32>(0.0, o.y);
    let da = scene_depth(ua);
    let db = scene_depth(ub);
    let dc = scene_depth(uc);
    let dd = scene_depth(ud);
    let second = max(abs(da + db - 2.0 * d0), abs(dc + dd - 2.0 * d0)) / max(d0, 0.01);
    var edge = smoothstep(depth_threshold, depth_threshold * 2.0, second);

    let n0 = scene_normal(uv);
    let na = scene_normal(ua);
    let nb = scene_normal(ub);
    let nc = scene_normal(uc);
    let nd = scene_normal(ud);
    let all_solid = min(min(length(n0), length(na)), min(min(length(nb), length(nc)), length(nd)));
    if (all_solid > 0.5) {
        let turn = max(
            max(1.0 - dot(n0, na), 1.0 - dot(n0, nb)),
            max(1.0 - dot(n0, nc), 1.0 - dot(n0, nd)),
        );
        edge = max(edge, smoothstep(normal_threshold * 0.5, normal_threshold, turn));
    }

    if (color_threshold > 0.0) {
        let la = luma(to_gamma(scene(ua).rgb));
        let lb = luma(to_gamma(scene(ub).rgb));
        let lc = luma(to_gamma(scene(uc).rgb));
        let ld = luma(to_gamma(scene(ud).rgb));
        let l = max(abs(la - lb), abs(lc - ld));
        edge = max(edge, smoothstep(color_threshold, color_threshold * 2.0, l));
    }
    return vec4<f32>(mix(color.rgb, ink.rgb, clamp(edge * ink.a, 0.0, 1.0)), color.a);
}
