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

/// A texture by number (1-based: 0 is "no texture") as the camera API sees it (`targetTexture`);
/// `iron_render_api` owns the textures and has its own type for the same numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureRef(pub u32);

impl Handle for TextureRef {
    fn to_raw(self) -> u64 {
        u64::from(self.0)
    }

    fn from_raw(raw: u64) -> Self {
        TextureRef(raw as u32)
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
    /// `solidColor` (the background colour) or `skybox`.
    pub clear_flags: String,
    /// The part of the screen it draws into: x, y, width, height from the top-left, 0 to 1.
    pub rect: [f64; 4],
    /// The layers it draws, a bit each.
    pub culling_mask: u32,
    /// Per layer, how far away things are still drawn (0: out to the far plane).
    pub layer_cull_distances: [f64; 32],
    pub layer_cull_spherical: bool,
    /// `none` or `fxaa`.
    pub antialiasing: String,
    /// Draws into this texture instead of the screen (0: the screen).
    pub target_texture: TextureRef,
    /// The editor's ground grid.
    pub scene_grid: bool,
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

/// Removes one camera.
pub fn remove(c: CameraRef) {
    CAMERAS.with(|v| {
        if let Some(slot) = v.borrow_mut().get_mut((c.0 as usize).wrapping_sub(1)) {
            *slot = None;
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
    quickjs,
    handles(GameObject = ObjectId, Camera = CameraRef, Texture = TextureRef),
    values(Color = Rgba [r: f64, g: f64, b: f64, a: f64])
)]
pub mod camera_api {
    use super::{CAMERAS, CameraData, CameraRef, TextureRef, with};
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
                    clear_flags: "solidColor".to_string(),
                    rect: [0.0, 0.0, 1.0, 1.0],
                    culling_mask: u32::MAX,
                    layer_cull_distances: [0.0; 32],
                    layer_cull_spherical: false,
                    antialiasing: "none".to_string(),
                    target_texture: TextureRef(0),
                    scene_grid: false,
                }));
                CameraRef(c.len() as u32)
            })
        }

        /// The main camera (`Camera.main`): the first active one that draws to the screen; none
        /// without one.
        pub fn main() -> CameraRef {
            super::cameras()
                .into_iter()
                .find(|(_, d)| {
                    d.target_texture.0 == 0
                        && iron_scene::api::with_scene(|s| s.active(d.object)).unwrap_or(true)
                })
                .map_or(CameraRef(0), |(c, _)| c)
        }

        /// The camera on an object, or none (what `go.getComponent(Camera)` does).
        pub fn find_on(object: ObjectId) -> CameraRef {
            super::camera_on(object).unwrap_or(CameraRef(0))
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

        /// What is behind everything: `solidColor` (the background colour) or `skybox`
        /// (`RenderSettings.skybox`).
        #[property]
        pub fn clear_flags(self) -> String {
            with(self, |d| d.clear_flags.clone()).unwrap_or_default()
        }

        /// A name other than `solidColor` and `skybox` is ignored.
        pub fn set_clear_flags(self, v: &str) {
            if v == "solidColor" || v == "skybox" {
                with(self, |d| d.clear_flags = v.to_string());
            }
        }

        /// The part of the screen it draws into, from the top-left, 0 to 1.
        pub fn set_rect(self, x: f64, y: f64, width: f64, height: f64) {
            with(self, |d| d.rect = [x, y, width, height]);
        }

        #[property]
        pub fn rect_x(self) -> f64 {
            with(self, |d| d.rect[0]).unwrap_or(0.0)
        }

        #[property]
        pub fn rect_y(self) -> f64 {
            with(self, |d| d.rect[1]).unwrap_or(0.0)
        }

        #[property]
        pub fn rect_width(self) -> f64 {
            with(self, |d| d.rect[2]).unwrap_or(1.0)
        }

        #[property]
        pub fn rect_height(self) -> f64 {
            with(self, |d| d.rect[3]).unwrap_or(1.0)
        }

        /// The layers it draws, a bit per layer (all by default). Signed masks work too.
        #[property]
        pub fn culling_mask(self) -> f64 {
            with(self, |d| f64::from(d.culling_mask)).unwrap_or(0.0)
        }

        pub fn set_culling_mask(self, mask: f64) {
            with(self, |d| d.culling_mask = mask as i64 as u32);
        }

        /// How far away things on a layer are still drawn (0: out to `farClipPlane`).
        pub fn layer_cull_distance(self, layer: f64) -> f64 {
            with(self, |d| {
                d.layer_cull_distances.get(layer as usize).copied()
            })
            .flatten()
            .unwrap_or(0.0)
        }

        pub fn set_layer_cull_distance(self, layer: f64, distance: f64) {
            with(self, |d| {
                if let Some(x) = d.layer_cull_distances.get_mut(layer as usize) {
                    *x = distance.max(0.0);
                }
            });
        }

        /// Measure cull distances straight from the camera rather than along its view.
        #[property]
        pub fn layer_cull_spherical(self) -> bool {
            with(self, |d| d.layer_cull_spherical).unwrap_or(false)
        }

        pub fn set_layer_cull_spherical(self, on: bool) {
            with(self, |d| d.layer_cull_spherical = on);
        }

        /// Post anti-aliasing: `none` or `fxaa`.
        #[property]
        pub fn antialiasing(self) -> String {
            with(self, |d| d.antialiasing.clone()).unwrap_or_default()
        }

        pub fn set_antialiasing(self, v: &str) {
            if v == "none" || v == "fxaa" {
                with(self, |d| d.antialiasing = v.to_string());
            }
        }

        /// Draws into this texture instead of the screen (none: the screen).
        #[property]
        pub fn target_texture(self) -> TextureRef {
            with(self, |d| d.target_texture).unwrap_or(TextureRef(0))
        }

        pub fn set_target_texture(self, t: TextureRef) {
            with(self, |d| d.target_texture = t);
        }

        /// Draws the editor's ground grid on y = 0 (not saved with scenes).
        #[property]
        pub fn scene_grid(self) -> bool {
            with(self, |d| d.scene_grid).unwrap_or(false)
        }

        pub fn set_scene_grid(self, on: bool) {
            with(self, |d| d.scene_grid = on);
        }
    }
}
