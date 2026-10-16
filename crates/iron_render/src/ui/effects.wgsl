// Full-screen passes for UI effects: separable Gaussian blur, compositing a
// layer (with opacity and an optional clip from the clip table), and the final blit.
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
    opacity: f32,
    scale: f32,
    clip: f32, // clip table index (1-based, 0 = none)
    _pad: f32,
};
@group(0) @binding(3) var<uniform> composite: CompositeParams;
@group(0) @binding(4) var<storage, read> clips: array<Clip>;

// --- Clips (overflow, clip-path) ---------------------------------------------
// A table of rounded rectangles with elliptical corners. Entries chain to their
// parent (1-based index, 0 = none); coverage is the product along the chain.

struct Clip {
    rect: vec4<f32>,   // logical px
    rx: vec4<f32>,     // corner radii x: top-left, top-right, bottom-right, bottom-left
    ry: vec4<f32>,     // corner radii y
    bounds: vec4<f32>, // the whole chain's bounding box (x, y, w, h)
    info: vec4<f32>,   // x: parent index (1-based, 0 = none)
};

fn clip_corner(q: vec2<f32>, radii: vec4<f32>) -> f32 {
    let top = select(radii.x, radii.y, q.x > 0.0);
    let bottom = select(radii.w, radii.z, q.x > 0.0);
    return select(top, bottom, q.y > 0.0);
}

// Approximate signed distance (logical px) to a box with elliptical corners.
fn sd_clip(p: vec2<f32>, c: Clip) -> f32 {
    let half = c.rect.zw * 0.5;
    let q0 = p - (c.rect.xy + half);
    let edge = abs(q0) - half;
    let r = vec2(min(clip_corner(q0, c.rx), half.x), min(clip_corner(q0, c.ry), half.y));
    if r.x > 0.0 && r.y > 0.0 {
        let q = edge + r; // relative to the corner's centre
        if q.x > 0.0 && q.y > 0.0 {
            return (length(q / r) - 1.0) * min(r.x, r.y);
        }
    }
    return min(max(edge.x, edge.y), 0.0) + length(max(edge, vec2(0.0)));
}

fn clip_coverage_with(p: vec2<f32>, index: u32, scale: f32) -> f32 {
    var i = index;
    var cover = 1.0;
    for (var n = 0; n < 32 && i > 0u && cover > 0.0; n++) {
        let c = clips[i - 1u];
        cover *= clamp(0.5 - sd_clip(p, c) * scale, 0.0, 1.0);
        i = u32(c.info.x);
    }
    return cover;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(src, samp, uv_at(in.pos), 0.0) * composite.opacity;
    return color * clip_coverage_with(in.pos.xy / composite.scale, u32(composite.clip), composite.scale);
}

// --- Blit ---------------------------------------------------------------------

@fragment
fn fs_blit(in: VsOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(src, samp, uv_at(in.pos), 0.0);
}
