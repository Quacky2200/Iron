// Film grain: fresh noise every frame, grains `size` pixels across, strongest in the midtones as
// film's is (`response` 0 is even everywhere); `colored` mixes in noise that differs per channel.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let p = floor(uv * fx.size.xy / max(size, 0.5));
    let f = f32(frame_index() % 64u);
    let at = p + vec2<f32>(f * 17.13, f * 31.77);
    // Two uniform values summed: a triangular spread, so grain is mostly small.
    let mono = hash21(at) + hash21(at * 1.37 + vec2<f32>(5.1, 9.2)) - 1.0;
    let rgb = vec3<f32>(
        hash21(at + vec2<f32>(1.7, 3.1)) + hash21(at * 1.9 + vec2<f32>(2.3, 7.7)) - 1.0,
        hash21(at + vec2<f32>(8.1, 2.9)) + hash21(at * 2.3 + vec2<f32>(4.4, 1.2)) - 1.0,
        hash21(at + vec2<f32>(5.5, 6.1)) + hash21(at * 2.9 + vec2<f32>(9.3, 3.3)) - 1.0,
    );
    let n = mix(vec3<f32>(mono), rgb, clamp(colored, 0.0, 1.0));
    let l = clamp(luma(to_gamma(color.rgb)), 0.0, 1.0);
    let weight = mix(1.0, 4.0 * l * (1.0 - l), clamp(response, 0.0, 1.0));
    // Added to how the picture looks, then back to light.
    let g = to_gamma(color.rgb) + n * intensity * weight * 0.5;
    return vec4<f32>(to_linear(g), color.a);
}
