// Full-screen passes for UI effects: separable Gaussian blur, compositing a
// layer (with opacity and an optional rounded clip), and the final blit.
// All passes draw one full-screen triangle; scissor rects limit the work.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vi: u32) -> VsOut {
    var points = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var out: VsOut;
    out.pos = vec4(points[vi], 0.0, 1.0);
    return out;
}

fn uv_at(pos: vec4<f32>) -> vec2<f32> {
    return pos.xy / vec2<f32>(textureDimensions(src));
}

// --- Blur -------------------------------------------------------------------

struct BlurParams {
    dir: vec2<f32>, // (1, 0) or (0, 1)
    sigma: f32,     // physical pixels
    radius: f32,    // taps either side
};
@group(0) @binding(2) var<uniform> blur: BlurParams;

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src));
    let uv = uv_at(in.pos);
    let r = i32(blur.radius);
    var sum = vec4(0.0);
    var weights = 0.0;
    for (var i = -r; i <= r; i++) {
        let x = f32(i);
        let w = exp(-(x * x) / (2.0 * blur.sigma * blur.sigma));
        sum += textureSampleLevel(src, samp, uv + blur.dir * x * texel, 0.0) * w;
        weights += w;
    }
    return sum / weights;
}

// --- Composite ----------------------------------------------------------------

struct CompositeParams {
    clip_rect: vec4<f32>,  // logical px
    clip_radii: vec4<f32>,
    opacity: f32,
    scale: f32,
    use_clip: f32,
    _pad: f32,
};
@group(0) @binding(3) var<uniform> composite: CompositeParams;

fn corner_radius(q: vec2<f32>, radii: vec4<f32>) -> f32 {
    let top = select(radii.x, radii.y, q.x > 0.0);
    let bottom = select(radii.w, radii.z, q.x > 0.0);
    return select(top, bottom, q.y > 0.0);
}

fn sd_rounded_box(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let half = rect.zw * 0.5;
    let q0 = p - (rect.xy + half);
    let r = min(corner_radius(q0, radii), min(half.x, half.y));
    let q = abs(q0) - half + vec2(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    var color = textureSampleLevel(src, samp, uv_at(in.pos), 0.0) * composite.opacity;
    if composite.use_clip > 0.5 {
        let d = sd_rounded_box(in.pos.xy / composite.scale, composite.clip_rect, composite.clip_radii);
        color *= clamp(0.5 - d * composite.scale, 0.0, 1.0);
    }
    return color;
}

// --- Blit ---------------------------------------------------------------------

@fragment
fn fs_blit(in: VsOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(src, samp, uv_at(in.pos), 0.0);
}
