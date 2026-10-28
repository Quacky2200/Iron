// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Depth of field like a real camera's: things at `focus_distance` are sharp and the rest blur by
// the thin-lens circle of confusion for the focal length and f-stop (on a 24 mm tall sensor),
// up to `max_blur` pixels.
fn coc_at(uv: vec2<f32>) -> f32 {
    let d = max(scene_depth(uv), 0.01);
    let f = focal_length * 0.001;
    let s1 = max(focus_distance, f + 0.01);
    let aperture_m = f / max(aperture, 0.1);
    let blur_m = aperture_m * f * abs(d - s1) / (d * (s1 - f));
    // Diameter on the sensor to a radius in pixels of the view's height.
    let px = blur_m / 0.024 * fx.size.y * fx.rect.w * 0.5;
    return min(px, max_blur);
}

fn behind(tap: vec2<f32>, uv: vec2<f32>) -> bool {
    return scene_depth(tap) > scene_depth(uv) + 0.05;
}

fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    if (max_blur < 0.5) {
        return color;
    }
    return gather(uv, color, max_blur);
}
