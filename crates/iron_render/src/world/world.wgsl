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
    // x: 1 when unlit.
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
fn fs_fill() -> @location(0) vec4<f32> {
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

@fragment
fn fs_mesh(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let m = meshes[in.instance];
    let texel = textureSample(tex, samp, in.uv) * m.color;
    let base = texel.rgb;
    // Opaque meshes (colour alpha 1) ignore the texture's alpha; see-through ones use it.
    let alpha = select(1.0, texel.a, m.color.a < 1.0);
    if (m.params.x > 0.5) {
        let unlit = base + m.emission.rgb;
        return vec4<f32>(unlit * alpha, alpha);
    }
    var n = normalize(in.normal);
    if (!front) {
        n = -n;
    }
    let v = normalize(g.camera.xyz - in.world);
    let smooth_ = clamp(m.emission.w, 0.0, 1.0);
    let shininess = mix(4.0, 256.0, smooth_ * smooth_);
    var diffuse = g.ambient.rgb;
    var specular = vec3<f32>(0.0);
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
        let ndl = max(dot(n, l), 0.0);
        diffuse += light.color.rgb * ndl * strength;
        let h = normalize(l + v);
        // Energy-ish normalised Blinn-Phong, scaled down for rough surfaces.
        let spec = pow(max(dot(n, h), 0.0), shininess) * (shininess + 8.0) / 25.0 * smooth_;
        specular += light.color.rgb * spec * ndl * strength;
    }
    let lit = base * diffuse + specular * 0.04 + m.emission.rgb;
    return vec4<f32>(lit * alpha, alpha);
}
