// SPDX-License-Identifier: Apache-2.0

// Environment cube maps: a procedural sky drawn into each face, faces copied in from 2D
// renders (reflection probes), and the mip chain blurred down for rough reflections.

struct Sky {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    ground: vec4<f32>,
    // xyz: towards the sun; w: the cosine of the sun's angular radius.
    sun_dir: vec4<f32>,
    // rgb: the sun's colour (HDR); w: the face being drawn.
    sun_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> sky: Sky;

// The blits' source (bindings apart from the sky's: each pipeline uses one set).
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

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

// The direction a cube face's texel looks along (WebGPU's face order and orientation).
fn face_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let s = uv.x * 2.0 - 1.0;
    let t = uv.y * 2.0 - 1.0;
    switch face {
        case 0u: { return normalize(vec3<f32>(1.0, -t, -s)); }
        case 1u: { return normalize(vec3<f32>(-1.0, -t, s)); }
        case 2u: { return normalize(vec3<f32>(s, 1.0, t)); }
        case 3u: { return normalize(vec3<f32>(s, -1.0, -t)); }
        case 4u: { return normalize(vec3<f32>(s, -t, 1.0)); }
        default: { return normalize(vec3<f32>(-s, -t, -1.0)); }
    }
}

// The sky seen along `d`: zenith to horizon to ground gradients, and the sun.
fn sky_color(d: vec3<f32>) -> vec3<f32> {
    let y = d.y;
    var c: vec3<f32>;
    if (y >= 0.0) {
        c = mix(sky.horizon.rgb, sky.zenith.rgb, pow(y, 0.5));
    } else {
        // A soft step from the horizon down to the ground.
        c = mix(sky.horizon.rgb, sky.ground.rgb, smoothstep(0.0, 0.12, -y));
    }
    let cos_sun = dot(d, normalize(sky.sun_dir.xyz));
    if (sky.sun_dir.w < 1.0) {
        // A disc with a soft edge, and a glow around it.
        let disc = smoothstep(sky.sun_dir.w - 0.0005, sky.sun_dir.w + 0.0005, cos_sun);
        let glow = pow(max(cos_sun, 0.0), 64.0) * 0.08;
        c += sky.sun_color.rgb * (disc + glow);
    }
    return c;
}

@fragment
fn fs_sky(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(sky_color(face_dir(u32(sky.sun_color.w), in.uv)), 1.0);
}

// A 4-tap box: each texel of the next mip averages the 2 × 2 under it, plus a little of
// the neighbours (the bilinear taps sit off-centre), so each mip is blurrier.
@fragment
fn fs_down(in: Out) -> @location(0) vec4<f32> {
    let t = 0.75 / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, in.uv + vec2<f32>(-t.x, -t.y));
    c += textureSample(src, samp, in.uv + vec2<f32>(t.x, -t.y));
    c += textureSample(src, samp, in.uv + vec2<f32>(-t.x, t.y));
    c += textureSample(src, samp, in.uv + vec2<f32>(t.x, t.y));
    return c * 0.25;
}

// Copies a 2D render into a cube face, mirrored left to right (a camera's image and a
// cube face are mirror images of each other).
@fragment
fn fs_copy_flip(in: Out) -> @location(0) vec4<f32> {
    return textureSample(src, samp, vec2<f32>(1.0 - in.uv.x, in.uv.y));
}
