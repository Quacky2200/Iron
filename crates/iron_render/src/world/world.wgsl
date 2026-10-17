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
    // The shadow map's view-projection.
    shadow_view_proj: mat4x4<f32>,
    // x: the shadowed light's index (-1: none); y: strength; z: a shadow texel in
    // metres; w: 1 for soft (filtered) shadows.
    shadow: vec4<f32>,
    // Screen to world, for the skybox.
    inv_view_proj: mat4x4<f32>,
    // The viewport in target pixels: x, y, width, height.
    viewport: vec4<f32>,
    // x: 1 with an environment map; y: intensity; z: its last mip; w: 1 for sky ambient.
    env: vec4<f32>,
    // x: 1 to draw the sky behind everything.
    flags: vec4<f32>,
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
    // x: 1 when unlit; y: 1 additive, 2 a light beam; z: metallic 0..1.
    params: vec4<f32>,
    // Texture tiling: scale (xy), offset (zw), in bottom-up UV space like Unity's.
    tiling: vec4<f32>,
};

struct Light {
    // xyz: position; w: 0 directional, 1 point, 2 spot.
    position: vec4<f32>,
    // xyz: the direction it shines; w: range (point and spot).
    direction: vec4<f32>,
    // rgb: colour × intensity; w unused.
    color: vec4<f32>,
    // x, y: cosines of the spot's inner and outer half-angles.
    spot: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var<storage, read> quads: array<Quad>;
@group(0) @binding(2) var<storage, read> meshes: array<MeshInstance>;
@group(0) @binding(3) var<storage, read> lights: array<Light>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_samp: sampler_comparison;
@group(2) @binding(2) var env_map: texture_cube<f32>;
@group(2) @binding(3) var env_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) straight: f32,
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
    out.pos = g.view_proj * q.model * vec4<f32>(c.x, c.y, 0.0, 1.0);
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
    if (in.straight > 0.5) {
        t = vec4<f32>(t.rgb * t.a, t.a);
    }
    let c = t * in.color;
    // Fully transparent texels don't write depth (UI panels have see-through parts).
    // Additive quads have no alpha, only light.
    if (c.a <= 0.002 && max(c.r, max(c.g, c.b)) <= 0.002) {
        discard;
    }
    return c;
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
    if (g.flags.x > 0.5) {
        // The sky along this pixel's view ray.
        let uv = (pos.xy - g.viewport.xy) / g.viewport.zw;
        let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
        let far = g.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
        let dir = normalize(far.xyz / far.w - g.camera.xyz);
        return vec4<f32>(textureSampleLevel(env_map, env_samp, dir, 0.0).rgb * g.env.y, 1.0);
    }
    return g.background;
}

struct MeshOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) instance: u32,
};

@vertex
fn vs_mesh(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) ii: u32,
) -> MeshOut {
    let m = meshes[ii];
    let world = m.model * vec4<f32>(position, 1.0);
    var out: MeshOut;
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    out.normal = (m.normal * vec4<f32>(normal, 0.0)).xyz;
    // Mesh UVs here run top-down (v = 0 at the texture's top row); tiling works bottom-up.
    let t = m.tiling;
    out.uv = vec2<f32>(uv.x * t.x + t.z, 1.0 - ((1.0 - uv.y) * t.y + t.w));
    out.instance = ii;
    return out;
}

const PI: f32 = 3.14159265;

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

// The light arriving from direction `dir`, blurred for `roughness`: the sky's map (lower,
// blurrier mips for rougher surfaces), or the ambient colour without one.
fn environment(dir: vec3<f32>, roughness: f32) -> vec3<f32> {
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
    return g.shadow_view_proj * meshes[ii].model * vec4<f32>(position, 1.0);
}

// How much of the shadowed light reaches `world` (1: all), whose normal is `n`.
fn shadow_at(world: vec3<f32>, n: vec3<f32>) -> f32 {
    // Pushed off the surface by a texel or two, against acne.
    let p = g.shadow_view_proj * vec4<f32>(world + n * g.shadow.z * 1.5, 1.0);
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0) {
        return 1.0;
    }
    var lit = 0.0;
    if (g.shadow.w > 0.5) {
        let ts = 1.0 / f32(textureDimensions(shadow_map).x);
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                lit += textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(f32(x), f32(y)) * ts, ndc.z);
            }
        }
        lit /= 9.0;
    } else {
        lit = textureSampleCompareLevel(shadow_map, shadow_samp, uv, ndc.z);
    }
    return mix(1.0, lit, g.shadow.y);
}

@fragment
fn fs_mesh(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let m = meshes[in.instance];
    let texel = textureSample(tex, samp, in.uv) * m.color;
    let base = texel.rgb;
    // Opaque meshes (colour alpha 1) ignore the texture's alpha; see-through ones use it.
    let alpha = select(1.0, texel.a, m.color.a < 1.0);
    if (m.params.y > 0.5) {
        // Glowing meshes add light (no coverage, so premultiplied blending adds).
        var light = (base + m.emission.rgb) * m.color.a;
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
        return vec4<f32>(light, 0.0);
    }
    if (m.params.x > 0.5) {
        let unlit = base + m.emission.rgb;
        return vec4<f32>(unlit * alpha, alpha);
    }
    var n = normalize(in.normal);
    if (!front) {
        n = -n;
    }
    let v = normalize(g.camera.xyz - in.world);
    // Metallic-roughness PBR (Cook-Torrance: GGX, Smith, Schlick), Unity's Standard
    // shader's model: smoothness is 1 - roughness.
    let roughness = clamp(1.0 - m.emission.w, 0.045, 1.0);
    let metallic = clamp(m.params.z, 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), base, metallic);
    let albedo = base * (1.0 - metallic);
    let ndv = max(dot(n, v), 1e-4);
    var color = vec3<f32>(0.0);
    let count = u32(g.ambient.w);
    for (var i = 0u; i < count; i++) {
        let light = lights[i];
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
        if (i32(i) == i32(g.shadow.x)) {
            strength *= shadow_at(in.world, n);
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
        color += (kd + spec * PI) * light.color.rgb * ndl * strength;
    }
    // Light from all around: the environment (the ambient colour, for now) seen in the
    // diffuse and reflected by the specular, with the split-sum approximation.
    let env_brdf = env_brdf_approx(f0, roughness, ndv);
    color += albedo * ambient_light(n) + env_brdf * environment(reflect(-v, n), roughness);
    let lit = color + m.emission.rgb;
    return vec4<f32>(lit * alpha, alpha);
}
