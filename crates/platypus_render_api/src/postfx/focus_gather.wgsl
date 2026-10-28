// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Out-of-focus blur by gathering: 48 taps on a spiral over a disc (or a polygon with `blades`
// sides), each counted only if its own blur circle reaches this pixel. What is behind a pixel
// can't smear over it, so a sharp edge in front of a blurred background stays sharp, and a
// blurred foreground does spill over what is sharp behind it. Bright taps count more, which makes
// highlights into bokeh.
//
// The including shader defines `coc_at(uv)` (the blur circle's radius in pixels at `uv`) and
// `behind(tap, uv)` (whether `tap` is further away than `uv`).
const TAPS: i32 = 48;
const GOLDEN: f32 = 2.39996323;
const PI: f32 = 3.14159265;

fn gather(uv: vec2<f32>, color: vec4<f32>, reach: f32) -> vec4<f32> {
    let centre = coc_at(uv);
    var sum = color.rgb;
    var total = 1.0;
    let sides = floor(blades);
    for (var i = 0; i < TAPS; i++) {
        var r = sqrt((f32(i) + 0.5) / f32(TAPS)) * reach;
        let theta = f32(i) * GOLDEN;
        if (sides >= 3.0) {
            let seg = 2.0 * PI / sides;
            let a = theta - seg * floor(theta / seg) - 0.5 * seg;
            r = r * cos(PI / sides) / cos(a);
        }
        let tap = uv + vec2<f32>(cos(theta), sin(theta)) * r * pixel();
        var coc = coc_at(tap);
        if (behind(tap, uv)) {
            coc = min(coc, centre);
        }
        let length_px = length(vec2<f32>(cos(theta), sin(theta)) * r);
        var w = clamp(coc - length_px + 1.0, 0.0, 1.0);
        let c = scene(tap).rgb;
        w = w * (1.0 + highlight * pow(luma(c), 2.0) * 4.0);
        sum += c * w;
        total += w;
    }
    return vec4<f32>(sum / total, color.a);
}
