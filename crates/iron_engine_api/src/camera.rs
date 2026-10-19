//! `Camera`: a component on a GameObject. Scripts make one with `Camera.attach(go)` (what
//! `go.addComponent(Camera, { .. })` compiles to) and set its properties; the host reads the cameras
//! with [`cameras`] when it draws.

use crate::color::Rgba;
use iron_api::{Handle, iron_api};
use iron_scene::ObjectId;
use std::cell::RefCell;

/// A camera by number (1-based: 0 is "no camera" in a script's `Camera | null`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraRef(pub u32);

impl Handle for CameraRef {
    fn to_raw(self) -> u64 {
        u64::from(self.0)
    }

    fn from_raw(raw: u64) -> Self {
        CameraRef(raw as u32)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraData {
    pub object: ObjectId,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub background: Rgba,
    pub depth: f32,
    pub orthographic: bool,
    pub orthographic_size: f32,
}

thread_local! {
    static CAMERAS: RefCell<Vec<Option<CameraData>>> = const { RefCell::new(Vec::new()) };
}

/// The live cameras, in the order they were made.
pub fn cameras() -> Vec<(CameraRef, CameraData)> {
    CAMERAS.with(|c| {
        c.borrow()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| d.clone().map(|d| (CameraRef(i as u32 + 1), d)))
            .collect()
    })
}

/// The first camera on an object.
pub fn camera_on(object: ObjectId) -> Option<CameraRef> {
    cameras()
        .into_iter()
        .find(|(_, d)| d.object == object)
        .map(|(c, _)| c)
}

/// Runs `f` on a camera's data (what animation writes through).
pub fn with_camera<R>(c: CameraRef, f: impl FnOnce(&mut CameraData) -> R) -> Option<R> {
    with(c, f)
}

/// Removes the cameras of destroyed GameObjects.
pub fn remove_on(object: ObjectId) {
    CAMERAS.with(|c| {
        for slot in c.borrow_mut().iter_mut() {
            if slot.as_ref().is_some_and(|d| d.object == object) {
                *slot = None;
            }
        }
    });
}

pub fn reset() {
    CAMERAS.with(|c| c.borrow_mut().clear());
}

fn with<R>(c: CameraRef, f: impl FnOnce(&mut CameraData) -> R) -> Option<R> {
    CAMERAS.with(|v| {
        let mut v = v.borrow_mut();
        v.get_mut((c.0 as usize).checked_sub(1)?)?.as_mut().map(f)
    })
}

#[iron_api(
    namespace = "Cameras",
    handles(GameObject = ObjectId, Camera = CameraRef),
    values(Color = Rgba [r: f64, g: f64, b: f64, a: f64])
)]
pub mod camera_api {
    use super::{CAMERAS, CameraData, CameraRef, with};
    use crate::color::Rgba;
    use iron_scene::ObjectId;

    /// How many cameras exist.
    pub fn count() -> u32 {
        CAMERAS.with(|c| c.borrow().iter().flatten().count() as u32)
    }

    impl CameraRef {
        /// Adds a camera to a GameObject (what `go.addComponent(Camera)` does).
        pub fn attach(object: ObjectId) -> CameraRef {
            CAMERAS.with(|c| {
                let mut c = c.borrow_mut();
                c.push(Some(CameraData {
                    object,
                    fov: 60.0,
                    near: 0.1,
                    far: 1000.0,
                    background: Rgba {
                        r: 0.05,
                        g: 0.06,
                        b: 0.09,
                        a: 1.0,
                    },
                    depth: 0.0,
                    orthographic: false,
                    orthographic_size: 5.0,
                }));
                CameraRef(c.len() as u32)
            })
        }

        #[property]
        pub fn game_object(self) -> ObjectId {
            with(self, |d| d.object).unwrap_or(ObjectId {
                index: u32::MAX,
                generation: u32::MAX,
            })
        }

        /// Vertical field of view in degrees (perspective), 1 to 179.
        #[property]
        pub fn field_of_view(self) -> f32 {
            with(self, |d| d.fov).unwrap_or(60.0)
        }

        pub fn set_field_of_view(self, v: f32) {
            with(self, |d| d.fov = v.clamp(1.0, 179.0));
        }

        #[property]
        pub fn near_clip_plane(self) -> f32 {
            with(self, |d| d.near).unwrap_or(0.1)
        }

        pub fn set_near_clip_plane(self, v: f32) {
            with(self, |d| d.near = v);
        }

        #[property]
        pub fn far_clip_plane(self) -> f32 {
            with(self, |d| d.far).unwrap_or(1000.0)
        }

        pub fn set_far_clip_plane(self, v: f32) {
            with(self, |d| d.far = v);
        }

        #[property]
        pub fn background_color(self) -> Rgba {
            with(self, |d| d.background).unwrap_or(Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            })
        }

        pub fn set_background_color(self, c: Rgba) {
            with(self, |d| d.background = c);
        }

        /// Drawing order: the camera with the higher depth draws over the lower.
        #[property]
        pub fn depth(self) -> f32 {
            with(self, |d| d.depth).unwrap_or(0.0)
        }

        pub fn set_depth(self, v: f32) {
            with(self, |d| d.depth = v);
        }

        #[property]
        pub fn orthographic(self) -> bool {
            with(self, |d| d.orthographic).unwrap_or(false)
        }

        pub fn set_orthographic(self, on: bool) {
            with(self, |d| d.orthographic = on);
        }

        /// Half the height of the view in world units (orthographic).
        #[property]
        pub fn orthographic_size(self) -> f32 {
            with(self, |d| d.orthographic_size).unwrap_or(5.0)
        }

        pub fn set_orthographic_size(self, v: f32) {
            with(self, |d| d.orthographic_size = v);
        }
    }
}
