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
    clip: f32,  // clip table index (1-based, 0 = none)
    blend: f32, // blend mode for fs_composite_blend (BlendMode::index)
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

// --- Blend modes ----------------------------------------------------------------
// W3C compositing: the layer (source) blends with a copy of what's beneath it
// (backdrop), in sRGB like browsers. The pass replaces the destination pixels.

@group(0) @binding(5) var backdrop: texture_2d<f32>;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3(0.0031308));
}

fn from_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3(2.4));
    return select(high, low, c <= vec3(0.04045));
}

fn hard_light(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let multiply = cb * 2.0 * cs;
    let s = 2.0 * cs - 1.0;
    let screen = cb + s - cb * s;
    return select(screen, multiply, cs <= vec3(0.5));
}

fn soft_light(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let d = select(sqrt(cb), ((16.0 * cb - 12.0) * cb + 4.0) * cb, cb <= vec3(0.25));
    let dark = cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
    let light = cb + (2.0 * cs - 1.0) * (d - cb);
    return select(light, dark, cs <= vec3(0.5));
}

fn color_dodge(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let r = min(vec3(1.0), cb / max(1.0 - cs, vec3(1e-6)));
    return select(select(r, vec3(1.0), cs >= vec3(1.0)), vec3(0.0), cb <= vec3(0.0));
}

fn color_burn(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let r = 1.0 - min(vec3(1.0), (1.0 - cb) / max(cs, vec3(1e-6)));
    return select(select(r, vec3(0.0), cs <= vec3(0.0)), vec3(1.0), cb >= vec3(1.0));
}

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.3, 0.59, 0.11));
}

fn clip_color(c: vec3<f32>) -> vec3<f32> {
    let l = lum(c);
    let n = min(c.r, min(c.g, c.b));
    let x = max(c.r, max(c.g, c.b));
    var out = c;
    if n < 0.0 {
        out = l + (out - l) * l / max(l - n, 1e-6);
    }
    if x > 1.0 {
        out = l + (out - l) * (1.0 - l) / max(x - l, 1e-6);
    }
    return out;
}

fn set_lum(c: vec3<f32>, l: f32) -> vec3<f32> {
    return clip_color(c + (l - lum(c)));
}

fn sat(c: vec3<f32>) -> f32 {
    return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
}

// The spec's SetSat: the largest channel becomes `s`, the smallest 0, the middle
// one scales between them.
fn set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let n = min(c.r, min(c.g, c.b));
    let range = sat(c);
    if range <= 0.0 {
        return vec3(0.0);
    }
    return (c - n) * s / range;
}

fn blend(mode: u32, cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    switch mode {
        case 1u: { return cb * cs; }
        case 2u: { return cb + cs - cb * cs; }
        case 3u: { return hard_light(cs, cb); }
        case 4u: { return min(cb, cs); }
        case 5u: { return max(cb, cs); }
        case 6u: { return color_dodge(cb, cs); }
        case 7u: { return color_burn(cb, cs); }
        case 8u: { return hard_light(cb, cs); }
        case 9u: { return soft_light(cb, cs); }
        case 10u: { return abs(cb - cs); }
        case 11u: { return cb + cs - 2.0 * cb * cs; }
        case 12u: { return set_lum(set_sat(cs, sat(cb)), lum(cb)); }
        case 13u: { return set_lum(set_sat(cb, sat(cs)), lum(cb)); }
        case 14u: { return set_lum(cs, lum(cb)); }
        case 15u: { return set_lum(cb, lum(cs)); }
        default: { return cs; }
    }
}

@fragment
fn fs_composite_blend(in: VsOut) -> @location(0) vec4<f32> {
    let s = textureSampleLevel(src, samp, uv_at(in.pos), 0.0) * composite.opacity;
    let b = textureLoad(backdrop, vec2<i32>(in.pos.xy), 0);
    let cs = to_srgb(s.rgb / max(s.a, 1e-6));
    let cb = to_srgb(b.rgb / max(b.a, 1e-6));
    let mixed = clamp(blend(u32(composite.blend), cb, cs), vec3(0.0), vec3(1.0));
    // Source-over with the blended colour where both are present.
    let a = s.a + b.a * (1.0 - s.a);
    let c = s.a * (1.0 - b.a) * cs + b.a * (1.0 - s.a) * cb + s.a * b.a * mixed;
    let straight = select(vec3(0.0), c / a, a > 0.0);
    let result = vec4(from_srgb(straight) * a, a);
    let cover = clip_coverage_with(in.pos.xy / composite.scale, u32(composite.clip), composite.scale);
    return mix(b, result, cover);
}

// --- Blit ---------------------------------------------------------------------

@fragment
fn fs_blit(in: VsOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(src, samp, uv_at(in.pos), 0.0);
}
