// Colour fringes that grow towards the edges: red and blue slide apart along the line from the
// middle of the view, with the colours between spread over eight samples.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let c = view_uv(uv) - 0.5;
    let off = c * amount * fx.rect.zw;
    var sum = vec3<f32>(0.0);
    var weight = vec3<f32>(0.0);
    for (var i = 0; i < 8; i++) {
        let t = f32(i) / 7.0;
        let w = vec3<f32>(
            clamp(1.0 - 2.0 * t, 0.0, 1.0),
            1.0 - abs(2.0 * t - 1.0),
            clamp(2.0 * t - 1.0, 0.0, 1.0),
        );
        sum += scene(uv + off * (t * 2.0 - 1.0)).rgb * w;
        weight += w;
    }
    return vec4<f32>(sum / weight, color.a);
}
