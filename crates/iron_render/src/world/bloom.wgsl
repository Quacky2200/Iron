// SPDX-License-Identifier: Apache-2.0

// Bloom: bright parts of the HDR world image, blurred over a chain of half-size
// textures and added back (like Unity's Bloom: threshold, intensity, scatter). The
// composite also applies exposure (each camera's manual or auto exposure, then Unity's post
// exposure) and tonemapping (none, neutral, ACES).

struct Params {
    threshold: f32,
    // Soft knee: how gradually pixels start to bloom around the threshold.
    knee: f32,
    intensity: f32,
    // A multiplier (2 to the post exposure in stops).
    exposure: f32,
    // 0 none (clipped at white), 1 neutral, 2 ACES.
    tonemap: u32,
    // Auto exposure: which part of the view counts (0 all alike, 1 centre-weighted, 2
    // spot); seconds since the last frame; stops added; the averages (EV100) it follows
    // between; how fast it adapts as the view brightens and darkens.
    metering: u32,
    delta_time: f32,
    compensation: f32,
    min_ev: f32,
    max_ev: f32,
    dark_to_light: f32,
    light_to_dark: f32,
    // The part of the histogram averaged, percent from the darkest.
    low: f32,
    high: f32,
    // The manual exposure's stops (0: light as it is).
    fixed_stops: f32,
    // Compensation curve: points (EV100, stops), two to a vec4.
    curve_points: u32,
    // The soft floor: below `dark_below` (EV100) it adapts only `dark_adaptation` of the way.
    dark_adaptation: f32,
    dark_below: f32,
    _pad0: f32,
    _pad1: f32,
    curve: array<vec4<f32>, 4>,
};

// A view's part of the image and its camera's state.
struct View {
    // x, y, width, height, 0..1.
    rect: vec4<f32>,
    // 0: the manual exposure, 1: auto exposure's.
    weight: f32,
    // 1: jump straight to the metered exposure.
    snap: u32,
    // 1: don't adapt.
    hold: u32,
    // What the world was pre-exposed by on its way into the HDR image (1 outside HD).
    pre: f32,
    // The manual exposure's stops, eased towards `Params::fixed_stops` over the transition.
    fixed: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> p: Params;
// The full HDR image, for the composite.
@group(0) @binding(3) var base: texture_2d<f32>;
// The camera's auto exposure state: x the stops adapted to so far, y 1 once set, z the
// metered EV100. The composite reads it; `cs_meter` updates it.
@group(0) @binding(4) var<storage, read> meter: vec4<f32>;
@group(0) @binding(5) var<storage, read_write> meter_out: vec4<f32>;
@group(0) @binding(6) var<uniform> view: View;

// The histogram: log2 luminance from HIST_MIN over HIST_RANGE stops in HIST_BINS bins
// (0.1875 stops each): starlight to the sun's glare in real-world units (the HD pipeline).
const HIST_BINS: u32 = 256u;
const HIST_MIN: f32 = -16.0;
const HIST_RANGE: f32 = 48.0;
var<workgroup> bins: array<atomic<u32>, 256>;

fn curve_point(i: u32) -> vec2<f32> {
    let v = p.curve[i / 2u];
    return select(v.xy, v.zw, (i % 2u) == 1u);
}

// The compensation curve at `ev`: straight lines between its points, flat past the ends.
fn curve_at(ev: f32) -> f32 {
    let n = p.curve_points;
    if (n == 0u) {
        return 0.0;
    }
    var prev = curve_point(0u);
    if (ev <= prev.x) {
        return prev.y;
    }
    for (var i = 1u; i < n; i++) {
        let next = curve_point(i);
        if (ev <= next.x) {
            let t = (ev - prev.x) / max(next.x - prev.x, 1e-4);
            return mix(prev.y, next.y, t);
        }
        prev = next;
    }
    return prev.y;
}

// Auto exposure for one view: its part of the HDR image is metered on a 64 × 64 grid into
// a histogram of log luminance (weighted by the metering mode), the part between the low and
// high percentiles averaged, and the exposure that shows that average as mid grey (HDRP's
// 1 / (1.2 × 2^EV100), plus compensation and its curve) eased towards.
@compute @workgroup_size(16, 16, 1)
fn cs_meter(@builtin(local_invocation_index) i: u32, @builtin(local_invocation_id) id: vec3<u32>) {
    atomicStore(&bins[i], 0u);
    workgroupBarrier();
    let size = vec2<f32>(textureDimensions(base));
    for (var y = 0u; y < 4u; y++) {
        for (var x = 0u; x < 4u; x++) {
            let local = (vec2<f32>(id.xy * 4u + vec2<u32>(x, y)) + 0.5) / 64.0;
            let uv = view.rect.xy + local * view.rect.zw;
            let c = textureLoad(base, vec2<i32>(uv * size), 0).rgb;
            let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)) / view.pre;
            let d = local - 0.5;
            var w = 1.0;
            if (p.metering == 1u) {
                w = exp(-dot(d, d) * 6.0);
            } else if (p.metering == 2u) {
                w = select(0.0, 1.0, length(d) < 0.12);
            }
            let wi = u32(round(w * 64.0));
            if (wi > 0u) {
                let b = u32(clamp((log2(max(l, 1e-7)) - HIST_MIN) / HIST_RANGE * f32(HIST_BINS), 0.0, f32(HIST_BINS - 1u)));
                atomicAdd(&bins[b], wi);
            }
        }
    }
    workgroupBarrier();
    if (i != 0u) {
        return;
    }
    var total = 0.0;
    for (var k = 0u; k < HIST_BINS; k++) {
        total += f32(atomicLoad(&bins[k]));
    }
    if (total <= 0.0) {
        return;
    }
    // Average the bins between the percentiles, taking part of a bin cut by either.
    let lo = total * p.low / 100.0;
    let hi = max(total * p.high / 100.0, lo + 1.0);
    var below = 0.0;
    var sum = 0.0;
    var weight = 0.0;
    for (var k = 0u; k < HIST_BINS; k++) {
        let c = f32(atomicLoad(&bins[k]));
        let part = max(0.0, min(below + c, hi) - max(below, lo));
        sum += part * (HIST_MIN + (f32(k) + 0.5) / f32(HIST_BINS) * HIST_RANGE);
        weight += part;
        below += c;
    }
    // EV100 = log2(L × 100 / 12.5).
    let scene = sum / max(weight, 1e-6) + 3.0;
    // Below the soft floor, adapt only part of the way down.
    let soft = select(scene, p.dark_below + (scene - p.dark_below) * p.dark_adaptation, scene < p.dark_below);
    let ev = clamp(soft, p.min_ev, p.max_ev);
    let target_stops = -ev - log2(1.2) + p.compensation + curve_at(scene);
    var stops = target_stops;
    if (meter_out.y > 0.5 && view.snap == 0u) {
        let current = meter_out.x;
        if (view.hold == 1u) {
            stops = current;
        } else {
            // A lower exposure means the view got brighter.
            let speed = select(p.light_to_dark, p.dark_to_light, target_stops < current);
            stops = current + (target_stops - current) * (1.0 - exp(-p.delta_time * speed));
        }
    }
    meter_out = vec4<f32>(stops, 1.0, scene, 0.0);
}

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

// What the camera's exposure multiplies the light by: the manual exposure blended with auto
// exposure's (by how far it has faded in), before the post exposure.
fn auto_scale() -> f32 {
    let adapted = select(view.fixed, meter.x, meter.y > 0.5);
    // The world came in pre-exposed: divide that back out.
    return exp2(mix(view.fixed, adapted, view.weight)) / view.pre;
}

// The threshold applies to light as auto exposure shows it, so what blooms is what looks
// bright now: a tunnel's exit seen from inside, not the dim lamps. (Post exposure comes
// after, as in Unity.)
@fragment
fn fs_prefilter(in: Out) -> @location(0) vec4<f32> {
    let c = down(in.uv);
    let brightness = max(c.r, max(c.g, c.b)) * auto_scale();
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
    let x = max(c * p.exposure * auto_scale(), vec3<f32>(0.0));
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
