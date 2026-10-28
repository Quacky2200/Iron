// Tilt-shift: a band across the view stays sharp and the rest blurs the further it is from the
// band, which makes a scene look like a miniature. The band can be turned by `angle` (degrees).
fn coc_at(uv: vec2<f32>) -> f32 {
    var p = view_uv(uv) - 0.5;
    let a = radians(angle);
    // Along the band's normal, in fractions of the view's height.
    let along = -p.x * sin(a) * aspect() + p.y * cos(a);
    let dist = abs(along - (focus - 0.5));
    return clamp((dist - width) / 0.35, 0.0, 1.0) * max_blur;
}

fn behind(tap: vec2<f32>, uv: vec2<f32>) -> bool {
    return false;
}

fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    if (max_blur < 0.5) {
        return color;
    }
    return gather(uv, color, max_blur);
}
