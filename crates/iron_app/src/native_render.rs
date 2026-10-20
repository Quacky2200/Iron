//! The app's side of rendering a compiled game: what [`render_sync::sync`] cannot do on its own
//! because it lives in the app. World panels (each has a UI tree, a native `Runtime` and a
//! texture), cloth, the fonts and stylesheets scripts add (`UiResources`), the cursor and the
//! window (`Cursor`, `Screen`), the quality settings from `package.json`, and what the world
//! renderer reports back (particles).

use super::*;
use iron_core::Value;
use iron_ui_native::{RootId, Runtime};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// The first node id the native reconciler hands out (the same as the screen's runtime).
const FIRST_NODE: u32 = 1 << 30;

#[derive(Default)]
struct State {
    /// One reconciler per world panel (a runtime serves one tree).
    runtimes: HashMap<u32, (Runtime, RootId)>,
    /// Panels a `WorldPanel` component exists for (the others only had a mount so far).
    live: HashSet<u32>,
    /// The `Screen` size last told to the game, and the display state last pushed.
    screen: Option<(u32, u32, u64)>,
    display: Option<String>,
    cursor_version: Option<u64>,
    root: Option<std::path::PathBuf>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn num(n: f64) -> Value {
    Value::Number(n)
}

/// The game's quality levels (`package.json`), the player's choice and the layer names: before
/// the program runs, because its top level reads them. Again after a reload.
pub fn configure_engine(root: &std::path::Path) {
    st(|s| s.root = Some(root.to_path_buf()));
    let manifest = info::Manifest::load(root);
    iron_engine_api::screen::configure_quality(&manifest.quality);
    iron_engine_api::screen::apply_default_quality();
    // `iron.layers`: names of the layers above the built-in ones.
    if let Some(layers) = manifest.layers.as_object() {
        iron_engine_api::layers::set_game_layers(
            layers
                .iter()
                .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_str()?.to_string()))),
        );
    }
}

/// Configures again after a reload reset the engine's state (before the new program runs).
pub fn reconfigure() {
    if let Some(root) = st(|s| s.root.clone()) {
        configure_engine(&root);
    }
}

/// Particle counts the world renderer reports (`[id, count, alive, ...]`): `onStopped` runs and
/// `stopAction: "destroy"` applies.
pub fn report_particles(changed: &[f64], scene: &mut iron_scene::Scene) {
    if changed.is_empty() {
        return;
    }
    let _scope = iron_scene::api::enter(scene);
    for c in changed.as_chunks::<3>().0 {
        iron_render_api::report_particles(
            iron_render_api::ParticleSystemRef(c[0] as u32),
            c[1] as u32,
            c[2] != 0.0,
        );
    }
}

impl App {
    /// Called before drawing each frame of a compiled game.
    pub(super) fn sync_native_render(&mut self) {
        {
            // `CameraFollow` and `CameraOrbit` move their cameras after every `onLateUpdate`.
            let _scope = iron_scene::api::enter(&mut self.scene.scene);
            let dt = iron_engine_api::time::state().delta_time;
            // `Animation` and `Animator` components pose their objects (events are not
            // delivered to components yet).
            iron_engine_api::animation::update(dt, None);
            iron_render_api::late_update_camera_follows(dt);
            iron_render_api::late_update_camera_orbits(dt);
        }
        let extras = render_sync::sync(&mut self.world_view, &mut self.scene, &self.asset_root);
        self.apply_ui_resources();
        self.apply_cursor_and_screen();
        for (name, args) in extras.cloths {
            if let Err(e) = self
                .cloth
                .apply(&name, &args, &self.scene, &mut self.world_view)
            {
                tracing::warn!("cloth.{name}: {e}");
            }
        }
        self.sync_panels(extras.panels);
    }

    /// A compiled game reloaded: its registries start over, and so does what hangs on them.
    pub(super) fn reset_native_render(&mut self) {
        render_sync::reset();
        st(|s| *s = State::default());
        self.panels = panels::Panels::default();
        self.cloth = cloth_sync::ClothSystem::default();
        self.world_view = world_view::WorldViews::default();
        (self.pointer_on, self.keys_on) = (Surface::Screen, Surface::Screen);
    }

    /// `addStyleSheet`, `removeStyleSheet` and `loadFont` from the game, on the screen and every
    /// panel.
    fn apply_ui_resources(&mut self) {
        use iron_engine_api::ui_resources::UiRequest;
        for request in iron_engine_api::ui_resources::take_requests() {
            match request {
                UiRequest::AddStyleSheet { id, css } => {
                    self.stylesheet_op("addStyleSheet", &[num(f64::from(id)), Value::String(css)]);
                }
                UiRequest::RemoveStyleSheet { id } => {
                    self.stylesheet_op("removeStyleSheet", &[num(f64::from(id))]);
                }
                UiRequest::LoadFont {
                    family,
                    src,
                    weight,
                    style,
                } => {
                    self.load_font_op(&[
                        Value::String(family),
                        Value::String(src),
                        if weight > 0.0 {
                            num(weight)
                        } else {
                            Value::Null
                        },
                        if style.is_empty() {
                            Value::Null
                        } else {
                            Value::String(style)
                        },
                    ]);
                }
            }
        }
    }

    /// `Cursor` and `Screen`: what the game asked for, and what the window is now.
    fn apply_cursor_and_screen(&mut self) {
        let version = iron_engine_api::cursor::version();
        if st(|s| s.cursor_version.replace(version)) != Some(version) {
            let c = iron_engine_api::cursor::state();
            self.cursor_op("visible", &[Value::Bool(c.visible)]);
            self.cursor_op("lock", &[Value::String(c.lock_state.clone())]);
            match c.image {
                Some((src, hotspot)) => {
                    let mut args = vec![Value::String(src)];
                    if let Some(h) = hotspot {
                        args.push(Value::Array(vec![num(h[0]), num(h[1])]));
                    }
                    self.cursor_op("image", &args);
                }
                None => self.cursor_op("image", &[Value::Null]),
            }
        }
        for r in iron_engine_api::screen::take_requests() {
            self.screen_op(&[
                Value::String(r.mode),
                num(f64::from(r.width)),
                num(f64::from(r.height)),
                num(r.refresh_rate),
            ]);
        }
        // What the window is now, for `Screen.resolutions`, `windowSize`, ...
        let window = self.gfx.as_ref().map(|g| g.window.clone());
        let state = display::state(window.as_deref(), &self.display);
        let text = state.to_string();
        if st(|s| s.display.replace(text.clone())).as_deref() != Some(&text) {
            let res = |v: &serde_json::Value| iron_engine_api::screen::Resolution {
                width: v["width"].as_u64().unwrap_or(0) as u32,
                height: v["height"].as_u64().unwrap_or(0) as u32,
                refresh_rate: v["refreshRate"].as_f64().unwrap_or(0.0),
            };
            let resolutions = state["resolutions"]
                .as_array()
                .map(|a| a.iter().map(res).collect())
                .unwrap_or_default();
            let modes = state["supportedModes"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            iron_engine_api::screen::set_display(
                state["mode"].as_str().unwrap_or("Windowed"),
                (
                    state["windowWidth"].as_u64().unwrap_or(1280) as u32,
                    state["windowHeight"].as_u64().unwrap_or(720) as u32,
                ),
                resolutions,
                res(&state["currentResolution"]),
                modes,
            );
        }
        // The size and scale `Screen` reports follow the window: `onChange` listeners run.
        let (w, h, scale) = self.screen;
        let key = (
            w.round() as u32,
            h.round() as u32,
            f64::from(scale).to_bits(),
        );
        if w > 0.0 && st(|s| s.screen.replace(key)) != Some(key) {
            let _scope = iron_scene::api::enter(&mut self.scene.scene);
            iron_engine_api::screen::update(key.0, key.1, f64::from(scale));
        }
    }

    /// World panels: the `WorldPanel` components' settings, new roots of `createRoot(panel)`,
    /// and every panel's pending state updates.
    fn sync_panels(&mut self, specs: Vec<Vec<Value>>) {
        let (root, fonts, sheets) = (&self.asset_root, &self.font_sources, &self.sheet_sources);
        let mut ids = HashSet::new();
        for args in specs {
            if let Some(id) = args.first().and_then(Value::as_f64) {
                ids.insert(id as u32);
            }
            if let Err(e) = self
                .panels
                .apply(&args, || new_panel_tree(root, fonts, sheets))
            {
                tracing::warn!("world panel: {e}");
            }
        }
        // A panel whose component is gone goes with its tree.
        let gone: Vec<u32> = st(|s| {
            let gone = s
                .live
                .iter()
                .filter(|id| !ids.contains(id))
                .copied()
                .collect();
            s.live = ids;
            gone
        });
        for id in gone {
            self.panels.remove(id);
            st(|s| s.runtimes.remove(&id));
        }
        let Some(native) = self.native.as_mut() else {
            return;
        };
        let mounts = native.take_panel_mounts();
        let scene = &mut self.scene.scene;
        let _scope = iron_scene::api::enter(scene);
        for (target, element) in mounts {
            let id = target as u32;
            let (root, fonts, sheets) = (&self.asset_root, &self.font_sources, &self.sheet_sources);
            let panel = self
                .panels
                .get_or_create(id, || new_panel_tree(root, fonts, sheets));
            let errors = st(|s| {
                let (rt, rid) = s.runtimes.entry(id).or_insert_with(|| {
                    let mut rt = Runtime::new(FIRST_NODE);
                    let rid = rt.create_root(iron_ui::ROOT);
                    (rt, rid)
                });
                let rid = *rid;
                rt.render(rid, element, &mut panel.tree)
            });
            for e in errors {
                tracing::warn!(target: "ui", "{e}");
            }
        }
        // State set by timers and effects since the last frame.
        let ids: Vec<u32> = st(|s| s.runtimes.keys().copied().collect());
        for id in ids {
            let Some(panel) = self.panels.map.get_mut(&id) else {
                continue;
            };
            let errors = st(|s| {
                s.runtimes
                    .get_mut(&id)
                    .map(|(rt, _)| rt.flush(&mut panel.tree))
                    .unwrap_or_default()
            });
            for e in errors {
                tracing::warn!(target: "ui", "{e}");
            }
        }
    }
}

/// A UI event of a world panel through the handlers of its reconciler, while the panel's
/// interaction layer handles the input; says whether a handler prevented the default action.
/// Follow with [`panel_settle`].
pub(super) fn panel_event(scene: &mut iron_scene::Scene, id: u32, e: &iron_ui::UiEvent) -> bool {
    let Some(mut ev) = native::to_event(e) else {
        return false;
    };
    let _scope = iron_scene::api::enter(scene);
    st(|s| {
        s.runtimes
            .get_mut(&id)
            .is_some_and(|(rt, _)| rt.dispatch_handlers(&mut ev))
    })
}

/// Commits what the handlers of a panel's events changed.
pub(super) fn panel_settle(id: u32, tree: &mut UiTree) {
    let errors = st(|s| {
        s.runtimes
            .get_mut(&id)
            .map(|(rt, _)| rt.settle(tree))
            .unwrap_or_default()
    });
    for e in errors {
        tracing::warn!(target: "ui", "{e}");
    }
}
