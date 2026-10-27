// A gaussian blur in two passes: along x, then along y. The radius (in pixels) is two sigmas,
// taken with 33 taps spaced radius / 16 apart.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let r = max(radius, 0.0);
    if (r < 0.01) {
        return color;
    }
    var dir = vec2<f32>(0.0, 1.0);
    if (pass_index() == 0u) {
        dir = vec2<f32>(1.0, 0.0);
    }
    let step = dir * pixel() * (r / 16.0);
    var sum = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = -16; i <= 16; i++) {
        let x = f32(i) / 8.0;
        let w = exp(-0.5 * x * x);
        sum += scene(uv + step * f32(i)).rgb * w;
        total += w;
    }
    return vec4<f32>(sum / total, color.a);
}
