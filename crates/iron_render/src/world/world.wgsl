// The 3D world pass: lit meshes and instanced quads (planes) seen through a
// perspective or orthographic camera. Colours are linear and premultiplied.

struct Globals {
    view_proj: mat4x4<f32>,
    // The camera's background (premultiplied), for `fs_fill`.
    background: vec4<f32>,
    // xyz: the camera's position, for specular highlights.
    camera: vec4<f32>,
    // rgb: ambient light; w: how many lights this view has.
    ambient: vec4<f32>,
    // x: the sun's shadow cascades (0: none).
    shadow: vec4<f32>,
    // Each cascade's texel size in metres (for the normal offset).
    cascade_texels: vec4<f32>,
    // Screen to world, for the skybox.
    inv_view_proj: mat4x4<f32>,
    // The viewport in target pixels: x, y, width, height.
    viewport: vec4<f32>,
    // x: 1 with an environment map; y: intensity; z: its last mip; w: 1 for sky ambient.
    env: vec4<f32>,
    // x: 1 to draw the sky behind everything; yz: the target's size in pixels; w: 1 to
    // draw the editor's ground grid.
    flags: vec4<f32>,
    // Fragments where dot(xyz, p) + w < 0 are dropped (all zero: none).
    clip: vec4<f32>,
    // rgb: the fog's colour (linear); w: its mode (0 none, 1 linear, 2 exponential,
    // 3 exponential squared).
    fog: vec4<f32>,
    // x: density per metre; y, z: linear fog's start and end in metres.
    fog_params: vec4<f32>,
    // xyz: the camera's forward axis; w: 1 for orthographic (fog by depth along it).
    view_dir: vec4<f32>,
    // Height fog: x density at the base height (y), thinning by e every 1 / z metres
    // above it; w: 1 when on.
    fog_height: vec4<f32>,
    // Light shafts: x intensity, y anisotropy, z samples per volume slice, w how far the
    // volume reaches (0: off).
    fog_volume: vec4<f32>,
    // rgb: how much of each colour of light the fog scatters into shafts.
    fog_albedo: vec4<f32>,
    // x: how many fog volumes `local_fogs` holds.
    fog_local: vec4<f32>,
    local_fogs: array<LocalFog, 8>,
};

// A box of fog: centre (xyz) and density (w); half extents (xyz) and the soft edge's depth
// (w); its colour (rgb) and w 1 for an ellipsoid inside the box.
struct LocalFog {
    centre: vec4<f32>,
    half: vec4<f32>,
    color: vec4<f32>,
};

struct Quad {
    model: mat4x4<f32>,
    color: vec4<f32>,
    // Texture rectangle: origin (xy) and size (zw) in UV space.
    uv: vec4<f32>,
    // x: 1 when the texture holds straight alpha (premultiplied here).
    params: vec4<f32>,
};

struct MeshInstance {
    model: mat4x4<f32>,
    // The inverse transpose of the model's 3×3 part (normals under non-uniform scale).
    normal: mat4x4<f32>,
    // Straight alpha (premultiplied in the shader).
    color: vec4<f32>,
    // rgb: emission; w: smoothness 0..1.
    emission: vec4<f32>,
    // x: 1 when unlit; y: 1 additive, 2 a light beam; z: metallic 0..1; w: 1 for a planar
    // mirror (its texture is the reflection, sampled at the screen position).
    params: vec4<f32>,
    // Texture tiling: scale (xy), offset (zw), in bottom-up UV space like Unity's.
    tiling: vec4<f32>,
    // The reflection probe it uses (bound in group 2): centre, and mip count (0: none).
    probe: vec4<f32>,
    // The probe's half extents, and 1 for box projection.
    probe_box: vec4<f32>,
    // x: its layer's bit, for lights' masks; y: its maps (MAP_* bits); z: its render
    // mode (0 opaque, 1 cutout, 2 fade, 3 transparent).
    layer: vec4<u32>,
    // rgb: the specular colour; w: 1 for the specular workflow.
    specular: vec4<f32>,
    // x: normal map strength; y: occlusion strength; z: alpha cut-off; w: the first
    // bone + 1 for skinned meshes (0: rigid).
    maps: vec4<f32>,
    // The detail map's tiling: scale (xy), offset (zw).
    detail: vec4<f32>,
};

const MAP_NORMAL: u32 = 1u;
const MAP_METALLIC: u32 = 2u;
const MAP_GLTF: u32 = 4u;
const MAP_OCCLUSION: u32 = 8u;
const MAP_EMISSION: u32 = 16u;
const MAP_DETAIL: u32 = 32u;

struct Light {
    // xyz: position; w: 0 directional, 1 point, 2 spot.
    position: vec4<f32>,
    // xyz: the direction it shines; w: range (point and spot).
    direction: vec4<f32>,
    // rgb: colour × intensity; w: how strongly it lights fog (light shafts).
    color: vec4<f32>,
    // x, y: cosines of the spot's inner and outer half-angles.
    spot: vec4<f32>,
    // x: the layers it lights.
    mask: vec4<u32>,
    // x: its first shadow matrix (-1: none; the sun's are its cascades); y: strength;
    // z: the soft filter's radius in texels (0: hard); w: a texel's size at 1 m (spot
    // and point lights).
    shadow: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var<storage, read> quads: array<Quad>;
@group(0) @binding(2) var<storage, read> meshes: array<MeshInstance>;
@group(0) @binding(3) var<storage, read> lights: array<Light>;
// Skinned meshes' bones: world matrix times inverse bind pose.
@group(0) @binding(4) var<storage, read> bones: array<mat4x4<f32>>;
// The material: its main texture and sampler (shared by the maps), then its maps.
// Normal, metallic and occlusion maps are bound as data (linear), the rest as colour.
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(1) @binding(2) var normal_map: texture_2d<f32>;
@group(1) @binding(3) var metallic_map: texture_2d<f32>;
@group(1) @binding(4) var occlusion_map: texture_2d<f32>;
@group(1) @binding(5) var emission_map: texture_2d<f32>;
@group(1) @binding(6) var detail_map: texture_2d<f32>;
// A mirror's reflection, rendered this frame.
@group(1) @binding(7) var reflection_tex: texture_2d<f32>;
// Shadows (drawn once a frame) and the environment.
@group(2) @binding(0) var sun_shadows: texture_depth_2d_array;
@group(2) @binding(1) var shadow_samp: sampler_comparison;
@group(2) @binding(2) var env_map: texture_cube<f32>;
@group(2) @binding(3) var env_samp: sampler;
@group(2) @binding(4) var local_shadows: texture_depth_2d_array;
// The sun's cascades (4 slots), then spot and point lights' layers.
@group(2) @binding(5) var<storage, read> shadow_matrices: array<mat4x4<f32>>;

const MAX_CASCADES: i32 = 4;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) straight: f32,
    @location(3) world: vec3<f32>,
};

@vertex
fn vs_quad(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    let c = corners[vi];
    let q = quads[ii];
    var out: VsOut;
    let world = q.model * vec4<f32>(c.x, c.y, 0.0, 1.0);
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    // The texture's top-left sits at the quad's top-left corner (-x, +y).
    let t = vec2<f32>(c.x + 0.5, 0.5 - c.y);
    out.uv = q.uv.xy + t * q.uv.zw;
    out.color = q.color;
    out.straight = q.params.x;
    return out;
}

@fragment
fn fs_quad(in: VsOut) -> @location(0) vec4<f32> {
    var t = textureSample(tex, samp, in.uv);
    if (clipped(in.world)) {
        discard;
    }
    if (in.straight > 0.5) {
        t = vec4<f32>(t.rgb * t.a, t.a);
    }
    let c = t * in.color;
    // Fully transparent texels don't write depth (UI panels have see-through parts).
    // Additive quads have no alpha, only light.
    if (c.a <= 0.002 && max(c.r, max(c.g, c.b)) <= 0.002) {
        discard;
    }
    return fogged(c, in.world, in.pos.xy);
}

// Where the fog on the way to `p` starts: the camera, or for orthographic views the
// camera's plane (they look along parallel rays).
fn fog_eye(p: vec3<f32>) -> vec3<f32> {
    if (g.view_dir.w > 0.5) {
        return p - g.view_dir.xyz * max(dot(p - g.camera.xyz, g.view_dir.xyz), 0.0);
    }
    return g.camera.xyz;
}

// How much of what's at `p` shows through the fog (1: all, 0: only fog). Perspective
// cameras measure straight to the point (turning doesn't move the fog); orthographic
// ones by depth along their view, so a 2D scene fogs by layer. Without `distance` (the
// background) only height fog counts.
fn fog_clear(p: vec3<f32>, distance: bool) -> f32 {
    let mode = g.fog.w;
    if (mode < 0.5) {
        return 1.0;
    }
    let eye = fog_eye(p);
    var clear = 1.0;
    if (distance) {
        let d = length(p - eye);
        if (mode < 1.5) {
            let start = g.fog_params.y;
            let end = g.fog_params.z;
            clear = clamp((end - d) / max(end - start, 1e-4), 0.0, 1.0);
        } else if (mode < 2.5) {
            clear = exp(-g.fog_params.x * d);
        } else {
            let x = g.fog_params.x * d;
            clear = exp(-x * x);
        }
    }
    return clear * height_clear(eye, p);
}

// How much light gets through height fog on the way from `a` to `b` (1: all of it). The
// fog's density along the line is exponential in height, so its integral is exact:
// density × length × e^(-falloff (a.y - base)) × (1 - e^(-falloff Δy)) / (falloff Δy).
fn height_clear(a: vec3<f32>, b: vec3<f32>) -> f32 {
    if (g.fog_height.w < 0.5) {
        return 1.0;
    }
    let falloff = g.fog_height.z;
    let k = falloff * (b.y - a.y);
    // Level lines (k near 0) keep the first terms of the series.
    let along = select((1.0 - exp(min(-k, 60.0))) / k, 1.0 - 0.5 * k, abs(k) < 1e-3);
    let at_eye = exp(min(-falloff * (a.y - g.fog_height.y), 60.0));
    return exp(-g.fog_height.x * length(b - a) * at_eye * along);
}

// How much light gets through fog volume `i` on the way from `a` to `b` (1: all of it):
// where the line crosses the box, its density integrated at fixed points (smooth from
// pixel to pixel), thinning over the soft edge.
const LOCAL_FOG_SAMPLES: i32 = 8;

fn local_fog_clear(a: vec3<f32>, b: vec3<f32>, i: u32) -> f32 {
    let v = g.local_fogs[i];
    let len = length(b - a);
    if (len < 1e-4) {
        return 1.0;
    }
    let dir = (b - a) / len;
    let safe = select(dir, vec3<f32>(1e-6), abs(dir) < vec3<f32>(1e-6));
    let lo = (v.centre.xyz - v.half.xyz - a) / safe;
    let hi = (v.centre.xyz + v.half.xyz - a) / safe;
    let near = min(lo, hi);
    let far = max(lo, hi);
    let t0 = max(max(max(near.x, near.y), near.z), 0.0);
    let t1 = min(min(min(far.x, far.y), far.z), len);
    if (t1 <= t0) {
        return 1.0;
    }
    let step = (t1 - t0) / f32(LOCAL_FOG_SAMPLES);
    var depth = 0.0;
    for (var k = 0; k < LOCAL_FOG_SAMPLES; k++) {
        let x = a + dir * (t0 + (f32(k) + 0.5) * step);
        var inside: f32;
        if (v.color.w > 0.5) {
            // How far in from the ellipsoid's surface, roughly in metres.
            let q = (x - v.centre.xyz) / v.half.xyz;
            inside = (1.0 - length(q)) * min(min(v.half.x, v.half.y), v.half.z);
        } else {
            let d = v.half.xyz - abs(x - v.centre.xyz);
            inside = min(min(d.x, d.y), d.z);
        }
        let edge = clamp(inside / max(v.half.w, 1e-4), 0.0, 1.0);
        depth += edge * edge * (3.0 - 2.0 * edge);
    }
    return exp(-v.centre.w * depth * step);
}

// How much of what's at `p` shows through every fog volume.
fn local_clear(eye: vec3<f32>, p: vec3<f32>) -> f32 {
    var clear = 1.0;
    let n = u32(g.fog_local.x);
    for (var i = 0u; i < n; i++) {
        clear *= local_fog_clear(eye, p, i);
    }
    return clear;
}

// A premultiplied colour at `p` seen through the fog volumes, each in its own colour.
fn through_volumes(c: vec4<f32>, eye: vec3<f32>, p: vec3<f32>) -> vec4<f32> {
    var rgb = c.rgb;
    let n = u32(g.fog_local.x);
    for (var i = 0u; i < n; i++) {
        let f = 1.0 - local_fog_clear(eye, p, i);
        rgb = rgb * (1.0 - f) + g.local_fogs[i].color.rgb * (c.a * f);
    }
    return vec4<f32>(rgb, c.a);
}

// A premultiplied colour at `p` (pixel `px`) seen through the fog: what it covers turns
// to fog colour, light it adds (alpha 0) fades out, and lit fog in front adds light.
fn fogged(c: vec4<f32>, p: vec3<f32>, px: vec2<f32>) -> vec4<f32> {
    var out = c;
    if (g.fog.w > 0.5) {
        let f = 1.0 - fog_clear(p, true);
        var rgb = c.rgb * (1.0 - f) + g.fog.rgb * (c.a * f);
        if (g.fog_volume.w > 0.0 && c.a > 0.002) {
            rgb += shafts(p, px, true) * c.a;
        }
        out = vec4<f32>(rgb, c.a);
    }
    if (g.fog_local.x > 0.5) {
        out = through_volumes(out, fog_eye(p), p);
    }
    return out;
}

// Henyey-Greenstein, scaled so scattering evenly (anisotropy 0) gives 1.
fn phase(cos_theta: f32) -> f32 {
    let k = g.fog_volume.y;
    let d = 1.0 + k * k - 2.0 * k * cos_theta;
    return (1.0 - k * k) / (d * sqrt(d));
}

// The light that fog at `x` scatters back along the view ray `dir` from one light: its
// falloff, cone and (hard) shadow.
fn scattered(light: Light, x: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var strength = light.color.w;
    // Which way its light travels at `x`.
    var travel: vec3<f32>;
    if (light.position.w < 0.5) {
        travel = normalize(light.direction.xyz);
    } else {
        let from_light = x - light.position.xyz;
        let d = length(from_light);
        travel = from_light / max(d, 1e-4);
        // As on surfaces: zero at the range.
        let r = clamp(d / max(light.direction.w, 1e-4), 0.0, 1.0);
        let fade = 1.0 - r * r * r * r;
        strength *= fade * fade / (1.0 + d * d);
        if (light.position.w > 1.5) {
            strength *= smoothstep(light.spot.y, light.spot.x, dot(travel, normalize(light.direction.xyz)));
        }
    }
    if (strength <= 0.0) {
        return vec3<f32>(0.0);
    }
    if (light.shadow.x >= 0.0) {
        if (light.position.w < 0.5) {
            strength *= sun_shadow(light, x, vec3<f32>(0.0), 0);
        } else {
            strength *= local_shadow(light, x, vec3<f32>(0.0), 0);
        }
    }
    // Looking straight at a light, its light travels back along the view.
    return light.color.rgb * strength * phase(dot(travel, -dir));
}

// Light shafts' froxel volume (volume.rs): per screen cell and depth slice, the light the
// fog scatters towards the camera from the eye out to that slice. Slices are spaced by
// the square of their index out to the reach (fine nearby, where shafts are big on screen).
@group(3) @binding(0) var volume: texture_3d<f32>;
@group(3) @binding(2) var volume_samp: sampler;
@group(3) @binding(1) var volume_out: texture_storage_3d<rgba16float, write>;


// How far along the view ray slice `w` (0..1 across the slices) reaches.
fn slice_distance(w: f32) -> f32 {
    return g.fog_volume.w * w * w;
}

// Every light that lights fog, scattering at `x` back along the view ray `dir`.
fn scattered_all(x: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = u32(g.ambient.w);
    for (var l = 0u; l < count; l++) {
        let light = lights[l];
        if (light.color.w > 0.0) {
            sum += scattered(light, x, dir);
        }
    }
    return sum;
}

// Fills one column of the volume: the view ray through this screen cell, its fog's share
// in each sub-step lit as `scattered_all` finds it at the sub-step's middle, added up; each
// slice stores the total out to its centre.
@compute @workgroup_size(8, 8, 1)
fn cs_volume(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(volume_out);
    if (id.x >= size.x || id.y >= size.y) {
        return;
    }
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size.xy);
    let far4 = g.inv_view_proj * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 1.0, 1.0);
    let far = far4.xyz / far4.w;
    let eye = fog_eye(far);
    let dir = normalize(far - eye);
    let slices = i32(size.z);
    // Samples per slice: more keep a thin cone near its tip sampled.
    let substeps = i32(g.fog_volume.z);
    var t = 0.0;
    var clear = 1.0;
    var sum = vec3<f32>(0.0);
    for (var k = 0; k < slices; k++) {
        let end = slice_distance((f32(k) + 0.5) / f32(slices));
        let step = (end - t) / f32(substeps);
        for (var i = 0; i < substeps; i++) {
            let at = eye + dir * (t + step);
            let next = fog_clear(at, true) * local_clear(eye, at);
            sum += (clear - next) * scattered_all(eye + dir * (t + 0.5 * step), dir);
            clear = next;
            t += step;
        }
        textureStore(volume_out, vec3<i32>(i32(id.x), i32(id.y), k), vec4<f32>(sum, 1.0));
    }
}

// Light the fog between the camera and `p` (pixel `px`) sends to the camera: the volume
// at `p`, plus, past its last slice, the sun lighting the rest of the fog as at its middle.
// The fog's albedo tints it.
fn shafts(p: vec3<f32>, px: vec2<f32>, distance: bool) -> vec3<f32> {
    let eye = fog_eye(p);
    let len = length(p - eye);
    if (len < 1e-4) {
        return vec3<f32>(0.0);
    }
    let dir = (p - eye) / len;
    let slices = f32(textureDimensions(volume).z);
    let last = slice_distance((slices - 0.5) / slices);
    let uv = (px - g.viewport.xy) / g.viewport.zw;
    let w = sqrt(min(len, last) / g.fog_volume.w);
    var sum = textureSampleLevel(volume, volume_samp, vec3<f32>(uv, w), 0.0).rgb;
    if (len > last) {
        let rest = fog_clear(eye + dir * last, distance) - fog_clear(p, distance);
        let x = eye + dir * (0.5 * (last + len));
        let count = u32(g.ambient.w);
        for (var l = 0u; l < count; l++) {
            let light = lights[l];
            if (light.color.w > 0.0 && light.position.w < 0.5) {
                sum += rest * scattered(light, x, dir);
            }
        }
    }
    return sum * g.fog_albedo.rgb * g.fog_volume.x;
}

// A camera's background over its viewport, at the far plane: it clears colour and
// (with depth writes) depth for that camera only.
@vertex
fn vs_fill(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 1.0, 1.0);
}

@fragment
fn fs_fill(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (pos.xy - g.viewport.xy) / g.viewport.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let far4 = g.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let far = far4.xyz / far4.w;
    var c = g.background;
    if (g.flags.x > 0.5) {
        // The sky along this pixel's view ray.
        let dir = normalize(far - g.camera.xyz);
        c = vec4<f32>(textureSampleLevel(env_map, env_samp, dir, 0.0).rgb * g.env.y, 1.0);
    }
    // Distance fog leaves the background alone (Unity's); height fog covers it out to the
    // far plane, so the horizon sinks into it (and shafts reach into the sky).
    if (g.fog.w > 0.5 && g.fog_height.w > 0.5) {
        let f = 1.0 - fog_clear(far, false);
        var rgb = c.rgb * (1.0 - f) + g.fog.rgb * (c.a * f);
        if (g.fog_volume.w > 0.0) {
            rgb += shafts(far, pos.xy, false) * c.a;
        }
        c = vec4<f32>(rgb, c.a);
    }
    if (g.fog_local.x > 0.5) {
        c = through_volumes(c, fog_eye(far), far);
    }
    return c;
}

// The editor's ground grid: each pixel's view ray meets y = 0, where 1 m and 10 m lines
// are drawn anti-aliased (by their screen-space width) and fade out with distance, the
// X axis (z = 0) red and the Z axis (x = 0) blue. It writes the plane's depth for the
// depth test only, so objects in front hide it. Derivatives come before any discard.
struct GridOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

// How much of a line every `spacing` metres covers this pixel (0 to 1).
fn grid_lines(p: vec2<f32>, spacing: f32, width_px: f32) -> f32 {
    let c = p / spacing;
    let w = max(fwidth(c), vec2<f32>(1e-5));
    let d = abs(fract(c - 0.5) - 0.5) / w;
    let line = 1.0 - min(min(d.x, d.y) / width_px, 1.0);
    // Lines closer than about two pixels apart turn to mush: fade them out.
    return line * (1.0 - smoothstep(0.25, 0.5, max(w.x, w.y)));
}

@fragment
fn fs_grid(@builtin(position) pos: vec4<f32>) -> GridOut {
    let uv = (pos.xy - g.viewport.xy) / g.viewport.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let near4 = g.inv_view_proj * vec4<f32>(ndc, 0.0, 1.0);
    let far4 = g.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let a = near4.xyz / near4.w;
    let ray = far4.xyz / far4.w - a;
    let safe_y = select(ray.y, 1e-6, abs(ray.y) < 1e-6);
    let t = -a.y / safe_y;
    let p = a + ray * clamp(t, 0.0, 1.0);

    let minor = grid_lines(p.xz, 1.0, 1.0);
    let major = grid_lines(p.xz, 10.0, 1.5);
    let fw = max(fwidth(p.xz), vec2<f32>(1e-5));
    let x_axis = 1.0 - min(abs(p.z) / fw.y / 1.5, 1.0);
    let z_axis = 1.0 - min(abs(p.x) / fw.x / 1.5, 1.0);

    // Fade with distance, farther the higher the camera is.
    let reach = max(abs(g.camera.y) * 12.0, 30.0);
    let fade = 1.0 - smoothstep(reach * 0.3, reach, length(p.xz - g.camera.xz));
    // Lines seen at a grazing angle fade too (they'd alias).
    let facing = smoothstep(0.0, 0.15, abs(normalize(ray).y));

    var c = vec4<f32>(0.5, 0.5, 0.5, max(minor * 0.25, major * 0.5));
    let axis = max(x_axis, z_axis);
    if (axis > 0.0) {
        let tint = select(vec3<f32>(0.2, 0.35, 1.0), vec3<f32>(1.0, 0.2, 0.2), x_axis >= z_axis);
        c = mix(c, vec4<f32>(tint, 0.9), axis);
    }
    let alpha = c.a * fade * facing;

    var out: GridOut;
    let clip = g.view_proj * vec4<f32>(p, 1.0);
    out.depth = clamp(clip.z / clip.w, 0.0, 1.0);
    out.color = vec4<f32>(c.rgb * alpha, alpha);
    if (t <= 0.0 || t >= 1.0 || alpha <= 0.002) {
        discard;
    }
    return out;
}

// Gizmo lines: each instance is a segment, drawn as a quad a constant width on screen
// (two triangles; the end and side come from the vertex index). A segment crossing the
// near plane is cut there first.
const LINE_WIDTH: f32 = 1.6;

struct LineOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    // -1 to 1 across the line, for its soft edge.
    @location(1) side: f32,
};

@vertex
fn vs_line(
    @builtin(vertex_index) vi: u32,
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    @location(2) color: vec4<f32>,
) -> LineOut {
    var ca = g.view_proj * vec4<f32>(a, 1.0);
    var cb = g.view_proj * vec4<f32>(b, 1.0);
    let near = 1e-4;
    if (ca.w < near && cb.w >= near) {
        ca = mix(ca, cb, (near - ca.w) / (cb.w - ca.w));
    } else if (cb.w < near && ca.w >= near) {
        cb = mix(cb, ca, (near - cb.w) / (ca.w - cb.w));
    }
    var out: LineOut;
    out.color = vec4<f32>(color.rgb * color.a, color.a);
    if (ca.w < near && cb.w < near) {
        // All behind the camera.
        out.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        out.side = 0.0;
        return out;
    }
    // Corners: (end, side) for the two triangles.
    let ends = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);
    let sides = array<f32, 6>(-1.0, 1.0, -1.0, 1.0, 1.0, -1.0);
    let end = ends[vi];
    let side = sides[vi];
    let half = g.viewport.zw * 0.5;
    let sa = ca.xy / ca.w * half;
    let sb = cb.xy / cb.w * half;
    var dir = sb - sa;
    if (dot(dir, dir) < 1e-8) {
        dir = vec2<f32>(1.0, 0.0);
    }
    let n = normalize(vec2<f32>(-dir.y, dir.x));
    // Half the width plus a pixel for the soft edge.
    let reach = LINE_WIDTH * 0.5 + 1.0;
    var c = select(ca, cb, end > 0.5);
    c = vec4<f32>(c.xy + n * side * reach / half * c.w, c.zw);
    out.pos = c;
    out.side = side;
    return out;
}

fn line_alpha(side: f32) -> f32 {
    let reach = LINE_WIDTH * 0.5 + 1.0;
    let px = abs(side) * reach;
    return clamp(LINE_WIDTH * 0.5 + 0.5 - px, 0.0, 1.0);
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    return in.color * line_alpha(in.side);
}

@fragment
fn fs_line_hidden(in: LineOut) -> @location(0) vec4<f32> {
    return in.color * line_alpha(in.side) * 0.25;
}

// Fills (`LineRenderer2D.fillColor`): the tessellated shape's triangles, one colour.
struct ShapeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn vs_shape(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> ShapeOut {
    var out: ShapeOut;
    out.pos = g.view_proj * vec4<f32>(position, 1.0);
    out.color = vec4<f32>(color.rgb * color.a, color.a);
    out.world = position;
    return out;
}

@fragment
fn fs_shape(in: ShapeOut) -> @location(0) vec4<f32> {
    return fogged(in.color, in.world, in.pos.xy);
}

// Strokes (`LineRenderer2D`): each instance is a segment `width` world units wide, drawn as a
// capsule. The quad lies in the world along the segment and across it (square to the
// camera's forward axis), a pixel wider all round. The fragment shader cuts the capsule out
// by signed distance, so edges are smooth without MSAA and a run of segments gets round
// joins and caps where the capsules overlap.
struct StrokeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Where this fragment is from the segment's start, along it (x) and across it (y), in
    // world units.
    @location(1) local: vec2<f32>,
    // x: the segment's length; y: the stroke's radius.
    @location(2) @interpolate(flat) shape: vec2<f32>,
    @location(3) world: vec3<f32>,
};

fn unproject(ndc: vec3<f32>) -> vec3<f32> {
    let p = g.inv_view_proj * vec4<f32>(ndc, 1.0);
    return p.xyz / p.w;
}

@vertex
fn vs_stroke(
    @builtin(vertex_index) vi: u32,
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    @location(2) width: f32,
    @location(3) color: vec4<f32>,
) -> StrokeOut {
    var out: StrokeOut;
    out.color = vec4<f32>(color.rgb * color.a, color.a);
    // The camera's forward axis, through the middle of the screen.
    let forward = normalize(unproject(vec3<f32>(0.0, 0.0, 1.0)) - unproject(vec3<f32>(0.0, 0.0, 0.0)));
    let seg = b - a;
    let len = length(seg);
    let along = select(vec3<f32>(1.0, 0.0, 0.0), seg / max(len, 1e-6), len > 1e-6);
    var across = cross(forward, along);
    if (dot(across, across) < 1e-10) {
        // The segment points along the view: any direction across it will do.
        across = cross(forward, vec3<f32>(0.0, 1.0, 0.0));
        if (dot(across, across) < 1e-10) {
            across = vec3<f32>(1.0, 0.0, 0.0);
        }
    }
    across = normalize(across);
    let r = width * 0.5;
    // Corners: (end, side) for the two triangles.
    let ends = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);
    let sides = array<f32, 6>(-1.0, 1.0, -1.0, 1.0, 1.0, -1.0);
    let end = ends[vi];
    let side = sides[vi];
    let base = select(a, b, end > 0.5);
    // One pixel at this end, in world units, so the soft edge always fits in the quad.
    let c = g.view_proj * vec4<f32>(base, 1.0);
    var px = r;
    if (c.w > 1e-4) {
        let ndc = c.xyz / c.w;
        px = length(unproject(ndc + vec3<f32>(0.0, 2.0 / g.viewport.w, 0.0)) - base);
    }
    let reach = r + px;
    let u = select(-reach, len + reach, end > 0.5);
    let v = side * reach;
    out.world = a + along * u + across * v;
    out.pos = g.view_proj * vec4<f32>(out.world, 1.0);
    out.local = vec2<f32>(u, v);
    out.shape = vec2<f32>(len, r);
    return out;
}

@fragment
fn fs_stroke(in: StrokeOut) -> @location(0) vec4<f32> {
    let len = in.shape.x;
    let q = vec2<f32>(in.local.x - clamp(in.local.x, 0.0, len), in.local.y);
    let d = length(q) - in.shape.y;
    let aa = max(fwidth(d), 1e-6);
    return fogged(in.color * clamp(0.5 - d / aa, 0.0, 1.0), in.world, in.pos.xy);
}

struct MeshOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) instance: u32,
    // xyz: along increasing u in the world; w: the bitangent's side.
    @location(4) tangent: vec4<f32>,
    @location(5) detail_uv: vec2<f32>,
};

// Tiling (scale xy, offset zw) in Unity's bottom-up UV space, for top-down mesh UVs.
fn tiled(uv: vec2<f32>, t: vec4<f32>) -> vec2<f32> {
    return vec2<f32>(uv.x * t.x + t.z, 1.0 - ((1.0 - uv.y) * t.y + t.w));
}

@vertex
fn vs_mesh(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @builtin(instance_index) ii: u32,
) -> MeshOut {
    let m = meshes[ii];
    let world = m.model * vec4<f32>(position, 1.0);
    var out: MeshOut;
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    out.normal = (m.normal * vec4<f32>(normal, 0.0)).xyz;
    out.tangent = vec4<f32>((m.model * vec4<f32>(tangent.xyz, 0.0)).xyz, tangent.w);
    // Mesh UVs here run top-down (v = 0 at the texture's top row); tiling works bottom-up.
    out.uv = tiled(uv, m.tiling);
    out.detail_uv = tiled(uv, m.detail);
    out.instance = ii;
    return out;
}

// A skinned vertex's matrix: its bones blended by weight.
fn skin_matrix(m: MeshInstance, joints: vec4<u32>, weights: vec4<f32>) -> mat4x4<f32> {
    let first = u32(m.maps.w) - 1u;
    return bones[first + joints.x] * weights.x
        + bones[first + joints.y] * weights.y
        + bones[first + joints.z] * weights.z
        + bones[first + joints.w] * weights.w;
}

@vertex
fn vs_mesh_skinned(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
    @builtin(instance_index) ii: u32,
) -> MeshOut {
    let m = meshes[ii];
    // Bones are in world space: the object's own matrix doesn't apply.
    let skin = skin_matrix(m, joints, weights);
    let world = skin * vec4<f32>(position, 1.0);
    var out: MeshOut;
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    out.normal = (skin * vec4<f32>(normal, 0.0)).xyz;
    out.tangent = vec4<f32>((skin * vec4<f32>(tangent.xyz, 0.0)).xyz, tangent.w);
    out.uv = tiled(uv, m.tiling);
    out.detail_uv = tiled(uv, m.detail);
    out.instance = ii;
    return out;
}

@vertex
fn vs_shadow_skinned(
    @location(0) position: vec3<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
    @builtin(instance_index) ii: u32,
) -> @builtin(position) vec4<f32> {
    let skin = skin_matrix(meshes[ii], joints, weights);
    return g.view_proj * skin * vec4<f32>(position, 1.0);
}

struct CutoutOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance: u32,
};

// Cut-out casters: depth from the light, with the texture's holes.
@vertex
fn vs_shadow_cutout(
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) ii: u32,
) -> CutoutOut {
    let m = meshes[ii];
    var out: CutoutOut;
    out.pos = g.view_proj * m.model * vec4<f32>(position, 1.0);
    out.uv = tiled(uv, m.tiling);
    out.instance = ii;
    return out;
}

@fragment
fn fs_shadow_cutout(in: CutoutOut) {
    let m = meshes[in.instance];
    if (textureSample(tex, samp, in.uv).a * m.color.a < m.maps.z) {
        discard;
    }
}

const PI: f32 = 3.14159265;

// Behind the view's clip plane (mirror views).
fn clipped(p: vec3<f32>) -> bool {
    return any(g.clip != vec4<f32>(0.0)) && dot(g.clip.xyz, p) + g.clip.w < 0.0;
}

// GGX / Trowbridge-Reitz normal distribution.
fn ggx(ndh: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = ndh * ndh * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

// Smith-Schlick geometry (shadowing and masking), Unreal's k for direct light.
fn smith(ndv: f32, ndl: f32, roughness: f32) -> f32 {
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    return (ndv / (ndv * (1.0 - k) + k)) * (ndl / (ndl * (1.0 - k) + k));
}

fn fresnel(f0: vec3<f32>, vdh: f32) -> vec3<f32> {
    let x = 1.0 - vdh;
    let x5 = x * x * x * x * x;
    return f0 + (vec3<f32>(1.0) - f0) * x5;
}

// The split-sum environment BRDF, fitted analytically (Karis, "Physically Based Shading
// on Mobile"): how much of the environment a surface reflects.
fn env_brdf_approx(f0: vec3<f32>, roughness: f32, ndv: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * ndv)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + ab.y;
}

// The light arriving at `p` from direction `dir`, blurred for `roughness`: the mesh's
// reflection probe, else the sky's map (lower, blurrier mips for rougher surfaces), else
// the ambient colour.
fn environment(dir: vec3<f32>, roughness: f32, m: MeshInstance, p: vec3<f32>) -> vec3<f32> {
    if (m.probe.w > 0.5) {
        var d = dir;
        if (m.probe_box.w > 0.5) {
            // Box projection: where the ray leaves the probe's box, seen from its centre,
            // so nearby walls line up.
            let lo = m.probe.xyz - m.probe_box.xyz;
            let hi = m.probe.xyz + m.probe_box.xyz;
            let a = (hi - p) / dir;
            let b = (lo - p) / dir;
            let far = max(a, b);
            let t = min(far.x, min(far.y, far.z));
            d = p + dir * t - m.probe.xyz;
        }
        let lod = sqrt(roughness) * (m.probe.w - 1.0);
        return textureSampleLevel(env_map, env_samp, d, lod).rgb * g.env.y;
    }
    if (g.env.x < 0.5) {
        return g.ambient.rgb;
    }
    // Rough surfaces see mostly the blurriest mips: sqrt spreads smooth ones out.
    let lod = sqrt(roughness) * g.env.z;
    return textureSampleLevel(env_map, env_samp, dir, lod).rgb * g.env.y;
}

// Light from all around reaching a surface facing `n`: the sky's blurriest mip, or the
// ambient colour.
fn ambient_light(n: vec3<f32>) -> vec3<f32> {
    if (g.env.w < 0.5) {
        return g.ambient.rgb;
    }
    return textureSampleLevel(env_map, env_samp, n, g.env.z).rgb * g.env.y;
}

// Shadow casters: just depth, from the light.
@vertex
fn vs_shadow(@location(0) position: vec3<f32>, @builtin(instance_index) ii: u32) -> @builtin(position) vec4<f32> {
    // The layer's matrix is the view in the shadow pass's globals.
    return g.view_proj * meshes[ii].model * vec4<f32>(position, 1.0);
}

// The share of a shadow map's comparisons that pass at `uv` (1: lit): a (2r + 1)² box of
// them, each bilinear-filtered by the sampler, or one when `r` is 0.
fn pcf(t: texture_depth_2d_array, layer: i32, uv: vec2<f32>, depth: f32, r: i32) -> f32 {
    if (r <= 0) {
        return textureSampleCompareLevel(t, shadow_samp, uv, layer, depth);
    }
    let ts = 1.0 / f32(textureDimensions(t).x);
    var lit = 0.0;
    for (var y = -r; y <= r; y++) {
        for (var x = -r; x <= r; x++) {
            lit += textureSampleCompareLevel(t, shadow_samp, uv + vec2<f32>(f32(x), f32(y)) * ts, layer, depth);
        }
    }
    let side = f32(2 * r + 1);
    return lit / (side * side);
}

// A point through a shadow matrix: map uv (xy) and depth (z); w < 0 when off the map.
fn shadow_coords(m: mat4x4<f32>, p: vec3<f32>) -> vec4<f32> {
    let c = m * vec4<f32>(p, 1.0);
    let ndc = c.xyz / c.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let inside = c.w > 0.0 && all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) && ndc.z <= 1.0;
    return vec4<f32>(uv, ndc.z, select(-1.0, 1.0, inside));
}

// How much of the sun reaches `world` (normal `n`): the first cascade holding it, filtered
// over radius `r`.
fn sun_shadow(light: Light, world: vec3<f32>, n: vec3<f32>, r: i32) -> f32 {
    let count = min(i32(g.shadow.x), MAX_CASCADES);
    for (var c = 0; c < count; c++) {
        // Pushed off the surface by a texel or two, against acne.
        let s = shadow_coords(shadow_matrices[c], world + n * g.cascade_texels[c] * 1.5);
        if (s.w > 0.0) {
            return mix(1.0, pcf(sun_shadows, c, s.xy, s.z, r), light.shadow.y);
        }
    }
    return 1.0;
}

// How much of a spot or point light reaches `world`: its map, or a point light's cube
// face towards `world`, filtered over radius `r`.
fn local_shadow(light: Light, world: vec3<f32>, n: vec3<f32>, r: i32) -> f32 {
    var index = i32(light.shadow.x);
    let to = world - light.position.xyz;
    if (light.position.w < 1.5) {
        // Faces in order +x, -x, +y, -y, +z, -z.
        let a = abs(to);
        if (a.x >= a.y && a.x >= a.z) {
            index += select(1, 0, to.x > 0.0);
        } else if (a.y >= a.z) {
            index += select(3, 2, to.y > 0.0);
        } else {
            index += select(5, 4, to.z > 0.0);
        }
    }
    // Texels grow with distance from the light; so does the push off the surface.
    let offset = n * length(to) * light.shadow.w * 1.5;
    let s = shadow_coords(shadow_matrices[index], world + offset);
    if (s.w < 0.0) {
        return 1.0;
    }
    return mix(1.0, pcf(local_shadows, index - MAX_CASCADES, s.xy, s.z, r), light.shadow.y);
}

@fragment
fn fs_mesh(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let m = meshes[in.instance];
    // Every map is sampled up front: sampling needs uniform control flow.
    let sampled = textureSample(tex, samp, in.uv);
    let normal_texel = textureSample(normal_map, samp, in.uv);
    let metallic_texel = textureSample(metallic_map, samp, in.uv);
    let occlusion_texel = textureSample(occlusion_map, samp, in.uv);
    let emission_texel = textureSample(emission_map, samp, in.uv);
    let detail_texel = textureSample(detail_map, samp, in.detail_uv);
    if (clipped(in.world)) {
        discard;
    }
    let maps = m.layer.y;
    let mode = m.layer.z;
    let mirror = m.params.w > 0.5;
    let texel = sampled * m.color;
    var base = texel.rgb;
    if ((maps & MAP_DETAIL) != 0u) {
        // Unity's detail albedo: x2 in gamma space (about 4.59 in linear light).
        base *= detail_texel.rgb * 4.594794;
    }
    // Opaque meshes ignore the alpha; the other modes use texture x colour.
    let alpha = select(texel.a, 1.0, mode == 0u);
    if (mode == 1u && alpha < m.maps.z) {
        discard;
    }
    var emission = m.emission.rgb;
    if ((maps & MAP_EMISSION) != 0u) {
        emission *= emission_texel.rgb;
    }
    if (m.params.y > 0.5) {
        // Glowing meshes add light (no coverage, so premultiplied blending adds).
        var light = (base + emission) * m.color.a;
        if (m.params.y > 1.5) {
            // A beam: soft where its surface turns away from the eye, bright near the
            // light and fading out towards the open end.
            let n = normalize(in.normal);
            let v = normalize(g.camera.xyz - in.world);
            // Denser towards the axis, like dust lit by the beam: a steep falloff reads
            // as a soft edge once on screen.
            let edge = abs(dot(n, v));
            let e2 = edge * edge;
            let along = clamp(in.uv.y, 0.0, 1.0);
            light *= e2 * e2 * (1.0 - along) * (1.0 - along) * smoothstep(0.0, 0.08, along);
        }
        return fogged(vec4<f32>(light, 0.0), in.world, in.pos.xy);
    }
    let out_alpha = select(alpha, 1.0, mode == 1u);
    if (m.params.x > 0.5) {
        let unlit = base + emission;
        return fogged(vec4<f32>(unlit * out_alpha, out_alpha), in.world, in.pos.xy);
    }
    var n = normalize(in.normal);
    var tangent = in.tangent.xyz;
    if (!front) {
        n = -n;
    }
    if ((maps & MAP_NORMAL) != 0u && dot(tangent, tangent) > 1e-12) {
        // Tangent space: u along the tangent, up the image along the bitangent.
        let t = normalize(tangent - n * dot(n, tangent));
        let b = cross(n, t) * select(in.tangent.w, -in.tangent.w, !front);
        var tn = normal_texel.xyz * 2.0 - 1.0;
        tn = vec3<f32>(tn.xy * m.maps.x, max(tn.z, 1e-3));
        n = normalize(t * tn.x + b * tn.y + n * tn.z);
    }
    let v = normalize(g.camera.xyz - in.world);
    // Smoothness is 1 - roughness (Unity's name).
    var smoothness = m.emission.w;
    var metallic = m.params.z;
    var occlusion = 1.0;
    if ((maps & MAP_OCCLUSION) != 0u) {
        let o = select(occlusion_texel.g, occlusion_texel.r, (maps & MAP_GLTF) != 0u);
        occlusion = mix(1.0, o, m.maps.y);
    }
    var f0: vec3<f32>;
    var albedo: vec3<f32>;
    if (m.specular.w > 0.5) {
        // The specular workflow: the reflectance is given; diffuse keeps the rest.
        var spec = m.specular.rgb;
        if ((maps & MAP_METALLIC) != 0u) {
            spec = metallic_texel.rgb;
            smoothness *= metallic_texel.a;
        }
        f0 = spec;
        albedo = base * (1.0 - max(spec.r, max(spec.g, spec.b)));
    } else {
        if ((maps & MAP_METALLIC) != 0u) {
            if ((maps & MAP_GLTF) != 0u) {
                // glTF: roughness in green, metallic in blue, scaling the factors.
                metallic *= metallic_texel.b;
                smoothness = 1.0 - (1.0 - smoothness) * metallic_texel.g;
            } else {
                // Unity: metallic in red, smoothness in alpha scaled by the slider.
                metallic = metallic_texel.r;
                smoothness *= metallic_texel.a;
            }
        }
        metallic = clamp(metallic, 0.0, 1.0);
        // Metallic-roughness PBR (Cook-Torrance: GGX, Smith, Schlick), Unity's Standard
        // shader's model.
        f0 = mix(vec3<f32>(0.04), base, metallic);
        albedo = base * (1.0 - metallic);
    }
    let roughness = clamp(1.0 - smoothness, 0.045, 1.0);
    let ndv = max(dot(n, v), 1e-4);
    var diffuse = vec3<f32>(0.0);
    var specular = vec3<f32>(0.0);
    let count = u32(g.ambient.w);
    for (var i = 0u; i < count; i++) {
        let light = lights[i];
        // Lights skip layers outside their culling mask.
        if ((light.mask.x & m.layer.x) == 0u) {
            continue;
        }
        var l: vec3<f32>;
        var strength = 1.0;
        if (light.position.w < 0.5) {
            l = -normalize(light.direction.xyz);
        } else {
            let to_light = light.position.xyz - in.world;
            let d = length(to_light);
            l = to_light / max(d, 1e-4);
            // Smooth falloff that reaches zero at the range.
            let x = clamp(d / max(light.direction.w, 1e-4), 0.0, 1.0);
            let fade = 1.0 - x * x * x * x;
            strength = fade * fade / (1.0 + d * d);
            if (light.position.w > 1.5) {
                let c = dot(-l, normalize(light.direction.xyz));
                strength *= smoothstep(light.spot.y, light.spot.x, c);
            }
        }
        if (light.shadow.x >= 0.0) {
            let r = i32(light.shadow.z + 0.5);
            if (light.position.w < 0.5) {
                strength *= sun_shadow(light, in.world, n, r);
            } else {
                strength *= local_shadow(light, in.world, n, r);
            }
        }
        let ndl = max(dot(n, l), 0.0);
        if (ndl <= 0.0) {
            continue;
        }
        let h = normalize(l + v);
        let f = fresnel(f0, max(dot(v, h), 0.0));
        let spec = ggx(max(dot(n, h), 0.0), roughness) * smith(ndv, ndl, roughness) * f / (4.0 * ndv * ndl + 1e-4);
        // Diffuse isn't divided by pi (lights' intensities are as Unity's); specular is
        // scaled up to match.
        let kd = (vec3<f32>(1.0) - f) * albedo;
        diffuse += kd * light.color.rgb * ndl * strength;
        specular += spec * PI * light.color.rgb * ndl * strength;
    }
    // Light from all around: sky light (or the ambient colour) in the diffuse, and the
    // reflection (a mirror's render, a probe, the sky, or the ambient colour) in the
    // specular, with the split-sum approximation. Occlusion darkens only these.
    let env_brdf = env_brdf_approx(f0, roughness, ndv);
    var reflected: vec3<f32>;
    if (mirror) {
        // The reflection was rendered flipped left to right within this viewport.
        let px = vec2<f32>(g.viewport.x * 2.0 + g.viewport.z - in.pos.x, in.pos.y);
        reflected = textureSampleLevel(reflection_tex, samp, px / g.flags.yz, 0.0).rgb;
    } else {
        reflected = environment(reflect(-v, n), roughness, m, in.world);
    }
    diffuse += albedo * ambient_light(n) * occlusion;
    specular += env_brdf * reflected * occlusion;
    if (mode == 3u) {
        // Transparent: the diffuse fades with the alpha, highlights and reflections
        // don't, and what's reflected covers what's behind (Unity's Standard).
        let reflectivity = max(f0.r, max(f0.g, f0.b));
        let a = 1.0 - (1.0 - alpha) * (1.0 - reflectivity);
        return fogged(vec4<f32>(diffuse * alpha + specular + emission, a), in.world, in.pos.xy);
    }
    let lit = diffuse + specular + emission;
    return fogged(vec4<f32>(lit * out_alpha, out_alpha), in.world, in.pos.xy);
}
