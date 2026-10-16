// Iron UI quads: rounded boxes with borders and fills (solid or gradient),
// and analytic outer / inset box shadows. One instance per primitive.
//
// Geometry is in logical pixels; `globals.scale` converts to physical pixels.
// Colours arrive linear and premultiplied, except gradient stops (sRGB, straight
// alpha), which are interpolated in premultiplied sRGB like CSS.

struct Globals {
    viewport: vec2<f32>, // physical pixels
    scale: f32,
    _pad: f32,
};

struct Gradient {
    // x: kind (0 linear, 1 radial), y: stop count,
    // zw: linear direction (unit vector) or radial radii.
    info: vec4<f32>,
    // x: linear gradient line length.
    info2: vec4<f32>,
    colors: array<vec4<f32>, 8>,
    stops: array<vec4<f32>, 2>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> gradients: array<Gradient>;

const KIND_QUAD: u32 = 0u;
const KIND_OUTER_SHADOW: u32 = 1u;
const KIND_INSET_SHADOW: u32 = 2u;

struct Instance {
    @location(0) rect: vec4<f32>,   // the shape: x, y, w, h
    @location(1) radii: vec4<f32>,  // top-left, top-right, bottom-right, bottom-left
    @location(2) p0: vec4<f32>,     // quad: border widths (t, r, b, l); shadows: second shape rect
    @location(3) p1: vec4<f32>,     // shadows: second shape radii
    @location(4) color0: vec4<f32>, // fill, or shadow colour
    @location(5) color1: vec4<f32>, // border colour
    @location(6) params: vec4<f32>, // x: kind, y: sigma (logical px), z: gradient index or -1
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) @interpolate(flat) rect: vec4<f32>,
    @location(1) @interpolate(flat) radii: vec4<f32>,
    @location(2) @interpolate(flat) p0: vec4<f32>,
    @location(3) @interpolate(flat) p1: vec4<f32>,
    @location(4) @interpolate(flat) color0: vec4<f32>,
    @location(5) @interpolate(flat) color1: vec4<f32>,
    @location(6) @interpolate(flat) params: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let kind = u32(inst.params.x);
    // Grow the drawn area by one physical pixel for anti-aliasing, and by the
    // blur extent for outer shadows.
    var grow = 1.0 / globals.scale;
    if kind == KIND_OUTER_SHADOW {
        grow += 3.0 * inst.params.y;
    }
    let area = vec4<f32>(inst.rect.xy - vec2(grow), inst.rect.zw + vec2(2.0 * grow));

    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let logical = area.xy + corners[vi] * area.zw;
    let physical = logical * globals.scale;
    let ndc = vec2(physical.x / globals.viewport.x * 2.0 - 1.0, 1.0 - physical.y / globals.viewport.y * 2.0);

    var out: VsOut;
    out.pos = vec4(ndc, 0.0, 1.0);
    out.rect = inst.rect;
    out.radii = inst.radii;
    out.p0 = inst.p0;
    out.p1 = inst.p1;
    out.color0 = inst.color0;
    out.color1 = inst.color1;
    out.params = inst.params;
    return out;
}

// Picks the corner radius for the quadrant `q` (relative to the centre, y down).
fn corner_radius(q: vec2<f32>, radii: vec4<f32>) -> f32 {
    let top = select(radii.x, radii.y, q.x > 0.0);
    let bottom = select(radii.w, radii.z, q.x > 0.0);
    return select(top, bottom, q.y > 0.0);
}

// Signed distance (logical px) to a rounded rectangle with per-corner radii.
fn sd_rounded_box(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let half = rect.zw * 0.5;
    let q0 = p - (rect.xy + half);
    let r = min(corner_radius(q0, radii), min(half.x, half.y));
    let q = abs(q0) - half + vec2(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
}

// Anti-aliased coverage from a signed distance: one physical pixel of feathering.
fn coverage(d: f32) -> f32 {
    return clamp(0.5 - d * globals.scale, 0.0, 1.0);
}

fn shape_coverage(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    if rect.z <= 0.0 || rect.w <= 0.0 {
        return 0.0;
    }
    return coverage(sd_rounded_box(p, rect, radii));
}

// --- Blurred rounded rectangles (Evan Wallace's analytic box shadow) -------

fn erf2(x: vec2<f32>) -> vec2<f32> {
    let s = sign(x);
    let a = abs(x);
    var r = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    r = r * r;
    return s - s / (r * r);
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    let pi = 3.141592653589793;
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * pi) * sigma);
}

fn shadow_x(x: f32, y: f32, sigma: f32, corner: f32, half: vec2<f32>) -> f32 {
    let delta = min(half.y - corner - abs(y), 0.0);
    let curved = half.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    let integral = 0.5 + 0.5 * erf2((x + vec2(-curved, curved)) * (sqrt(0.5) / sigma));
    return integral.y - integral.x;
}

// Coverage of a rounded rectangle convolved with a Gaussian of `sigma`.
fn blurred_box(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, sigma: f32) -> f32 {
    if rect.z <= 0.0 || rect.w <= 0.0 {
        return 0.0;
    }
    if sigma < 0.01 {
        return shape_coverage(p, rect, radii);
    }
    let half = rect.zw * 0.5;
    let point = p - (rect.xy + half);
    let corner = min(corner_radius(point, radii), min(half.x, half.y));
    let low = point.y - half.y;
    let high = point.y + half.y;
    let start = clamp(-3.0 * sigma, low, high);
    let end = clamp(3.0 * sigma, low, high);
    let step = (end - start) / 4.0;
    var y = start + step * 0.5;
    var value = 0.0;
    for (var i = 0; i < 4; i++) {
        value += shadow_x(point.x, point.y - y, sigma, corner, half) * gaussian(y, sigma) * step;
        y += step;
    }
    return clamp(value, 0.0, 1.0);
}

// --- Gradients --------------------------------------------------------------

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3(2.4));
    return select(high, low, c <= vec3(0.04045));
}

fn stop_position(g: Gradient, i: u32) -> f32 {
    return g.stops[i / 4u][i % 4u];
}

fn premultiply(c: vec4<f32>) -> vec4<f32> {
    return vec4(c.rgb * c.a, c.a);
}

// Gradient colour at `p` for a box `rect`, as linear premultiplied RGBA.
fn gradient_color(index: u32, p: vec2<f32>, rect: vec4<f32>) -> vec4<f32> {
    let g = gradients[index];
    let center = rect.xy + rect.zw * 0.5;
    var t: f32;
    if g.info.x < 0.5 {
        t = dot(p - center, g.info.zw) / g.info2.x + 0.5;
    } else {
        t = length((p - center) / g.info.zw);
    }
    let count = u32(g.info.y);
    var c = g.colors[0];
    if t >= stop_position(g, count - 1u) {
        c = g.colors[count - 1u];
    } else if t > stop_position(g, 0u) {
        for (var i = 0u; i + 1u < count; i++) {
            let a = stop_position(g, i);
            let b = stop_position(g, i + 1u);
            if t >= a && t < b {
                let f = select(0.0, (t - a) / (b - a), b > a);
                // Interpolate in premultiplied sRGB, like CSS.
                let m = mix(premultiply(g.colors[i]), premultiply(g.colors[i + 1u]), f);
                c = vec4(select(vec3(0.0), m.rgb / m.a, m.a > 0.0), m.a);
                break;
            }
        }
    }
    return vec4(srgb_to_linear(c.rgb) * c.a, c.a);
}

// --- Fragment ---------------------------------------------------------------

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.pos.xy / globals.scale;
    let kind = u32(in.params.x);
    let sigma = in.params.y;

    if kind == KIND_OUTER_SHADOW {
        // CSS clips outer shadows to outside the box that casts them.
        let shadow = blurred_box(p, in.rect, in.radii, sigma);
        return in.color0 * shadow * (1.0 - shape_coverage(p, in.p0, in.p1));
    }
    if kind == KIND_INSET_SHADOW {
        // Drawn inside the padding box, everywhere except the (blurred) hole.
        let inside = shape_coverage(p, in.rect, in.radii);
        return in.color0 * inside * (1.0 - blurred_box(p, in.p0, in.p1, sigma));
    }

    // A box: fill (solid colour with an optional gradient over it), then border.
    let outer = shape_coverage(p, in.rect, in.radii);
    if outer <= 0.0 {
        discard;
    }
    let b = in.p0; // border widths: top, right, bottom, left
    let inner_rect = vec4(
        in.rect.x + b.w,
        in.rect.y + b.x,
        in.rect.z - b.w - b.y,
        in.rect.w - b.x - b.z,
    );
    let inner_radii = max(
        in.radii - vec4(max(b.x, b.w), max(b.x, b.y), max(b.z, b.y), max(b.z, b.w)),
        vec4(0.0),
    );
    let inner = min(shape_coverage(p, inner_rect, inner_radii), outer);

    var fill = in.color0;
    let gradient = in.params.z;
    if gradient >= 0.0 {
        let g = gradient_color(u32(gradient), p, in.rect);
        fill = g + fill * (1.0 - g.a);
    }
    // CSS backgrounds extend under the border (background-clip: border-box).
    let base = fill * outer;
    let border = in.color1 * (outer - inner);
    return border + base * (1.0 - border.a);
}

// --- Background images ---------------------------------------------------------
// Drawn by a separate pipeline (one texture per draw). The instance's `rect` and
// `radii` are the box (the clip shape), `p0` is one tile of the image in logical
// px, and `params.yz` say whether it repeats along x and y.

@group(1) @binding(0) var image_tex: texture_2d<f32>;
@group(1) @binding(1) var image_samp: sampler;

@fragment
fn fs_image(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.pos.xy / globals.scale;
    let clip = shape_coverage(p, in.rect, in.radii);
    if clip <= 0.0 {
        discard;
    }
    let tile = in.p0;
    let uv = (p - tile.xy) / tile.zw;
    let repeat_x = in.params.y > 0.5;
    let repeat_y = in.params.z > 0.5;
    if (!repeat_x && (uv.x < 0.0 || uv.x > 1.0)) || (!repeat_y && (uv.y < 0.0 || uv.y > 1.0)) {
        discard;
    }
    // The sampler repeats. On axes that don't repeat, keep bilinear filtering
    // from wrapping around to the opposite edge. sRGB textures sample as linear.
    let half_texel = 0.5 / vec2<f32>(textureDimensions(image_tex));
    let clamped = clamp(uv, half_texel, vec2(1.0) - half_texel);
    let sample_uv = vec2(select(clamped.x, uv.x, repeat_x), select(clamped.y, uv.y, repeat_y));
    let c = textureSampleLevel(image_tex, image_samp, sample_uv, 0.0);
    return vec4(c.rgb * c.a, c.a) * clip;
}
