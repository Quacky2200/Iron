//! Frustum culling: what a camera can't see isn't drawn. Meshes are tested by their
//! bounding boxes, lights by the spheres they reach.

use iron_core::glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

/// The index of the near plane in [`Frustum`]'s planes.
const NEAR: usize = 4;

/// A camera's six clip planes in world space, each `xyz · p + w >= 0` inside.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// From a view-projection with depth 0 to 1 (wgpu's), perspective or orthographic.
    pub fn from_view_proj(m: Mat4) -> Self {
        let (r0, r1, r2, r3) = (m.row(0), m.row(1), m.row(2), m.row(3));
        let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|p| {
            let len = p.xyz().length();
            if len > 0.0 { p / len } else { p }
        });
        Self { planes }
    }

    /// Whether any of the sphere is inside.
    pub fn sphere(&self, center: Vec3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p.xyz().dot(center) + p.w >= -radius)
    }

    /// Whether any of the world-space box is (conservatively) inside.
    pub fn aabb(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().all(|p| {
            // The corner farthest along the plane's normal.
            let n = p.xyz();
            let far = Vec3::select(n.cmpge(Vec3::ZERO), max, min);
            n.dot(far) + p.w >= 0.0
        })
    }

    /// [`Self::aabb`] ignoring the near plane: for a shadow map's view, where what lies between
    /// the light and its near plane still casts into it.
    pub fn aabb_past_near(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().enumerate().all(|(i, p)| {
            let n = p.xyz();
            let far = Vec3::select(n.cmpge(Vec3::ZERO), max, min);
            i == NEAR || n.dot(far) + p.w >= 0.0
        })
    }

    /// Whether a local box moved by `model` is inside.
    pub fn transformed(&self, (min, max): (Vec3, Vec3), model: Mat4) -> bool {
        let (min, max) = transform_aabb(min, max, model);
        self.aabb(min, max)
    }
}

/// How far a camera's fog reaches before hiding everything: boxes wholly past it are
/// culled. Perspective cameras measure straight from the eye, orthographic ones by depth
/// along the view, as the shader's fog does.
#[derive(Clone, Copy, Debug)]
pub struct FogReach {
    eye: Vec3,
    forward: Vec3,
    orthographic: bool,
    distance: f32,
}

impl FogReach {
    pub fn new(camera: &super::WorldCamera, distance: f32) -> Self {
        Self {
            eye: camera.position,
            forward: camera.forward,
            orthographic: camera.orthographic,
            distance,
        }
    }

    /// Whether all of the world-space box is past the fog's reach.
    pub fn hides(&self, (min, max): (Vec3, Vec3)) -> bool {
        if self.orthographic {
            // The box's nearest depth along the view.
            let centre = (min + max) * 0.5;
            let half = (max - min) * 0.5;
            let depth = (centre - self.eye).dot(self.forward) - half.dot(self.forward.abs());
            depth > self.distance
        } else {
            // The box's nearest point to the eye.
            let nearest = self.eye.clamp(min, max);
            nearest.distance_squared(self.eye) > self.distance * self.distance
        }
    }
}

/// A box around a local box after a transform (Arvo's method: centre plus the
/// absolute matrix times the half extents).
pub fn transform_aabb(min: Vec3, max: Vec3, model: Mat4) -> (Vec3, Vec3) {
    let center = model.transform_point3((min + max) * 0.5);
    let half = (max - min) * 0.5;
    let extent = model.x_axis.xyz().abs() * half.x
        + model.y_axis.xyz().abs() * half.y
        + model.z_axis.xyz().abs() * half.z;
    (center - extent, center + extent)
}

/// A box holding a skinned mesh: its bind-pose box under each bone matrix, together. Each vertex
/// is a weighted mix of its bones' images of it, so it lies inside.
pub fn skinned_aabb(min: Vec3, max: Vec3, bones: &[Mat4]) -> Option<(Vec3, Vec3)> {
    bones
        .iter()
        .map(|b| transform_aabb(min, max, *b))
        .reduce(|(lo, hi), (a, b)| (lo.min(a), hi.max(b)))
}

/// The box around a set of points (`None` when empty).
pub fn bounds(points: &[[f32; 3]]) -> Option<(Vec3, Vec3)> {
    let mut it = points.iter().map(|p| Vec3::from_array(*p));
    let first = it.next()?;
    Some(it.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

/// The unit quad quads are drawn with, as a box.
pub const QUAD_BOUNDS: (Vec3, Vec3) = (Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0));

#[cfg(test)]
mod tests {
    use super::*;

    // M5-CUL-01
    #[test]
    fn frustums_keep_what_cameras_see() {
        let view = iron_core::glam::camera::rh::view::look_at_mat4(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::Y,
        );
        let proj = iron_core::glam::camera::rh::proj::directx::perspective(
            60f32.to_radians(),
            1.0,
            0.1,
            50.0,
        );
        let f = Frustum::from_view_proj(proj * view);
        let unit = (Vec3::splat(-0.5), Vec3::splat(0.5));
        assert!(f.transformed(unit, Mat4::IDENTITY), "in front");
        assert!(
            !f.transformed(unit, Mat4::from_translation(Vec3::new(0.0, 0.0, 8.0))),
            "behind"
        );
        assert!(
            !f.transformed(unit, Mat4::from_translation(Vec3::new(20.0, 0.0, 0.0))),
            "left of view"
        );
        assert!(
            !f.transformed(unit, Mat4::from_translation(Vec3::new(0.0, 0.0, -60.0))),
            "past far"
        );
        // A big box reaching into view from the side counts.
        assert!(f.transformed(
            unit,
            Mat4::from_scale_rotation_translation(
                Vec3::new(30.0, 1.0, 1.0),
                Default::default(),
                Vec3::new(18.0, 0.0, 0.0)
            )
        ));
        // A light's sphere: out of view, unless its range reaches in.
        assert!(!f.sphere(Vec3::new(12.0, 0.0, 0.0), 2.0));
        assert!(f.sphere(Vec3::new(12.0, 0.0, 0.0), 10.0));
        // Orthographic too.
        let ortho = iron_core::glam::camera::rh::proj::directx::orthographic(
            -2.0, 2.0, -2.0, 2.0, 0.1, 50.0,
        );
        let f = Frustum::from_view_proj(ortho * view);
        assert!(f.transformed(unit, Mat4::from_translation(Vec3::new(1.8, 0.0, -30.0))));
        assert!(!f.transformed(unit, Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0))));
    }

    // M7-FOG-07
    #[test]
    fn fog_reach_hides_boxes_wholly_past_it() {
        let cam = |orthographic: bool| super::super::WorldCamera {
            view_proj: Mat4::IDENTITY,
            position: Vec3::ZERO,
            forward: Vec3::NEG_Z,
            orthographic,
            clear: crate::Color::rgb(0.0, 0.0, 0.0),
        };
        let unit = |at: Vec3| (at - Vec3::splat(0.5), at + Vec3::splat(0.5));
        let r = FogReach::new(&cam(false), 10.0);
        assert!(!r.hides(unit(Vec3::new(0.0, 0.0, -9.0))), "inside");
        assert!(!r.hides(unit(Vec3::new(0.0, 0.0, -10.4))), "straddling");
        assert!(r.hides(unit(Vec3::new(0.0, 0.0, -11.0))), "past");
        // Perspective goes by distance: off to the side counts too.
        assert!(r.hides(unit(Vec3::new(9.0, 0.0, -6.0))));
        // Orthographic goes by depth: off to the side doesn't.
        let o = FogReach::new(&cam(true), 10.0);
        assert!(!o.hides(unit(Vec3::new(9.0, 0.0, -6.0))));
        assert!(o.hides(unit(Vec3::new(0.0, 0.0, -11.0))));
    }

    #[test]
    fn rotated_boxes_grow_to_hold_their_corners() {
        let r = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let (lo, hi) = transform_aabb(Vec3::splat(-1.0), Vec3::splat(1.0), r);
        assert!((hi.x - 2f32.sqrt()).abs() < 1e-5 && (lo.y + 2f32.sqrt()).abs() < 1e-5);
        assert_eq!(
            bounds(&[[1.0, -2.0, 0.0], [-1.0, 3.0, 2.0]]),
            Some((Vec3::new(-1.0, -2.0, 0.0), Vec3::new(1.0, 3.0, 2.0)))
        );
        assert_eq!(bounds(&[]), None);
    }

    // M5-CUL-03
    #[test]
    fn a_skinned_box_holds_every_bones_image_and_shadow_volumes_reach_back_past_near() {
        let bones = [
            Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)),
            Mat4::from_scale(Vec3::splat(2.0)),
        ];
        let (lo, hi) = skinned_aabb(Vec3::splat(-1.0), Vec3::splat(1.0), &bones).unwrap();
        assert_eq!((lo, hi), (Vec3::splat(-2.0), Vec3::new(6.0, 2.0, 2.0)));
        assert_eq!(skinned_aabb(Vec3::ZERO, Vec3::ONE, &[]), None);
        // An orthographic light 1 to 10 m down -z: a box behind its near plane still casts.
        let light = iron_core::glam::camera::rh::proj::directx::orthographic(
            -1.0, 1.0, -1.0, 1.0, 1.0, 10.0,
        );
        let f = Frustum::from_view_proj(light);
        let behind = (Vec3::new(-0.5, -0.5, 1.0), Vec3::new(0.5, 0.5, 2.0));
        assert!(!f.aabb(behind.0, behind.1) && f.aabb_past_near(behind.0, behind.1));
        let aside = (Vec3::new(3.0, 0.0, -5.0), Vec3::new(4.0, 1.0, -4.0));
        assert!(!f.aabb_past_near(aside.0, aside.1));
        let beyond = (Vec3::new(0.0, 0.0, -20.0), Vec3::new(0.5, 0.5, -15.0));
        assert!(!f.aabb_past_near(beyond.0, beyond.1));
    }
}
