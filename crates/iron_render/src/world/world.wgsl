// The 3D world pass: instanced quads (planes) seen through a perspective or
// orthographic camera. Each quad is a unit square in its own XY plane (-0.5..0.5,
// facing +z), placed by its model matrix. Colours are linear and premultiplied.

struct Globals {
    view_proj: mat4x4<f32>,
    // The camera's background (linear, premultiplied), for `fs_fill`.
    background: vec4<f32>,
};

struct Quad {
    model: mat4x4<f32>,
    color: vec4<f32>,
    // Texture rectangle: origin (xy) and size (zw) in UV space.
    uv: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var<storage, read> quads: array<Quad>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
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
    return out;
}

@fragment
fn fs_quad(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv) * in.color;
    // Fully transparent texels don't write depth (UI panels have see-through parts).
    if (c.a <= 0.002) {
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
