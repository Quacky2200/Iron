#!/usr/bin/env python3
"""Generates src/generated.rs: the plain data of iron_render_api. A *component* is a per-thread
registry of data, a handle class (attach / findOn / gameObject / one property per field) and a
scene-file kind. An *asset* (Material, Sprite) is the same without an object: `create()`, `copy()`
and an asset kind (inline `$type` in scene files). Run from the repository root:

    python crates/iron_render_api/gen/gen_components.py

A field is (ts name, rust name, type, default, doc). Types: f64, i32, bool, string, enum:a|b|c,
mask (a layer mask, a number), color, v2, v3, texture, mesh, material, sprite, object (a
GameObject). Optional hand-written members of a spec: `extra_fields` / `extra_defaults` (struct
fields and their values), `api` (members of the handle class), `items` (module items),
`extra_save` (statements; `d` is the data, `m` the property map), `extra_load` (match arms on `k`
and `v`), `skip_save` (fields not saved), `save_if_changed` (fields saved only when they differ
from the default) and `set_hook` (code run after a field is set).
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))

# store type, api get type, getter expr (F = the field, as `d.F`), api set type, set statement,
# to-json expr, from-json expr (`v: &Value` -> Option<store type>), fallback for a dead handle
TYPES = {
    "f64": ("f64", "f64", "d.F", "f64", "d.F = v", "json!(d.F)", "v.as_f64()", "0.0"),
    "i32": ("i32", "f64", "f64::from(d.F)", "f64", "d.F = v as i32", "json!(d.F)", "v.as_i64().map(|n| n as i32)", "0.0"),
    "bool": ("bool", "bool", "d.F", "bool", "d.F = v", "json!(d.F)", "v.as_bool()", "false"),
    "string": ("String", "String", "d.F.clone()", "&str", "d.F = v.to_string()", "json!(d.F)", "v.as_str().map(str::to_string)", "String::new()"),
    "mask": ("u32", "f64", "f64::from(d.F)", "f64", "d.F = v as i64 as u32", "json!(d.F)", "v.as_f64().map(|n| n as i64 as u32)", "0.0"),
    "color": ("Rgba", "Rgba", "d.F", "Rgba", "d.F = v", 'json!({ "$type": "Color", "value": [d.F.r, d.F.g, d.F.b, d.F.a] })', "color_of(v)", "Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }"),
    "v2": ("V2", "V2", "d.F", "V2", "d.F = v", 'json!({ "$type": "Vector2", "value": [d.F.x, d.F.y] })', "v2_of(v)", "V2 { x: 0.0, y: 0.0 }"),
    "v3": ("V3", "V3", "d.F", "V3", "d.F = v", 'json!({ "$type": "Vector3", "value": [d.F.x, d.F.y, d.F.z] })', "v3_of(v)", "V3::ZERO"),
    "texture": ("u32", "TextureRef", "TextureRef(d.F)", "TextureRef", "d.F = v.0", 'asset_ref("Texture", u64::from(d.F))', 'asset_handle(v, "Texture").map(|h| h as u32)', "TextureRef(0)"),
    "mesh": ("u32", "MeshRef", "MeshRef(d.F)", "MeshRef", "d.F = v.0", "mesh_value(d.F)", "mesh_of(v)", "MeshRef(0)"),
    "material": ("u32", "MaterialRef", "MaterialRef(d.F)", "MaterialRef", "d.F = v.0", 'asset_ref("Material", u64::from(d.F))', 'asset_handle(v, "Material").map(|h| h as u32)', "MaterialRef(0)"),
    "sprite": ("u32", "SpriteRef", "SpriteRef(d.F)", "SpriteRef", "d.F = v.0", 'asset_ref("Sprite", u64::from(d.F))', 'asset_handle(v, "Sprite").map(|h| h as u32)', "SpriteRef(0)"),
    "object": ("Option<ObjectId>", "ObjectId", "d.F.unwrap_or_else(none_object)", "ObjectId", "d.F = (v.to_raw() != 0).then_some(v)", "d.F.map_or(Value::Null, |o| object_ref(o.to_raw()))", "object_of(v)", "none_object()"),
}
# A colour that may be none (`Color | null`): `Option<Rgba>`, null in scene files.
TYPES["color?"] = ("Option<Rgba>", "Option<Rgba>", "d.F", "Option<Rgba>", "d.F = v", 'd.F.map_or(Value::Null, |c| json!({ "$type": "Color", "value": [c.r, c.g, c.b, c.a] }))', "if v.is_null() { Some(None) } else { color_of(v).map(Some) }", "None")
TYPES["enum"] = TYPES["string"]
# Handle-valued properties may be none (`null`): the declared type says so (`Texture | null`).
NULLABLE_HANDLES = ("texture", "mesh", "material", "sprite", "object")

VALUES = {
    "color": "Color = Rgba [r: f64, g: f64, b: f64, a: f64]",
    "v2": "Vector2 = V2 [x: f64, y: f64]",
    "v3": "Vector3 = V3 [x: f64, y: f64, z: f64]",
}
HANDLES = {
    "texture": "Texture = TextureRef",
    "mesh": "Mesh = MeshRef",
    "material": "Material = MaterialRef",
    "sprite": "Sprite = SpriteRef",
}


def default_lit(t, d):
    if isinstance(d, str) and d.startswith("="):
        return d[1:]
    if t == "color?":
        return "None"
    if t == "color":
        return "Rgba { r: %r, g: %r, b: %r, a: %r }" % tuple(float(x) for x in d)
    if t == "v2":
        return "V2 { x: %r, y: %r }" % tuple(float(x) for x in d)
    if t == "v3":
        return "V3 { x: %r, y: %r, z: %r }" % tuple(float(x) for x in d)
    if t in ("texture", "mesh", "material", "sprite"):
        return "0"
    if t == "object":
        return "None"
    if t == "bool":
        return "true" if d else "false"
    if t == "string" or t.startswith("enum:"):
        return '"%s".to_string()' % d
    if t in ("mask", "i32"):
        return str(d)
    return repr(float(d))




def minmax_api(low, rust, ts, doc):
    """Getter, setter, max getter and range setter of a MinMax field stored as [min, max]."""
    return """
        /// %(doc)s A fixed value, or `%(rust)s_max` above it for a random one per particle.
        #[property]
        pub fn %(rust)s(self) -> f64 {
            with_%(low)s(self, |d| d.%(rust)s[0]).unwrap_or(0.0)
        }

        /// Sets both ends.
        pub fn set_%(rust)s(self, v: f64) {
            with_%(low)s(self, |d| d.%(rust)s = [v, v]);
        }

        #[property]
        pub fn %(rust)s_max(self) -> f64 {
            with_%(low)s(self, |d| d.%(rust)s[1]).unwrap_or(0.0)
        }

        /// A random value between `min` and `max` per particle.
        pub fn set_%(rust)s_range(self, min: f64, max: f64) {
            with_%(low)s(self, |d| d.%(rust)s = [min, max]);
        }
""" % dict(low=low, rust=rust, doc=doc)


PS_MINMAX = [
    ("startLifetime", "start_lifetime", 5.0, "Seconds a particle lives."),
    ("startSpeed", "start_speed", 5.0, "Metres per second."),
    ("startSize", "start_size", 1.0, "Metres."),
    ("startRotation", "start_rotation", 0.0, "Degrees."),
    ("angularVelocity", "angular_velocity", 0.0, "Degrees per second."),
]
ONE = (1, 1, 1, 1)
MASK = 0xFFFFFFFF

COMPONENTS = [
    dict(
        name="Light", low="light", ref="LightRef", store="LIGHTS", data="LightData",
        fields=[
            ("type", "r#type", "enum:directional|point|spot", "directional", "`directional`, `point` or `spot`."),
            ("color", "color", "color", ONE, ""),
            ("intensity", "intensity", "f64", 1.0, ""),
            ("range", "range", "f64", 10.0, "How far point and spot lights reach (metres)."),
            ("spotAngle", "spot_angle", "f64", 30.0, "A spot light's full cone angle in degrees."),
            ("innerSpotAngle", "inner_spot_angle", "f64", 21.8, "Where a spot light starts fading towards its edge (degrees, full angle)."),
            ("haloSize", "halo_size", "f64", 0.0, "A glow around the light this many metres across, in its colour (0: none)."),
            ("beamIntensity", "beam_intensity", "f64", 0.0, "A spot light's visible beam as long as `range` (0: none)."),
            ("shadows", "shadows", "enum:none|hard|soft", "none", "Shadows it casts (Unity's `LightShadows`)."),
            ("shadowStrength", "shadow_strength", "f64", 1.0, "How dark its shadows are, 0 to 1."),
            ("cullingMask", "culling_mask", "mask", MASK, "The layers it lights, a bit each (all by default)."),
            ("volumetricIntensity", "volumetric_intensity", "f64", 1.0, "How strongly it lights the fog for light shafts (`RenderSettings.volumetricFog`): 1 as brightly as it lights surfaces, 0 not at all (HDRP's volumetric multiplier)."),
            ("spotShape", "spot_shape", "enum:cone|pyramid", "cone", "A spot light's shape (HDRP's): a `cone`, or a `pyramid` `spotAngle` high and `aspectRatio` times as wide (in tangent), its width along the object's x."),
            ("aspectRatio", "aspect_ratio", "f64", 1.0, "A pyramid spot light's width over its height."),
        ],
        api="""
        /// How many lights exist.
        pub fn count() -> f64 {
            super::lights().len() as f64
        }

        /// The `index`-th light, in the order they were made (none past the end).
        pub fn at(index: f64) -> LightRef {
            super::lights()
                .get(index as usize)
                .map_or(LightRef(0), |(r, _)| *r)
        }
""",
    ),
    dict(
        name="FogVolume", low="fog_volume", ref="FogVolumeRef", store="FOG_VOLUMES", data="FogVolumeData",
        fields=[
            ("size", "size", "v3", (10, 2, 10), "The box's size in metres, before the object's scale."),
            ("density", "density", "f64", 0.5, "How thick it is inside, per metre (0.5: a metre of it hides 40% of what is behind)."),
            ("color", "color", "color", (0.75, 0.78, 0.82, 1), "Its colour (linear)."),
            ("blendDistance", "blend_distance", "f64", 1.0, "How far in from its faces it fades in, in metres (0: hard edges)."),
            ("shape", "shape", "enum:box|ellipsoid", "box", "`box`, or `ellipsoid`: a soft round blob inside the box (mist over a pond)."),
        ],
    ),
    dict(
        name="PlaneRenderer", low="plane_renderer", ref="PlaneRendererRef", store="PLANES", data="PlaneData",
        fields=[
            ("size", "size", "v2", (1, 1), "Metres across."),
            ("color", "color", "color", ONE, ""),
        ],
    ),
    dict(
        name="Mirror", low="mirror", ref="MirrorRef", store="MIRRORS", data="MirrorData",
        fields=[
            ("hasNormal", "has_normal", "bool", False, "Whether `normal` was set; otherwise up (+y) for planes and +z for quads."),
            ("normal", "normal", "v3", (0, 1, 0), "The plane's normal in the object's space."),
            ("cullingMask", "culling_mask", "mask", MASK, "The layers it reflects, a bit each (all by default; set before it starts)."),
        ],
        skip_save=["hasNormal", "normal"],
        set_hook={"normal": "d.has_normal = true;"},
        extra_save='m.insert("normal".into(), if d.has_normal { json!({ "$type": "Vector3", "value": [d.normal.x, d.normal.y, d.normal.z] }) } else { Value::Null });',
        extra_load='"normal" => { if v.is_null() { d.has_normal = false; Some(()) } else { v3_of(v).map(|n| { d.has_normal = true; d.normal = n; }) } }',
    ),
    dict(
        name="ReflectionProbe", low="reflection_probe", ref="ReflectionProbeRef", store="PROBES", data="ProbeData",
        fields=[
            ("size", "size", "v3", (10, 10, 10), "The box of influence, centred on the object (metres)."),
            ("resolution", "resolution", "f64", 128.0, "The cubemap's face size in texels (a power of two, 16 to 1024)."),
            ("boxProjection", "box_projection", "bool", False, "Correct reflections for the box's walls (rooms)."),
            ("cullingMask", "culling_mask", "mask", MASK, "The layers it captures, a bit each (all by default; set before it starts)."),
            ("refreshMode", "refresh_mode", "enum:onAwake|everyFrame|viaScripting", "onAwake", "When it renders: once, every frame, or on `renderProbe()`."),
            ("renderRequests", "render_requests", "f64", 0.0, "How many times `renderProbe()` was called (the host renders when it grows)."),
        ],
        skip_save=["renderRequests"],
        api="""
        /// Captures the scene again on the next frame (after moving things, say).
        pub fn render_probe(self) {
            with_reflection_probe(self, |d| d.render_requests += 1.0);
        }
""",
    ),
    dict(
        name="WorldPanel", low="world_panel", ref="WorldPanelRef", store="PANELS", data="PanelData",
        fields=[
            ("width", "width", "f64", 800.0, "Width in UI pixels."),
            ("height", "height", "f64", 600.0, "Height in UI pixels."),
            ("pixelsPerMeter", "pixels_per_meter", "f64", 500.0, "UI pixels per metre: 500 makes an 800x600 panel 1.6 x 1.2 m."),
            ("resolution", "resolution", "f64", 2.0, "Texture pixels per UI pixel: higher keeps text sharp up close."),
            ("tint", "tint", "color", ONE, "A colour multiplied over the panel (alpha fades it)."),
            ("interactive", "interactive", "bool", True, "Whether the pointer can use it."),
        ],
        api="""
        /// Its width in metres.
        #[property]
        pub fn world_width(self) -> f64 {
            with_world_panel(self, |d| d.width / d.pixels_per_meter).unwrap_or(0.0)
        }

        /// Its height in metres.
        #[property]
        pub fn world_height(self) -> f64 {
            with_world_panel(self, |d| d.height / d.pixels_per_meter).unwrap_or(0.0)
        }
""",
    ),
    dict(
        name="SpriteRenderer", low="sprite_renderer", ref="SpriteRendererRef", store="SPRITE_RENDERERS", data="SpriteRendererData",
        fields=[
            ("sprite", "sprite", "sprite", 0, ""),
            ("color", "color", "color", ONE, ""),
            ("flipX", "flip_x", "bool", False, ""),
            ("flipY", "flip_y", "bool", False, ""),
            ("sortingOrder", "sorting_order", "i32", 0, "Draw order: higher draws over lower, whatever the distance (Unity's `sortingOrder`)."),
        ],
    ),
    dict(
        name="LineRenderer2D", low="line_renderer2d", ref="LineRenderer2DRef", store="STROKES", data="StrokeData",
        fields=[
            ("width", "width", "f64", 0.05, "Full width, in world units."),
            ("color", "color", "color", ONE, ""),
            ("loop", "r#loop", "bool", False, "Joins the last point back to the first (a closed shape)."),
            ("fillColor", "fill_color", "color?", None, "Fills the shape in this colour (null: no fill); a filled shape always closes."),
            ("fillRule", "fill_rule", "enum:evenodd|nonzero", "evenodd", "Where a self-crossing outline is inside: `evenodd` leaves a star's middle empty (GDI's `Alternate`), `nonzero` fills it (`Winding`)."),
            ("sortingOrder", "sorting_order", "i32", 0, "Draw order among 2D shapes: higher draws over lower, equal ones in the order they were made."),
        ],
        extra_fields=["pub points: Vec<V2>,"],
        extra_defaults=["points: Vec::new(),"],
        extra_save='m.insert("points".into(), Value::Array(d.points.iter().map(|p| json!({ "$type": "Vector2", "value": [p.x, p.y] })).collect()));',
        extra_load='"points" => v.as_array().and_then(|a| a.iter().map(v2_of).collect::<Option<Vec<V2>>>()).map(|p| d.points = p)',
        extra_value_types=["v2"],
        api="""
        /// The points (on the object's XY plane): a copy, so write a new array to change them.
        #[property]
        pub fn points(self) -> Vec<V2> {
            with_line_renderer2d(self, |d| d.points.clone()).unwrap_or_default()
        }

        /// Replaces the points (on the object's XY plane); a new array redraws it.
        #[like]
        pub fn set_points(self, points: &[V2]) {
            with_line_renderer2d(self, |d| d.points = points.to_vec());
        }

        #[property]
        pub fn point_count(self) -> f64 {
            with_line_renderer2d(self, |d| d.points.len() as f64).unwrap_or(0.0)
        }

        /// The i-th point (the origin past the end).
        pub fn point_at(self, index: f64) -> V2 {
            with_line_renderer2d(self, |d| d.points.get(index as usize).copied())
                .flatten()
                .unwrap_or(V2 { x: 0.0, y: 0.0 })
        }
""",
    ),
    dict(
        name="TextMesh", low="text_mesh", ref="TextMeshRef", store="TEXT_MESHES", data="TextMeshData",
        fields=[
            ("text", "text", "string", "", "The text; a line break starts a new line."),
            ("fontSize", "font_size", "f64", 32.0, "In UI pixels."),
            ("color", "color", "color", ONE, ""),
            ("font", "font", "string", "", "A family loaded with `loadFont` (empty: Iron's default, Inter)."),
            ("fontWeight", "font_weight", "f64", 400.0, "100 to 900; 400 is regular, 700 bold."),
            ("alignment", "alignment", "enum:left|center|right", "center", "How lines line up with each other."),
            ("maxWidth", "max_width", "f64", 0.0, "Wraps lines longer than this many UI pixels (0: only at line breaks)."),
            ("pixelsPerUnit", "pixels_per_unit", "f64", 100.0, "UI pixels per world unit (like a sprite's pixels per unit)."),
            ("resolution", "resolution", "f64", 2.0, "Texture pixels per UI pixel: more is sharper and costs more memory."),
        ],
    ),
    dict(
        name="MeshRenderer", low="mesh_renderer", ref="MeshRendererRef", store="MESH_RENDERERS", data="MeshRendererData",
        kind_on_filter="!internal_renderer(*r)",
        fields=[
            ("mesh", "mesh", "mesh", '=primitive_mesh("cube").0', "A built-in mesh by name (`mesh: \"cube\"`) or a script `Mesh`."),
            ("material", "material", "material", "=create_material().0", ""),
            ("shadowCastingMode", "shadow_casting_mode", "enum:on|off", "on", "Whether it casts shadows (Unity's `shadowCastingMode`: `on` or `off`)."),
        ],
    ),
    dict(
        name="SkinnedMeshRenderer", low="skinned_mesh_renderer", ref="SkinnedMeshRendererRef", store="SKINNED", data="SkinnedData",
        fields=[
            ("mesh", "mesh", "mesh", '=primitive_mesh("cube").0', "The mesh (with bone weights and bind poses)."),
            ("material", "material", "material", "=create_material().0", ""),
            ("shadowCastingMode", "shadow_casting_mode", "enum:on|off", "on", "Whether it casts shadows."),
            ("rootBone", "root_bone", "object", None, "The skeleton's top bone."),
        ],
        extra_fields=["pub bones: Vec<ObjectId>,"],
        extra_defaults=["bones: Vec::new(),"],
        extra_save='m.insert("bones".into(), Value::Array(d.bones.iter().map(|b| object_ref(b.to_raw())).collect()));',
        extra_load='"bones" => v.as_array().and_then(|a| a.iter().map(|x| object_of(x).flatten()).collect::<Option<Vec<ObjectId>>>()).map(|b| d.bones = b)',
        api="""
        /// Appends a bone: the objects the mesh's bones follow, in the order its bone weights and
        /// bind poses use. Not a `MeshRenderer` to `getComponent`.
        pub fn add_bone(self, bone: ObjectId) {
            with_skinned_mesh_renderer(self, |d| d.bones.push(bone));
        }

        pub fn clear_bones(self) {
            with_skinned_mesh_renderer(self, |d| d.bones.clear());
        }

        /// The bones, in the order the mesh's bone weights and bind poses use.
        #[property]
        pub fn bones(self) -> Vec<ObjectId> {
            with_skinned_mesh_renderer(self, |d| d.bones.clone()).unwrap_or_default()
        }

        pub fn set_bones(self, bones: &[ObjectId]) {
            with_skinned_mesh_renderer(self, |d| d.bones = bones.to_vec());
        }

        #[property]
        pub fn bone_count(self) -> f64 {
            with_skinned_mesh_renderer(self, |d| d.bones.len() as f64).unwrap_or(0.0)
        }

        /// The i-th bone (no object past the end).
        pub fn bone_at(self, index: f64) -> ObjectId {
            with_skinned_mesh_renderer(self, |d| d.bones.get(index as usize).copied())
                .flatten()
                .unwrap_or_else(none_object)
        }
""",
    ),
    dict(
        name="LODGroup", low="lod_group", ref="LODGroupRef", store="LOD_GROUPS", data="LodData",
        fields=[
            ("size", "size", "f64", 1.0, "The group's size in metres before its object's scale (its height, say)."),
            ("forced", "forced", "i32", -1, "The level always shown (-1: chosen by distance)."),
        ],
        skip_save=["forced"],
        extra_fields=["/// Per level: the screen height below which the next level takes over, and its renderers.", "pub lods: Vec<(f64, Vec<u32>)>,"],
        extra_defaults=["lods: Vec::new(),"],
        extra_handles=["MeshRenderer = MeshRendererRef"],
        extra_save='m.insert("lods".into(), Value::Array(d.lods.iter().map(|(h, rs)| json!({ "screenRelativeTransitionHeight": h, "renderers": rs.iter().map(|r| component_ref("MeshRenderer", u64::from(*r))).collect::<Vec<_>>() })).collect()));',
        extra_load='"lods" => v.as_array().and_then(|a| a.iter().map(|l| { let h = l.get("screenRelativeTransitionHeight")?.as_f64()?; let rs = l.get("renderers")?.as_array()?.iter().map(|r| r.get("handle").and_then(Value::as_u64).map(|h| h as u32)).collect::<Option<Vec<u32>>>()?; Some((h, rs)) }).collect::<Option<Vec<_>>>()).map(|l| d.lods = l)',
        api="""
        /// Removes every level.
        pub fn clear_lods(self) {
            with_lod_group(self, |d| d.lods.clear());
        }

        /// Adds a level after the others, shown down to this screen height (a fraction of the
        /// view's height, 0 to 1). Heights must go down from the first level: a higher one is
        /// ignored.
        pub fn add_lod(self, screen_relative_transition_height: f64) {
            with_lod_group(self, |d| {
                if d.lods.last().is_none_or(|l| screen_relative_transition_height <= l.0) {
                    d.lods.push((screen_relative_transition_height, Vec::new()));
                }
            });
        }

        /// Adds a renderer to a level.
        pub fn add_lod_renderer(self, lod: f64, renderer: MeshRendererRef) {
            with_lod_group(self, |d| {
                if let Some(l) = d.lods.get_mut(lod as usize) {
                    l.1.push(renderer.0);
                }
            });
        }

        #[property]
        pub fn lod_count(self) -> f64 {
            with_lod_group(self, |d| d.lods.len() as f64).unwrap_or(0.0)
        }

        /// The transition height of a level.
        pub fn lod_height(self, lod: f64) -> f64 {
            with_lod_group(self, |d| d.lods.get(lod as usize).map(|l| l.0))
                .flatten()
                .unwrap_or(0.0)
        }

        pub fn lod_renderer_count(self, lod: f64) -> f64 {
            with_lod_group(self, |d| d.lods.get(lod as usize).map(|l| l.1.len() as f64))
                .flatten()
                .unwrap_or(0.0)
        }

        pub fn lod_renderer_at(self, lod: f64, index: f64) -> MeshRendererRef {
            MeshRendererRef(
                with_lod_group(self, |d| {
                    d.lods
                        .get(lod as usize)
                        .and_then(|l| l.1.get(index as usize))
                        .copied()
                })
                .flatten()
                .unwrap_or(0),
            )
        }

        /// Always shows this level (-1: back to choosing by distance).
        pub fn force_lod(self, index: f64) {
            with_lod_group(self, |d| d.forced = index as i32);
        }
""",
    ),
    dict(
        name="CameraFollow", low="camera_follow", ref="CameraFollowRef", store="FOLLOWS", data="FollowData",
        fields=[
            ("target", "target", "object", None, "The object to follow."),
            ("offset", "offset", "v3", (0, 3, 6), "Where to sit, from the target."),
            ("offsetInTargetSpace", "offset_in_target_space", "bool", False, "Rotate the offset with the target (behind a car), or keep it in world space."),
            ("smoothTime", "smooth_time", "f64", 0.25, "About how long catching up takes, in seconds (0: instantly)."),
            ("lookAtTarget", "look_at_target", "bool", True, ""),
        ],
        extra_fields=["pub velocity: [f64; 3],"],
        extra_defaults=["velocity: [0.0; 3],"],
        items="""
/// Runs every `CameraFollow`: follows its target from an offset, easing into place, and looks at
/// it. The host calls it once a frame after the other late updates (`onLateUpdate`).
pub fn late_update_camera_follows(dt: f64) {
    use iron_math::mathf_api::smooth_damp;
    use iron_scene::api::scene_api as scene;
    for (r, d) in camera_follows() {
        let Some(target) = d.target.filter(|t| iron_scene::api::with_scene(|s| s.contains(*t)).unwrap_or(false)) else {
            continue;
        };
        let offset = if d.offset_in_target_space {
            scene::rotation(target).rotate(d.offset)
        } else {
            d.offset
        };
        let target_pos = scene::position(target);
        let goal = target_pos.add(offset);
        let at = scene::position(d.object);
        if d.smooth_time <= 0.0 {
            scene::set_position(d.object, goal);
        } else {
            let dt = dt.max(1e-4);
            let mut velocity = d.velocity;
            let mut axis = |i: usize, a: f64, b: f64| {
                let r = smooth_damp(a, b, velocity[i], d.smooth_time, dt, f64::NAN);
                velocity[i] = r.y;
                r.x
            };
            let p = V3 { x: axis(0, at.x, goal.x), y: axis(1, at.y, goal.y), z: axis(2, at.z, goal.z) };
            scene::set_position(d.object, p);
            with_camera_follow(r, |d| d.velocity = velocity);
        }
        if d.look_at_target {
            iron_scene::api::TransformRef(d.object).look_at(target_pos);
        }
    }
}
""",
    ),
    dict(
        name="CameraOrbit", low="camera_orbit", ref="CameraOrbitRef", store="ORBITS", data="OrbitData",
        fields=[
            ("target", "target", "object", None, "Orbits this object (or `center` when there is none)."),
            ("center", "center", "v3", (0, 0, 0), "Orbits this point when there is no target."),
            ("distance", "distance", "f64", 8.0, ""),
            ("yaw", "yaw", "f64", 0.0, "Degrees."),
            ("pitch", "pitch", "f64", 20.0, "Degrees."),
            ("minPitch", "min_pitch", "f64", -80.0, ""),
            ("maxPitch", "max_pitch", "f64", 80.0, ""),
            ("autoRotate", "auto_rotate", "f64", 0.0, "Degrees per second."),
        ],
        items="""
/// Runs every `CameraOrbit` (once a frame after the other late updates).
pub fn late_update_camera_orbits(dt: f64) {
    use iron_math::{Q, mathf_api::repeat};
    use iron_scene::api::scene_api as scene;
    for (r, d) in camera_orbits() {
        let yaw = repeat(d.yaw + d.auto_rotate * dt, 360.0);
        let pitch = d.pitch.clamp(d.min_pitch.min(d.max_pitch), d.max_pitch.max(d.min_pitch));
        with_camera_orbit(r, |d| {
            d.yaw = yaw;
            d.pitch = pitch;
        });
        let center = match d.target {
            Some(t) if iron_scene::api::with_scene(|s| s.contains(t)).unwrap_or(false) => scene::position(t),
            _ => d.center,
        };
        let rotation = Q::euler(-pitch, yaw, 0.0);
        let back = V3 { x: 0.0, y: 0.0, z: 1.0 };
        scene::set_position(d.object, center.add(rotation.rotate(back).scale(d.distance)));
        iron_scene::api::TransformRef(d.object).look_at(center);
    }
}
""",
    ),
    dict(
        name="ParticleSystem", low="particle_system", ref="ParticleSystemRef", store="PARTICLES", data="ParticleData",
        fields=[
            ("duration", "duration", "f64", 5.0, "Seconds per cycle."),
            ("loop", "r#loop", "bool", True, ""),
            ("playOnAwake", "play_on_awake", "bool", True, ""),
            ("startDelay", "start_delay", "f64", 0.0, ""),
            ("gravityModifier", "gravity_modifier", "f64", 0.0, "Multiplies gravity (9.81 m/s2 down)."),
            ("drag", "drag", "f64", 0.0, "Fraction of speed lost per second."),
            ("simulationSpace", "simulation_space", "enum:local|world", "local", "`world`: particles stay where they were emitted when the object moves."),
            ("maxParticles", "max_particles", "i32", 1000, ""),
            ("randomSeed", "random_seed", "i32", 1, ""),
            ("rateOverTime", "rate_over_time", "f64", 10.0, "Particles emitted per second."),
            ("texture", "texture", "texture", 0, "The particle image (a soft round spot by default)."),
            ("blendMode", "blend_mode", "enum:alpha|additive", "alpha", "`additive` makes them glow."),
            ("stopAction", "stop_action", "enum:none|destroy", "none", "What happens once it has stopped and its particles are gone."),
            ("shapeType", "shape_type", "enum:point|sphere|hemisphere|cone|box", "cone", "Where particles start; they go along the object's up (+y)."),
            ("shapeRadius", "shape_radius", "f64", 1.0, "Sphere, hemisphere and cone."),
            ("shapeAngle", "shape_angle", "f64", 25.0, "Cone: degrees from straight up."),
            ("shapeSize", "shape_size", "v3", (1, 1, 1), "Box."),
        ],
        skip_save=["shapeType", "shapeRadius", "shapeAngle", "shapeSize"],
        set_hook={"playOnAwake": "d.alive = v;"},
        extra_fields=[
            "pub start_lifetime: [f64; 2],", "pub start_speed: [f64; 2],", "pub start_size: [f64; 2],",
            "pub start_rotation: [f64; 2],", "pub angular_velocity: [f64; 2],",
            "pub start_color: Rgba,", "pub start_color2: Option<Rgba>,",
            "/// time, count, cycles, interval", "pub bursts: Vec<[f64; 4]>,",
            "/// time, value", "pub size_over_lifetime: Option<Vec<[f64; 2]>>,",
            "pub color_over_lifetime: Option<Vec<(f64, Rgba)>>,",
            "/// tilesX, tilesY, cycles", "pub texture_sheet: Option<[f64; 3]>,",
            "pub alive: bool,", "pub paused: bool,", "pub particle_count: u32,",
        ],
        extra_defaults=[
            "start_lifetime: [5.0, 5.0],", "start_speed: [5.0, 5.0],", "start_size: [1.0, 1.0],",
            "start_rotation: [0.0, 0.0],", "angular_velocity: [0.0, 0.0],",
            "start_color: Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },", "start_color2: None,",
            "bursts: Vec::new(),", "size_over_lifetime: None,", "color_over_lifetime: None,",
            "texture_sheet: None,", "alive: true,", "paused: false,", "particle_count: 0,",
        ],
        extra_save="""for (k, v) in [("startLifetime", d.start_lifetime), ("startSpeed", d.start_speed), ("startSize", d.start_size), ("startRotation", d.start_rotation), ("angularVelocity", d.angular_velocity)] { m.insert(k.into(), minmax_json(v)); }
                m.insert("startColor".into(), start_color_json(d.start_color, d.start_color2, true));
                m.insert("bursts".into(), Value::Array(d.bursts.iter().map(|b| json!({ "time": b[0], "count": b[1], "cycles": b[2], "interval": b[3] })).collect()));
                m.insert("shape".into(), shape_json(&d.shape_type, d.shape_radius, d.shape_angle, d.shape_size));
                m.insert("sizeOverLifetime".into(), d.size_over_lifetime.as_ref().map_or(Value::Null, |k| Value::Array(k.iter().map(|k| json!({ "time": k[0], "value": k[1] })).collect())));
                m.insert("colorOverLifetime".into(), d.color_over_lifetime.as_ref().map_or(Value::Null, |k| Value::Array(k.iter().map(|(t, c)| json!({ "time": t, "color": color_json(*c) })).collect())));
                m.insert("textureSheet".into(), d.texture_sheet.map_or(Value::Null, |s| json!({ "tilesX": s[0], "tilesY": s[1], "cycles": s[2] })));""",
        extra_load="""    "startLifetime" => minmax_of(v).map(|x| d.start_lifetime = x),
                "startSpeed" => minmax_of(v).map(|x| d.start_speed = x),
                "startSize" => minmax_of(v).map(|x| d.start_size = x),
                "startRotation" => minmax_of(v).map(|x| d.start_rotation = x),
                "angularVelocity" => minmax_of(v).map(|x| d.angular_velocity = x),
                "startColor" => start_color_of(v).map(|(a, b)| { d.start_color = a; d.start_color2 = b; }),
                "bursts" => v.as_array().and_then(|a| a.iter().map(|b| Some([b.get("time")?.as_f64()?, b.get("count")?.as_f64()?, b.get("cycles").and_then(Value::as_f64).unwrap_or(1.0), b.get("interval").and_then(Value::as_f64).unwrap_or(0.01)])).collect::<Option<Vec<_>>>()).map(|b| d.bursts = b),
                "shape" => shape_of(v).map(|(t, r, a, s)| { d.shape_type = t; if let Some(r) = r { d.shape_radius = r; } if let Some(a) = a { d.shape_angle = a; } if let Some(s) = s { d.shape_size = s; } }),
                "sizeOverLifetime" => if v.is_null() { d.size_over_lifetime = None; Some(()) } else { v.as_array().and_then(|a| a.iter().map(|k| Some([k.get("time")?.as_f64()?, k.get("value")?.as_f64()?])).collect::<Option<Vec<_>>>()).map(|k| d.size_over_lifetime = Some(k)) },
                "colorOverLifetime" => if v.is_null() { d.color_over_lifetime = None; Some(()) } else { v.as_array().and_then(|a| a.iter().map(|k| Some((k.get("time")?.as_f64()?, color_of(k.get("color")?)?))).collect::<Option<Vec<_>>>()).map(|k| d.color_over_lifetime = Some(k)) },
                "textureSheet" => if v.is_null() { d.texture_sheet = None; Some(()) } else { (|| Some([v.get("tilesX")?.as_f64()?, v.get("tilesY")?.as_f64()?, v.get("cycles").and_then(Value::as_f64).unwrap_or(1.0)]))().map(|s| d.texture_sheet = Some(s)) }""",
        extra_value_types=["color"],
        api="".join(minmax_api("particle_system", r, t, doc) for t, r, _, doc in PS_MINMAX) + """
        /// A colour, or `setStartColorRange` for a random mix of two per particle.
        #[property]
        pub fn start_color(self) -> Rgba {
            with_particle_system(self, |d| d.start_color).unwrap_or(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })
        }

        pub fn set_start_color(self, c: Rgba) {
            with_particle_system(self, |d| {
                d.start_color = c;
                d.start_color2 = None;
            });
        }

        pub fn set_start_color_range(self, a: Rgba, b: Rgba) {
            with_particle_system(self, |d| {
                d.start_color = a;
                d.start_color2 = Some(b);
            });
        }

        /// The second colour of a range (the first when there is none).
        #[property]
        pub fn start_color_max(self) -> Rgba {
            with_particle_system(self, |d| d.start_color2.unwrap_or(d.start_color)).unwrap_or(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })
        }

        /// Adds a burst: `count` particles at `time` seconds into each cycle, firing `cycles`
        /// times (0 keeps firing every `interval`).
        #[optional(cycles = 1, interval = 0.01)]
        pub fn add_burst(self, time: f64, count: f64, cycles: f64, interval: f64) {
            with_particle_system(self, |d| d.bursts.push([time, count, cycles, interval]));
        }

        pub fn clear_bursts(self) {
            with_particle_system(self, |d| d.bursts.clear());
        }

        #[property]
        pub fn burst_count(self) -> f64 {
            with_particle_system(self, |d| d.bursts.len() as f64).unwrap_or(0.0)
        }

        /// Adds a key to the size over the particle's life: `time` from 0 (birth) to 1 (death),
        /// linear between keys; `value` multiplies the start size.
        pub fn add_size_key(self, time: f64, value: f64) {
            with_particle_system(self, |d| d.size_over_lifetime.get_or_insert_with(Vec::new).push([time, value]));
        }

        pub fn clear_size_over_lifetime(self) {
            with_particle_system(self, |d| d.size_over_lifetime = None);
        }

        /// Adds a key to the colour over life; it multiplies the start colour (alpha too: fade out
        /// with alpha 0 at time 1).
        pub fn add_color_key(self, time: f64, color: Rgba) {
            with_particle_system(self, |d| d.color_over_lifetime.get_or_insert_with(Vec::new).push((time, color)));
        }

        pub fn clear_color_over_lifetime(self) {
            with_particle_system(self, |d| d.color_over_lifetime = None);
        }

        /// A flipbook: the texture cut into `tilesX` x `tilesY` frames, played `cycles` times per life.
        #[optional(cycles = 1)]
        pub fn set_texture_sheet(self, tiles_x: f64, tiles_y: f64, cycles: f64) {
            with_particle_system(self, |d| d.texture_sheet = Some([tiles_x, tiles_y, cycles]));
        }

        pub fn clear_texture_sheet(self) {
            with_particle_system(self, |d| d.texture_sheet = None);
        }

        /// Live particles (as of the last report).
        #[property]
        pub fn particle_count(self) -> f64 {
            with_particle_system(self, |d| f64::from(d.particle_count)).unwrap_or(0.0)
        }

        /// Emitting or has particles left, and not paused.
        #[property]
        pub fn is_playing(self) -> bool {
            with_particle_system(self, |d| d.alive && !d.paused).unwrap_or(false)
        }

        #[property]
        pub fn is_paused(self) -> bool {
            with_particle_system(self, |d| d.paused).unwrap_or(false)
        }

        /// Starts (or resumes) emitting.
        pub fn play(self) {
            with_particle_system(self, |d| {
                d.paused = false;
                d.alive = true;
            });
            control(self, "play", 0.0);
        }

        /// Stops emitting; with `clear`, removes the live particles too.
        pub fn stop(self, clear: bool) {
            control(self, if clear { "stopClear" } else { "stop" }, 0.0);
        }

        pub fn pause(self) {
            with_particle_system(self, |d| d.paused = true);
            control(self, "pause", 0.0);
        }

        pub fn clear(self) {
            control(self, "clear", 0.0);
        }

        /// Emits `count` particles at once (works while stopped, too).
        pub fn emit(self, count: f64) {
            control(self, "emit", count);
        }

        /// Calls `f()` once it has stopped and its particles are gone.
        pub fn on_stopped(self, f: Callback<()>) {
            add_stopped(self, f);
        }
""",
    ),
    dict(
        name="LineRenderer", low="line_renderer", ref="LineRendererRef", store="LINES", data="LineData",
        fields=[
            ("width", "width", "f64", 0.05, "Tube thickness (diameter, metres)."),
            ("sides", "sides", "i32", 8, "Points around the tube."),
            ("smoothing", "smoothing", "i32", 3, "Extra points between each pair of positions, for curves."),
            ("material", "material", "material", "=create_line_material().0", ""),
        ],
        extra_fields=["/// The points it goes through, in world space.", "pub positions: Vec<V3>,"],
        extra_defaults=["positions: Vec::new(),"],
        extra_value_types=["v3"],
        extra_save='m.insert("positions".into(), Value::Array(d.positions.iter().map(|p| v3_json(*p)).collect()));',
        extra_load='"positions" => v.as_array().and_then(|a| a.iter().map(v3_of).collect::<Option<Vec<V3>>>()).map(|p| d.positions = p)',
        attach_hook="line_attached(r);",
        remove_hook="line_removed(r);",
        awake_hook="line_redraw(r);",
        api="""
        /// The points it goes through (world space): a copy, so write a new array to change them.
        #[property]
        pub fn positions(self) -> Vec<V3> {
            with_line_renderer(self, |d| d.positions.clone()).unwrap_or_default()
        }

        /// Replaces the points (world space); a new array redraws the tube.
        #[like]
        pub fn set_positions(self, points: &[V3]) {
            with_line_renderer(self, |d| d.positions = points.to_vec());
        }

        #[property]
        pub fn position_count(self) -> f64 {
            with_line_renderer(self, |d| d.positions.len() as f64).unwrap_or(0.0)
        }

        /// The i-th point (the origin past the end).
        pub fn position_at(self, index: f64) -> V3 {
            with_line_renderer(self, |d| d.positions.get(index as usize).copied())
                .flatten()
                .unwrap_or(V3::ZERO)
        }

        /// Rebuilds the tube now if the points or its look changed (the host does it before each
        /// frame with `pre_render_lines`).
        pub fn redraw(self) {
            line_redraw(self);
        }
""",
    ),
    dict(
        name="Cloth", low="cloth", ref="ClothRef", store="CLOTHS_STORE", data="ClothData",
        fields=[
            ("stretchingStiffness", "stretching_stiffness", "f64", 1.0, "0 (rubbery) to 1 (keeps its edges' lengths)."),
            ("bendingStiffness", "bending_stiffness", "f64", 0.3, "0 (folds freely) to 1 (resists folding)."),
            ("damping", "damping", "f64", 0.1, "0..1: how much of its motion it loses each second."),
            ("useGravity", "use_gravity", "bool", True, ""),
            ("externalAcceleration", "external_acceleration", "v3", (0, 0, 0), "A steady acceleration on every vertex (steady wind)."),
            ("randomAcceleration", "random_acceleration", "v3", (0, 0, 0), "Up to this much more, varying (gusts)."),
            ("friction", "friction", "f64", 0.5, "0..1: how much sliding over colliders slows it."),
            ("solverIterations", "solver_iterations", "f64", 8.0, "Constraint passes per step: more is stiffer, and slower to run."),
            ("pinRadius", "pin_radius", "f64", 0.05, "Vertices within this of the pins stay put."),
            ("rebuildRequests", "rebuild_requests", "f64", 0.0, "How many times the simulation was (re)started (the host restarts when it grows)."),
            ("sourceMesh", "source_mesh", "mesh", 0, "The mesh the simulation started from."),
            ("simulatedMesh", "simulated_mesh", "mesh", 0, "The mesh the engine fills in each step; the object's renderer draws it."),
        ],
        skip_save=["rebuildRequests", "sourceMesh", "simulatedMesh"],
        extra_fields=[
            "/// Vertices within `pin_radius` of these points (object space) stay put.", "pub pins: Vec<V3>,",
            "/// Vertices in this object-space box (min, max) stay put.", "pub pin_box: Option<(V3, V3)>,",
            "pub sphere_colliders: Vec<ObjectId>,", "pub capsule_colliders: Vec<ObjectId>,",
        ],
        extra_defaults=["pins: Vec::new(),", "pin_box: None,", "sphere_colliders: Vec::new(),", "capsule_colliders: Vec::new(),"],
        extra_value_types=["v3"],
        extra_save="""m.insert("pins".into(), Value::Array(d.pins.iter().map(|p| v3_json(*p)).collect()));
                m.insert("pinBox".into(), d.pin_box.map_or(Value::Null, |(a, b)| json!({ "min": v3_json(a), "max": v3_json(b) })));
                m.insert("sphereColliders".into(), Value::Array(d.sphere_colliders.iter().map(|o| object_ref(o.to_raw())).collect()));
                m.insert("capsuleColliders".into(), Value::Array(d.capsule_colliders.iter().map(|o| object_ref(o.to_raw())).collect()));""",
        extra_load="""    "pins" => v.as_array().and_then(|a| a.iter().map(v3_of).collect::<Option<Vec<V3>>>()).map(|p| d.pins = p),
                "pinBox" => if v.is_null() { d.pin_box = None; Some(()) } else { (|| Some((v3_of(v.get("min")?)?, v3_of(v.get("max")?)?)))().map(|b| d.pin_box = Some(b)) },
                "sphereColliders" => v.as_array().and_then(|a| a.iter().map(|x| object_of(x).flatten()).collect::<Option<Vec<ObjectId>>>()).map(|o| d.sphere_colliders = o),
                "capsuleColliders" => v.as_array().and_then(|a| a.iter().map(|x| object_of(x).flatten()).collect::<Option<Vec<ObjectId>>>()).map(|o| d.capsule_colliders = o)""",
        remove_hook="cloth_removed(r);",
        extra_records=["ClothBox = ClothBox [min: Vector3Like, max: Vector3Like]"],
        items="""/// A cloth's pin box: vertices inside it stay put.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothBox {
    pub min: V3,
    pub max: V3,
}""",
        awake_hook="cloth_rebuild(r);",
        api="""
        /// Starts the simulation again from the mesh with the current settings. The object needs a
        /// `MeshRenderer`: it then draws the simulated copy. (The TypeScript started on `onStart`;
        /// here a scene load does it, and a script calls `rebuild()` after setting things up.)
        pub fn rebuild(self) {
            cloth_rebuild(self);
        }

        /// A point (object space): vertices near it stay put.
        pub fn add_pin(self, point: V3) {
            with_cloth(self, |d| d.pins.push(point));
        }

        pub fn clear_pins(self) {
            with_cloth(self, |d| d.pins.clear());
        }

        /// Vertices in this object-space box stay put (a flag's edge on its pole); `null` for
        /// none.
        #[property]
        pub fn pin_box(self) -> Option<ClothBox> {
            with_cloth(self, |d| d.pin_box)
                .flatten()
                .map(|(min, max)| ClothBox { min, max })
        }

        pub fn set_pin_box(self, b: Option<ClothBox>) {
            with_cloth(self, |d| d.pin_box = b.map(|b| (b.min, b.max)));
        }

        /// The points (object space) near which vertices stay put.
        #[property]
        pub fn pins(self) -> Vec<V3> {
            with_cloth(self, |d| d.pins.clone()).unwrap_or_default()
        }

        pub fn set_pins(self, pins: &[V3]) {
            with_cloth(self, |d| d.pins = pins.to_vec());
        }

        /// Objects with a `SphereCollider` the cloth cannot pass through.
        #[property]
        pub fn sphere_colliders(self) -> Vec<ObjectId> {
            with_cloth(self, |d| d.sphere_colliders.clone()).unwrap_or_default()
        }

        pub fn set_sphere_colliders(self, objects: &[ObjectId]) {
            with_cloth(self, |d| d.sphere_colliders = objects.to_vec());
        }

        /// Objects with a `CapsuleCollider` the cloth cannot pass through.
        #[property]
        pub fn capsule_colliders(self) -> Vec<ObjectId> {
            with_cloth(self, |d| d.capsule_colliders.clone()).unwrap_or_default()
        }

        pub fn set_capsule_colliders(self, objects: &[ObjectId]) {
            with_cloth(self, |d| d.capsule_colliders = objects.to_vec());
        }

        /// An object with a `SphereCollider` the cloth cannot pass through.
        pub fn add_sphere_collider(self, object: ObjectId) {
            with_cloth(self, |d| d.sphere_colliders.push(object));
        }

        /// An object with a `CapsuleCollider` the cloth cannot pass through.
        pub fn add_capsule_collider(self, object: ObjectId) {
            with_cloth(self, |d| d.capsule_colliders.push(object));
        }

        pub fn clear_colliders(self) {
            with_cloth(self, |d| {
                d.sphere_colliders.clear();
                d.capsule_colliders.clear();
            });
        }
""",
    ),
    dict(
        name="Material", low="material", ref="MaterialRef", store="MATERIALS", data="MaterialData", asset=True, shared=False,
        fields=[
            ("color", "color", "color", ONE, "Base colour (linear; alpha below 1 makes it see-through). For metals, the reflection's tint."),
            ("emission", "emission", "color", (0, 0, 0, 1), "Light it gives off by itself (linear, can exceed 1 for glow)."),
            ("smoothness", "smoothness", "f64", 0.5, "0 (rough, matte) to 1 (polished, mirror-like): 1 - roughness."),
            ("unlit", "unlit", "bool", False, "Ignore lights: just the colour plus emission."),
            ("mainTexture", "main_texture", "texture", 0, "Multiplies the colour; its alpha counts when `color.a` < 1."),
            ("mainTextureScale", "main_texture_scale", "v2", (1, 1), "How many times the texture repeats across the mesh's UVs."),
            ("mainTextureOffset", "main_texture_offset", "v2", (0, 0), "Where the texture starts (in its 0..1 space)."),
            ("metallic", "metallic", "f64", 0.0, "0 (plastic, stone, wood, skin) to 1 (metal)."),
            ("normalMap", "normal_map", "texture", 0, "Bumps from a tangent-space normal map (load it with `linear`)."),
            ("normalScale", "normal_scale", "f64", 1.0, "How strong the normal map's bumps are."),
            ("metallicGlossMap", "metallic_gloss_map", "texture", 0, "Metallic in red and smoothness in alpha."),
            ("hasSpecularColor", "has_specular_color", "bool", False, "The specular workflow is on."),
            ("specularColor", "specular_color", "color", (0, 0, 0, 1), "The specular workflow's reflectance head-on; `metallic` is then ignored."),
            ("occlusionMap", "occlusion_map", "texture", 0, "Ambient occlusion (green channel)."),
            ("occlusionStrength", "occlusion_strength", "f64", 1.0, ""),
            ("emissionMap", "emission_map", "texture", 0, "Multiplies `emission`."),
            ("detailAlbedoMap", "detail_albedo_map", "texture", 0, "Fine detail multiplied over the colour."),
            ("detailAlbedoMapScale", "detail_albedo_map_scale", "v2", (1, 1), ""),
            ("detailAlbedoMapOffset", "detail_albedo_map_offset", "v2", (0, 0), ""),
            ("renderingMode", "rendering_mode", "enum:opaque|cutout|fade|transparent", "opaque", "`opaque`, `cutout` (holes where alpha < `alphaCutoff`), `fade` or `transparent` (glass)."),
            ("alphaCutoff", "alpha_cutoff", "f64", 0.5, "With `cutout`: alpha below this is a hole."),
            ("doubleSided", "double_sided", "bool", False, "Draws back faces too, lit from that side."),
            ("gltfPacking", "gltf_packing", "bool", False, "glTF's packing of the metallic map (internal)."),
        ],
        skip_save=["hasSpecularColor", "specularColor"],
        save_if_changed=["normalMap", "normalScale", "metallicGlossMap", "occlusionMap", "occlusionStrength", "emissionMap", "detailAlbedoMap", "detailAlbedoMapScale", "detailAlbedoMapOffset", "renderingMode", "alphaCutoff", "doubleSided", "gltfPacking"],
        set_hook={"specularColor": "d.has_specular_color = true;"},
        extra_save='if d.has_specular_color { m.insert("specularColor".into(), json!({ "$type": "Color", "value": [d.specular_color.r, d.specular_color.g, d.specular_color.b, d.specular_color.a] })); }',
        extra_load='"specularColor" => { if v.is_null() { d.has_specular_color = false; Some(()) } else { color_of(v).map(|c| { d.has_specular_color = true; d.specular_color = c; }) } }',
        api="""
        /// Turns the specular workflow off again (`specularColor = null`).
        pub fn clear_specular_color(self) {
            with_material(self, |d| d.has_specular_color = false);
        }

        /// A copy with another colour.
        pub fn with_color(self, color: Rgba) -> MaterialRef {
            let copy = self.copy();
            with_material(copy, |d| d.color = color);
            copy
        }
""",
    ),
    dict(
        name="Sprite", low="sprite", ref="SpriteRef", store="SPRITES", data="SpriteData", asset=True, shared=False, create_name="empty",
        fields=[
            ("texture", "texture", "texture", 0, "The texture it is cut from."),
            ("hasRect", "has_rect", "bool", False, "Whether `rect` is set; otherwise the whole texture."),
            ("pivot", "pivot", "v2", (0.5, 0.5), "Where the object's origin sits (0..1 across the rect from its bottom-left)."),
            ("pixelsPerUnit", "pixels_per_unit", "f64", 100.0, "Texels that make a metre."),
        ],
        skip_save=["hasRect"],
        extra_fields=["pub rect: [f64; 4],"],
        extra_defaults=["rect: [0.0; 4],"],
        extra_save='m.insert("rect".into(), if d.has_rect { json!({ "$type": "Rect", "value": d.rect }) } else { Value::Null });',
        extra_load='"rect" => { if v.is_null() { d.has_rect = false; Some(()) } else { rect_of(v).map(|r| { d.has_rect = true; d.rect = r; }) } }',
        api="""
        /// Draws only this part of the texture: x, y, width, height in pixels from the
        /// texture's bottom-left.
        pub fn set_rect(self, x: f64, y: f64, width: f64, height: f64) {
            with_sprite(self, |d| {
                d.has_rect = true;
                d.rect = [x, y, width, height];
            });
        }

        /// A sprite of a texture: all of it, centred, `pixelsPerUnit` (default 100) texels to a metre.
        #[optional(pixels_per_unit = 100)]
        pub fn create(texture: TextureRef, pixels_per_unit: f64) -> SpriteRef {
            let s = super::create_sprite();
            with_sprite(s, |d| {
                d.texture = texture.0;
                d.pixels_per_unit = pixels_per_unit;
            });
            s
        }

        /// One cell of a sprite sheet cut into `columns` x `rows` equal cells, read left to right,
        /// top to bottom (needs the texture loaded).
        pub fn grid_cell(texture: TextureRef, columns: f64, rows: f64, index: f64) -> SpriteRef {
            let (w, h) = super::texture_size(texture);
            let (cw, ch) = (w / columns.max(1.0), h / rows.max(1.0));
            let (c, r) = (index % columns.max(1.0), (index / columns.max(1.0)).floor());
            let s = super::create_sprite();
            with_sprite(s, |d| {
                d.texture = texture.0;
                d.has_rect = true;
                d.rect = [c * cw, h - (r + 1.0) * ch, cw, ch];
            });
            s
        }

        /// Every cell of a sprite sheet cut into `columns` x `rows` cells, row by row from the
        /// top, each with this pivot and `pixelsPerUnit` when given.
        #[optional(pivot, pixels_per_unit)]
        pub fn grid(texture: TextureRef, columns: f64, rows: f64, pivot: V2, pixels_per_unit: f64) -> Vec<SpriteRef> {
            let mut cells = Vec::new();
            for r in 0..rows.max(0.0) as usize {
                for c in 0..columns.max(0.0) as usize {
                    let s = Self::grid_cell(texture, columns, rows, r as f64 * columns + c as f64);
                    with_sprite(s, |d| {
                        if !pivot.x.is_nan() {
                            d.pivot = pivot;
                        }
                        if !pixels_per_unit.is_nan() {
                            d.pixels_per_unit = pixels_per_unit;
                        }
                    });
                    cells.push(s);
                }
            }
            cells
        }

        /// Its size in world units (zero until the texture has loaded, for a whole-texture sprite).
        #[property]
        pub fn size(self) -> V2 {
            let (tw, th) = with_sprite(self, |d| super::texture_size(TextureRef(d.texture))).unwrap_or((0.0, 0.0));
            with_sprite(self, |d| {
                let (w, h) = if d.has_rect { (d.rect[2], d.rect[3]) } else { (tw, th) };
                V2 { x: w / d.pixels_per_unit, y: h / d.pixels_per_unit }
            })
            .unwrap_or(V2 { x: 0.0, y: 0.0 })
        }
""",
        extra_value_types=["v2"],
    ),
]


def gen(c):
    name, low, ref, store, data = c["name"], c["low"], c["ref"], c["store"], c["data"]
    asset = c.get("asset", False)
    fields = c["fields"]
    skip = set(c.get("skip_save", []))
    changed = set(c.get("save_if_changed", []))
    hook = c.get("set_hook", {})
    o = []
    w = o.append
    kinds = set()
    for ts, rs, t, d, doc in fields:
        kinds.add("enum" if t.startswith("enum:") else t.rstrip("?"))
    kinds |= set(c.get("extra_value_types", []))
    obj = "" if asset else "object: ObjectId"
    objarg = "" if asset else "object"

    w("")
    w("// ---- %s ----" % name)
    w("")
    w("numbered_handle!(")
    w("    /// A `%s` by number (1-based: 0 is \"none\")." % name)
    w("    %s" % ref)
    w(");")
    w("")
    w("/// The data of one `%s`." % name)
    w("#[derive(Clone, Debug, PartialEq)]")
    w("pub struct %s {" % data)
    if not asset:
        w("    pub object: ObjectId,")
    for ts, rs, t, d, doc in fields:
        key = "enum" if t.startswith("enum:") else t
        w("    pub %s: %s," % (rs, TYPES[key][0]))
    for line in c.get("extra_fields", []):
        w("    " + line)
    w("}")
    w("")
    w("impl %s {" % data)
    w("    fn new(%s) -> Self {" % obj)
    w("        %s {" % data)
    if not asset:
        w("            object,")
    for ts, rs, t, d, doc in fields:
        w("            %s: %s," % (rs, default_lit(t, d)))
    for line in c.get("extra_defaults", []):
        w("            " + line)
    w("        }")
    w("    }")
    w("}")
    w("")
    w("thread_local! {")
    w("    static %s: RefCell<Vec<Option<%s>>> = const { RefCell::new(Vec::new()) };" % (store, data))
    w("}")
    w("")
    w("/// Runs `f` on a `%s`'s data." % name)
    w("pub fn with_%s<R>(r: %s, f: impl FnOnce(&mut %s) -> R) -> Option<R> {" % (low, ref, data))
    w("    %s.with(|v| {" % store)
    w("        let mut v = v.borrow_mut();")
    w("        v.get_mut((r.0 as usize).checked_sub(1)?)?.as_mut().map(f)")
    w("    })")
    w("}")
    w("")
    w("/// The live `%ss`, in the order they were made%s." % (name, "" if asset else " (for the renderer)"))
    w("pub fn %ss() -> Vec<(%s, %s)> {" % (low, ref, data))
    w("    %s.with(|v| {" % store)
    w("        v.borrow()")
    w("            .iter()")
    w("            .enumerate()")
    w("            .filter_map(|(i, d)| d.clone().map(|d| (%s(i as u32 + 1), d)))" % ref)
    w("            .collect()")
    w("    })")
    w("}")
    w("")
    if asset:
        w("pub fn create_%s() -> %s {" % (low, ref))
        w("    %s.with(|v| {" % store)
        w("        let mut v = v.borrow_mut();")
        w("        v.push(Some(%s::new()));" % data)
        w("        %s(v.len() as u32)" % ref)
        w("    })")
        w("}")
        w("")
        w("/// Frees one (users keep their handle and see the defaults).")
        w("pub fn remove_%s(r: %s) {" % (low, ref))
    else:
        w("/// The `%ss` on an object, in the order they were added." % name)
        w("pub fn %ss_on(object: ObjectId) -> Vec<%s> {" % (low, ref))
        w("    %ss()" % low)
        w("        .into_iter()")
        w("        .filter(|(_, d)| d.object == object)")
        w("        .map(|(r, _)| r)")
        w("        .collect()")
        w("}")
        w("")
        w("pub fn attach_%s(object: ObjectId) -> %s {" % (low, ref))
        w("    let r = %s.with(|v| {" % store)
        w("        let mut v = v.borrow_mut();")
        w("        v.push(Some(%s::new(object)));" % data)
        w("        %s(v.len() as u32)" % ref)
        w("    });")
        if c.get("attach_hook"):
            w("    " + c["attach_hook"])
        w("    r")
        w("}")
        w("")
        w("pub fn remove_%s(r: %s) {" % (low, ref))
        if c.get("remove_hook"):
            w("    " + c["remove_hook"])
    w("    %s.with(|v| {" % store)
    w("        if let Some(slot) = v.borrow_mut().get_mut((r.0 as usize).wrapping_sub(1)) {")
    w("            *slot = None;")
    w("        }")
    w("    });")
    w("}")
    w("")
    if not asset:
        w("/// Removes the `%ss` of a destroyed object." % name)
        w("pub fn remove_%ss_on(object: ObjectId) {" % low)
        w("    for r in %ss_on(object) {" % low)
        w("        remove_%s(r);" % low)
        w("    }")
        w("}")
        w("")
    w("pub fn reset_%ss() {" % low)
    w("    %s.with(|v| v.borrow_mut().clear());" % store)
    w("}")
    if c.get("items"):
        w("")
        w(c["items"].rstrip())
    # saving and loading
    w("")
    w("/// The saved properties of a `%s`." % name)
    w("pub fn save_%s(r: %s) -> Map<String, Value> {" % (low, ref))
    w("    let mut m = Map::new();")
    if changed:
        w("    let default = %s::new(%s);" % (data, "" if asset else "none_object()"))
    w("    with_%s(r, |d| {" % low)
    for ts, rs, t, d, doc in fields:
        if ts in skip:
            continue
        key = "enum" if t.startswith("enum:") else t
        ins = 'm.insert("%s".into(), %s);' % (ts, TYPES[key][5].replace("d.F", "d." + rs))
        if ts in changed:
            ins = "if d.%s != default.%s { %s }" % (rs, rs, ins)
        w("        " + ins)
    if c.get("extra_save"):
        w("        " + c["extra_save"])
    w("    });")
    w("    m")
    w("}")
    w("")
    w("/// Applies saved properties to a `%s`; returns warnings." % name)
    w("pub fn load_%s(r: %s, props: &Map<String, Value>) -> Vec<String> {" % (low, ref))
    w("    let mut warnings = Vec::new();")
    w("    with_%s(r, |d| {" % low)
    w("        for (k, v) in props {")
    w("            let ok = match k.as_str() {")
    for ts, rs, t, d, doc in fields:
        if ts in skip:
            continue
        key = "enum" if t.startswith("enum:") else t
        if key == "enum":
            allowed = t.split(":")[1].split("|")
            w('                "%s" => v.as_str().filter(|s| [%s].contains(s)).map(|s| d.%s = s.to_string()),' % (ts, ", ".join('"%s"' % a for a in allowed), rs))
        else:
            w('                "%s" => %s.map(|x| d.%s = x),' % (ts, TYPES[key][6], rs))
    if c.get("extra_load"):
        w("                " + c["extra_load"] + ",")
    w("                other => {")
    w('                    warnings.push(format!("no property `{other}`; dropped"));')
    w("                    continue;")
    w("                }")
    w("            };")
    w("            if ok.is_none() {")
    w('                warnings.push(format!("property `{k}` has a value of the wrong shape"));')
    w("            }")
    w("        }")
    w("    });")
    w("    warnings")
    w("}")
    # the script API
    values = sorted(k for k in kinds if k in VALUES)
    handles = sorted(k for k in kinds if k in HANDLES and HANDLES[k].split(" ")[0] != name)
    hl = ["%s = %s" % (name, ref)]
    if not asset:
        hl.append("GameObject = ObjectId")
    hl += [HANDLES[k] for k in handles]
    hl += c.get("extra_handles", [])
    hl = list(dict.fromkeys(hl))
    attr = 'namespace = "%s", quickjs, handles(%s)' % (name, ", ".join(hl))
    if values:
        attr += ", values(%s)" % ", ".join(VALUES[k] for k in values)
    if c.get("extra_records"):
        attr += ", records(%s)" % ", ".join(c["extra_records"])
    w("")
    w("#[iron_api(%s)]" % attr)
    w("pub mod %s_api {" % low)
    w("    use super::*;")
    w("")
    w("    impl %s {" % ref)
    if asset:
        w("        /// A new %s with the defaults (`new %s({ .. })` sets properties from the object)." % (name, name))
        w("        pub fn %s() -> %s {" % (c.get("create_name", "create"), ref))
        w("            super::create_%s()" % low)
        w("        }")
        w("")
        w("        /// A copy.")
        w("        pub fn copy(self) -> %s {" % ref)
        w("            let copy = super::create_%s();" % low)
        w("            if let Some(data) = with_%s(self, |d| d.clone()) {" % low)
        w("                with_%s(copy, |d| *d = data);" % low)
        w("            }")
        w("            copy")
        w("        }")
        w("")
        w("        /// Frees it.")
        w("        pub fn destroy(self) {")
        w("            super::remove_%s(self);" % low)
        w("        }")
    else:
        w("        /// Adds a `%s` to a GameObject (what `go.addComponent(%s)` does)." % (name, name))
        w("        pub fn attach(object: ObjectId) -> %s {" % ref)
        w("            super::attach_%s(object)" % low)
        w("        }")
        w("")
        w("        /// The first `%s` on an object, or none." % name)
        w("        pub fn find_on(object: ObjectId) -> %s {" % ref)
        w("            super::%ss_on(object).first().copied().unwrap_or(%s(0))" % (low, ref))
        w("        }")
        w("")
        w("        #[property]")
        w("        pub fn game_object(self) -> ObjectId {")
        w("            with_%s(self, |d| d.object).unwrap_or_else(none_object)" % low)
        w("        }")
    for ts, rs, t, d, doc in fields:
        key = "enum" if t.startswith("enum:") else t
        sty, gty, gexp, sety, setstmt, tojson, fromjson, fallback = TYPES[key]
        rs_plain = rs[2:] if rs.startswith("r#") else rs
        w("")
        if doc:
            w("        /// %s" % doc)
        w("        #[property]")
        if key == "enum":
            allowed = t.split(":")[1].split("|")
            tsu = " | ".join('\\"%s\\"' % a for a in allowed)
            w('        #[ts_type = "%s"]' % tsu)
        elif key in NULLABLE_HANDLES:
            w("        #[nullable]")
        w("        pub fn %s(self) -> %s {" % (rs, gty))
        gx = gexp.replace("d.F", "d." + rs)
        w("            with_%s(self, |d| %s).unwrap_or_else(|| %s)" % (low, gx, fallback))
        w("        }")
        w("")
        if key == "enum":
            allowed = t.split(":")[1].split("|")
            w("        /// A name other than %s is ignored." % ", ".join("`%s`" % a for a in allowed))
            w('        #[ts_type = "%s"]' % tsu)
            w("        pub fn set_%s(self, v: &str) {" % rs_plain)
            w("            if [%s].contains(&v) {" % ", ".join('"%s"' % a for a in allowed))
            w("                with_%s(self, |d| d.%s = v.to_string());" % (low, rs))
            w("            }")
            w("        }")
        else:
            if key in NULLABLE_HANDLES:
                w("        #[nullable]")
            w("        pub fn set_%s(self, v: %s) {" % (rs_plain, sety))
            w("            with_%s(self, |d| { %s; %s });" % (low, setstmt.replace("d.F", "d." + rs), hook.get(ts, "")))
            w("        }")
    if c.get("api"):
        w(c["api"].rstrip("\n"))
    w("    }")
    w("}")
    # the kind
    w("")
    if asset:
        shared = "true" if c.get("shared", False) else "false"
        w("fn %s_asset_kind() -> AssetKind {" % low)
        w("    AssetKind {")
        w('        name: "%s".into(),' % name)
        w("        also: Vec::new(),")
        w("        shared: %s," % shared)
        w("        create: Box::new(|def| {")
        w("            let r = create_%s();" % low)
        w("            let props = def.as_object().cloned().unwrap_or_default();")
        w("            for w in load_%s(r, &props) {" % low)
        w('                eprintln!("%s: {w}");' % name)
        w("            }")
        w("            Ok(u64::from(r.0))")
        w("        }),")
        w("        describe: Box::new(|h| {")
        w("            with_%s(%s(h as u32), |_| ())?;" % (low, ref))
        w("            Some(Value::Object(save_%s(%s(h as u32))))" % (low, ref))
        w("        }),")
        w("    }")
        w("}")
    else:
        w("fn %s_kind() -> Kind {" % low)
        w("    Kind {")
        w('        name: "%s".into(),' % name)
        w("        create: Box::new(|o| Ok(u64::from(attach_%s(ObjectId::from_raw(o)).0)))," % low)
        w("        on: Box::new(|o| {")
        w("            %ss_on(ObjectId::from_raw(o))" % low)
        w("                .into_iter()")
        if c.get("kind_on_filter"):
            w("                .filter(|r| %s)" % c["kind_on_filter"])
        w("                .map(|r| u64::from(r.0))")
        w("                .collect()")
        w("        }),")
        w("        save: Box::new(|h| save_%s(%s(h as u32)))," % (low, ref))
        w("        load: Box::new(|h, props| load_%s(%s(h as u32), props))," % (low, ref))
        if c.get("awake_hook"):
            w("        awake: Box::new(|hs| {")
            w("            for h in hs {")
            w("                let r = %s(*h as u32);" % ref)
            w("                " + c["awake_hook"])
            w("            }")
            w("        }),")
        else:
            w("        awake: Box::new(|_| {}),")
        w("        object: Box::new(|h| with_%s(%s(h as u32), |d| d.object.to_raw()))," % (low, ref))
        w("        destroy: Box::new(|h| remove_%s(%s(h as u32)))," % (low, ref))
        w("    }")
        w("}")
    return o


def main():
    out = []
    out.append("// Generated by gen/gen_components.py; do not edit.")
    out.append("")
    out.append("use super::*;")
    for c in COMPONENTS:
        out.extend(gen(c))
    out.append("")
    out.append("/// Registers the scene-file kinds and asset kinds of the generated types.")
    out.append("pub fn register_generated_kinds() {")
    for c in COMPONENTS:
        if c.get("asset"):
            out.append("    iron_kinds::register_asset(%s_asset_kind());" % c["low"])
        else:
            out.append("    iron_kinds::register(%s_kind());" % c["low"])
    out.append("}")
    out.append("")
    out.append("/// The manifests of the generated types.")
    out.append("pub fn generated_manifests() -> Vec<String> {")
    out.append("    vec![")
    for c in COMPONENTS:
        out.append("        %s_api::MANIFEST.to_json()," % c["low"])
    out.append("    ]")
    out.append("}")
    out.append("")
    out.append("/// Removes the generated components of a destroyed object.")
    out.append("pub fn remove_generated_on(object: ObjectId) {")
    for c in COMPONENTS:
        if not c.get("asset"):
            out.append("    remove_%ss_on(object);" % c["low"])
    out.append("}")
    out.append("")
    out.append("/// Forgets every generated component and asset.")
    out.append("pub fn reset_generated() {")
    for c in COMPONENTS:
        out.append("    reset_%ss();" % c["low"])
    out.append("}")
    path = os.path.join(HERE, "..", "src", "generated.rs")
    with open(path, "w", encoding="utf8", newline="\n") as f:
        f.write("\n".join(out) + "\n")
    print("wrote", os.path.normpath(path))


main()
