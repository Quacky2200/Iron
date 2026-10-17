// Bloom: bright parts of the HDR world image, blurred over a chain of half-size
// textures and added back (like Unity's Bloom: threshold, intensity, scatter). The
// composite also applies exposure and tonemapping (Unity's post exposure and
// Tonemapping: none, neutral, ACES).

struct Params {
    threshold: f32,
    // Soft knee: how gradually pixels start to bloom around the threshold.
    knee: f32,
    intensity: f32,
    // A multiplier (2 to the post exposure in stops).
    exposure: f32,
    // 0 none (clipped at white), 1 neutral, 2 ACES.
    tonemap: u32,
    // Scalars, not a vec3 (which would align to 16 bytes and grow the struct).
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> p: Params;
// The full HDR image, for the composite.
@group(0) @binding(3) var base: texture_2d<f32>;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> Out {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var o: Out;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

// Dual-filter downsample: the centre and four diagonal half-texel taps.
fn down(uv: vec2<f32>) -> vec3<f32> {
    let t = 0.5 / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, uv).rgb * 4.0;
    c += textureSample(src, samp, uv + vec2<f32>(-t.x, -t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(t.x, -t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(-t.x, t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(t.x, t.y)).rgb;
    return c / 8.0;
}

// A 3×3 tent filter from the smaller level.
fn up(uv: vec2<f32>) -> vec3<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, uv).rgb * 4.0;
    c += textureSample(src, samp, uv + vec2<f32>(-t.x, 0.0)).rgb * 2.0;
    c += textureSample(src, samp, uv + vec2<f32>(t.x, 0.0)).rgb * 2.0;
    c += textureSample(src, samp, uv + vec2<f32>(0.0, -t.y)).rgb * 2.0;
    c += textureSample(src, samp, uv + vec2<f32>(0.0, t.y)).rgb * 2.0;
    c += textureSample(src, samp, uv + vec2<f32>(-t.x, -t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(t.x, -t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(-t.x, t.y)).rgb;
    c += textureSample(src, samp, uv + vec2<f32>(t.x, t.y)).rgb;
    return c / 16.0;
}

@fragment
fn fs_prefilter(in: Out) -> @location(0) vec4<f32> {
    let c = down(in.uv);
    let brightness = max(c.r, max(c.g, c.b));
    var soft = clamp(brightness - p.threshold + p.knee, 0.0, 2.0 * p.knee);
    soft = soft * soft / (4.0 * p.knee + 1e-4);
    let keep = max(soft, brightness - p.threshold) / max(brightness, 1e-4);
    return vec4<f32>(c * keep, 1.0);
}

@fragment
fn fs_down(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(down(in.uv), 1.0);
}

@fragment
fn fs_up(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(up(in.uv), 1.0);
}

// Khronos PBR Neutral: keeps colours as they are up to a bright shoulder, then
// desaturates them smoothly towards white.
fn neutral(c: vec3<f32>) -> vec3<f32> {
    let start = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = min(c.r, min(c.g, c.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    var color = c - offset;
    let peak = max(color.r, max(color.g, color.b));
    if (peak < start) {
        return color;
    }
    let d = 1.0 - start;
    let new_peak = 1.0 - d * d / (peak + d - start);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), g);
}

// ACES filmic, Narkowicz's fit: contrasty, highlights roll off to white.
fn aces(c: vec3<f32>) -> vec3<f32> {
    let x = c * 0.6;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn grade(c: vec3<f32>) -> vec3<f32> {
    let x = max(c * p.exposure, vec3<f32>(0.0));
    if (p.tonemap == 1u) {
        return neutral(x);
    }
    if (p.tonemap == 2u) {
        return aces(x);
    }
    return x;
}

@fragment
fn fs_composite(in: Out) -> @location(0) vec4<f32> {
    let b = textureSample(base, samp, in.uv);
    return vec4<f32>(grade(b.rgb + up(in.uv) * p.intensity), b.a);
}

// Exposure and tonemapping without bloom.
@fragment
fn fs_grade(in: Out) -> @location(0) vec4<f32> {
    let b = textureSample(base, samp, in.uv);
    return vec4<f32>(grade(b.rgb), b.a);
}
