// Colour grading. Exposure (in stops) and white balance work on the light; lift / gamma / gain,
// contrast, saturation and hue work on how the picture looks (gamma space), like a colourist's
// wheels: lift moves the shadows, gamma the midtones, gain the highlights.
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    var c = color.rgb * exp2(exposure);
    // balance.x: temperature (warm to cool, -1..1), balance.y: tint (green to magenta).
    c = c * vec3<f32>(
        1.0 + balance.x * 0.25 + balance.y * 0.1,
        1.0 - balance.y * 0.2,
        1.0 - balance.x * 0.25 + balance.y * 0.1,
    );
    var g = to_gamma(c);
    g = g * gain + lift;
    g = pow(max(g, vec3<f32>(0.0)), vec3<f32>(1.0) / max(gamma, vec3<f32>(0.01)));
    g = (g - 0.5) * contrast + 0.5;
    g = mix(vec3<f32>(luma(g)), g, saturation);
    if (abs(hue_shift) > 0.01) {
        let to_yiq = mat3x3<f32>(
            vec3<f32>(0.299, 0.596, 0.211),
            vec3<f32>(0.587, -0.274, -0.523),
            vec3<f32>(0.114, -0.322, 0.312),
        );
        let from_yiq = mat3x3<f32>(
            vec3<f32>(1.0, 1.0, 1.0),
            vec3<f32>(0.956, -0.272, -1.106),
            vec3<f32>(0.621, -0.647, 1.703),
        );
        var yiq = to_yiq * g;
        let a = radians(hue_shift);
        yiq = vec3<f32>(
            yiq.x,
            yiq.y * cos(a) - yiq.z * sin(a),
            yiq.y * sin(a) + yiq.z * cos(a),
        );
        g = from_yiq * yiq;
    }
    return vec4<f32>(to_linear(g), color.a);
}
