// Cel-shaded bands: the brightness is cut into `levels` steps (judged as it looks, with the
// edges between steps `softness` wide) and the colours keep their hue, so lit surfaces look
// flat and inked, which pairs with `outline`.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let l = max(luma(color.rgb), 1e-4);
    let g = pow(l, 1.0 / 2.2) * levels;
    let band = floor(g);
    let s = smoothstep(0.5 - softness, 0.5 + softness, g - band);
    let q = pow(max((band + s) / levels, 1e-4), 2.2);
    return vec4<f32>(color.rgb * (q / l), color.a);
}
