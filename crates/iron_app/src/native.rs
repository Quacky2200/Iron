//! The native backend: a game compiled by `iron_ts` (TypeScript to machine code) and run in this
//! process, instead of a QuickJS bundle.
//!
//! A native game is `dist/game.irbundle` (JSON written by `iron-build`: the bundled TypeScript
//! source plus the stylesheet and asset list) or a bare `.ts` / `.tsx` file. [`Native::load`]
//! JIT-compiles the source with the manifests of every engine namespace, runs the program's top
//! level (`register(..)`, `UiRoot.mount(..)`) and spawns the starting components. After that the
//! app calls the `Native` methods where it called QuickJS before:
//!
//! | App event | Native call |
//! |---|---|
//! | fixed tick | [`Native::fixed_tick`]: `onFixedUpdate`, physics step, contact hooks |
//! | frame | [`Native::frame`]: `Time`, timers and microtasks, `onUpdate` / `onLateUpdate`, UI state |
//! | before drawing | [`Native::pre_render`] |
//! | UI events | [`Native::ui_events`] (pointer, keys, focus) to the `Runtime` |
//! | hot reload | [`Native::reload`] |
//!
//! Every call runs with the app's scene made current (`iron_scene::api::enter`), because compiled
//! code reaches the scene through a per-thread pointer, not through an argument.
//!
//! UI events run through the handlers while the interaction layer handles the input, so
//! `preventDefault()` stops default actions as in the QuickJS path.

use anyhow::{Context, Result, anyhow, bail};
use iron_scene::Scene;
use iron_ts::jit::{Game, JitOptions, compile_and_load};
use iron_ts_rt::{NativeHost, ts_rt_microtasks_run, ts_rt_timers_advance};
use iron_ui::{EventKind, UiEvent, UiTree};
use iron_ui_native::{Event, RootId, Runtime};
use std::path::Path;

/// The first node id the native reconciler hands out; below it are the ids of React's side.
const FIRST_NODE: u32 = 1 << 30;

/// What `iron-build` wrote (`dist/game.irbundle`), or a bare source file.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // css, assets and title are for the asset and window setup that follows
pub struct Bundle {
    /// The program: one TypeScript / TSX source text.
    pub source: String,
    /// The bundled stylesheet, relative to `dist/`, if any.
    pub css: Option<String>,
    /// Assets the game names, relative to the game folder.
    pub assets: Vec<String>,
    pub title: Option<String>,
}

impl Bundle {
    pub fn read(path: &Path) -> Result<Bundle> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if path.extension().is_some_and(|e| e == "irbundle") {
            Bundle::parse(&text).with_context(|| path.display().to_string())
        } else {
            Ok(Bundle {
                source: text,
                ..Default::default()
            })
        }
    }

    pub fn parse(json: &str) -> Result<Bundle> {
        let v: serde_json::Value = serde_json::from_str(json).context("not JSON")?;
        let format = v["format"].as_u64().unwrap_or(0);
        if format != 1 {
            bail!("unknown bundle format {format}");
        }
        let text = |k: &str| v[k].as_str().map(str::to_string);
        Ok(Bundle {
            source: text("source").ok_or_else(|| anyhow!("the bundle has no `source`"))?,
            css: text("css"),
            assets: v["assets"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            title: text("title"),
        })
    }
}

/// Is this path a native bundle (rather than a QuickJS `game.js`)?
pub fn is_native(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "irbundle" || e == "ts" || e == "tsx")
}

/// What `iron-build dev` wrote to `dist/.iron-reload` for a failed build (`{"seq":N,"ok":false,
/// "error":".."}`); `None` when the last build worked or there is no such file.
pub fn failed_build(bundle: &Path) -> Option<String> {
    let text = std::fs::read_to_string(bundle.with_file_name(".iron-reload")).ok()?;
    parse_reload_note(&text)
}

fn parse_reload_note(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    (v["ok"] == serde_json::Value::Bool(false))
        .then(|| v["error"].as_str().unwrap_or("unknown error").to_string())
}

/// The manifests a game is compiled against: every engine namespace and the UI bridge.
fn manifests() -> Vec<String> {
    let mut all = iron_native::manifests();
    all.push(iron_ts_ui::manifest_json());
    all.push(iron_ts_ui::root_manifest_json());
    all
}

extern "C" fn console_hook(level: i32, text: *const u8, len: usize) {
    // SAFETY: the runtime passes a live UTF-8 buffer of `len` bytes.
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(text, len)) };
    match level {
        3 => tracing::warn!(target: "script", "{s}"),
        4 => tracing::error!(target: "script", "{s}"),
        2 => tracing::debug!(target: "script", "{s}"),
        _ => tracing::info!(target: "script", "{s}"),
    }
}

/// Registers the scene-file kinds of every engine crate, once per thread.
fn register_kinds() {
    iron_scenes::register_engine_kinds();
    iron_render_api::register_kinds();
    iron_engine_api::audio_components::register_kinds();
    iron_physics_api::register_kinds();
}

/// A compiled game in the running app.
pub struct Native {
    game: Game,
    host: NativeHost,
    rt: Runtime,
    screen_root: RootId,
    /// How many games this app compiled (struct headers of each get their own range).
    generation: u32,
    /// Components whose hook list changed in a reload: they start over, the others keep state.
    remount: Option<Vec<u32>>,
    /// `createRoot(panel).render(..)` calls: the panel's handle and its root element, for the
    /// app (each panel has its own tree and runtime).
    panel_mounts: Vec<(u64, iron_ui_native::Element)>,
    /// `createRoot(win).render(..)` calls for windows other than the main one: the window's handle
    /// and its root element, for the app (each window has its own tree and runtime).
    window_mounts: Vec<(u64, iron_ui_native::Element)>,
    /// Windows without a key that the last reload asks the app to close.
    reload_closes: Vec<u32>,
    /// Sets the engine state the program reads at start (`Application`, quality levels, layers):
    /// runs before the program's top level, at load and again after a reload's resets.
    env: Box<dyn Fn()>,
    /// Classes registered with `{ spawn: false }`: no starting object.
    no_spawn: Vec<String>,
    /// The game folder: scene files the program loads come from here.
    root: Option<std::path::PathBuf>,
    #[allow(dead_code)]
    pub bundle: Bundle,
}

fn active(o: u64) -> bool {
    use iron_api::Handle;
    iron_scene::api::scene_api::active(iron_scene::ObjectId::from_raw(o))
}

fn log_faults(faults: Vec<iron_ts_rt::Fault>) {
    for f in faults {
        tracing::error!(target: "script", "{}.{} threw; the component is disabled", f.class, f.hook);
    }
}

impl Native {
    /// Compiles and starts the game. The starting components are one GameObject per registered
    /// class, named after it.
    #[cfg(test)]
    pub fn load(bundle: Bundle, scene: &mut Scene, ui: &mut UiTree) -> Result<Native> {
        Self::load_with(bundle, scene, ui, Box::new(|| {}))
    }

    /// [`Native::load`] with `env` run before the program's top level (and before it again after
    /// every reload).
    pub fn load_with(
        bundle: Bundle,
        scene: &mut Scene,
        ui: &mut UiTree,
        env: Box<dyn Fn()>,
    ) -> Result<Native> {
        let generation = 0;
        iron_ts_rt::ts_rt_set_console(Some(console_hook));
        // Timers and promise jobs keep the event domain (window family) that made them.
        iron_ts_rt::ts_rt_set_domain_hooks(
            Some(iron_engine_api::window::runtime_current_domain),
            Some(iron_engine_api::window::runtime_enter_domain),
            Some(iron_engine_api::window::runtime_leave_domain),
        );
        register_kinds();
        let game = Self::compile(&bundle, generation)?;
        let mut rt = Runtime::new(FIRST_NODE);
        let screen_root = rt.create_root(iron_ui::ROOT);
        env();
        let host = {
            let _scope = iron_scene::api::enter(scene);
            game.init().map_err(|e| anyhow!("{e}"))?;
            NativeHost::new(game.classes().map_err(|e| anyhow!("{e}"))?)
        };
        let mut n = Native {
            game,
            host,
            rt,
            screen_root,
            generation,
            remount: None,
            panel_mounts: Vec::new(),
            window_mounts: Vec::new(),
            reload_closes: Vec::new(),
            env,
            root: None,
            no_spawn: no_spawn_classes(&bundle.source),
            bundle,
        };
        n.spawn_starting_components(scene);
        n.after_script(scene, ui);
        tracing::info!(classes = ?n.host.class_names(), "compiled and started the game in {:.0} ms ({:.0} lower, {:.0} codegen)",
            (n.game.lower_time + n.game.codegen_time).as_secs_f64() * 1000.0,
            n.game.lower_time.as_secs_f64() * 1000.0,
            n.game.codegen_time.as_secs_f64() * 1000.0);
        Ok(n)
    }

    fn compile(bundle: &Bundle, generation: u32) -> Result<Game> {
        compile_and_load(
            &bundle.source,
            &JitOptions {
                prefix: "g_",
                // Programs of different generations share one heap: their headers must differ.
                struct_base: generation * 100_000,
                manifests: &manifests(),
                symbols: iron_ts_ui::symbols(),
                ..Default::default()
            },
        )
        .map_err(|e| anyhow!("compiling the game: {e}"))
    }

    fn spawn_starting_components(&mut self, scene: &mut Scene) {
        let classes: Vec<&'static str> = self
            .host
            .registered_class_names()
            .into_iter()
            .filter(|c| !self.no_spawn.iter().any(|n| n == c))
            .collect();
        self.spawn_classes(scene, &classes);
    }

    fn spawn_classes(&mut self, scene: &mut Scene, classes: &[&'static str]) {
        use iron_api::Handle;
        let _scope = iron_scene::api::enter(scene);
        for &class in classes {
            let go = iron_scene::ObjectId::create();
            go.set_name(class);
            if let Err(e) = self.host.spawn(class, Some(go.to_raw())) {
                tracing::error!(target: "script", "{e}");
            }
        }
    }

    /// Scene files the program asked for (`SceneManager.load`): read from the game folder.
    /// Runs with the scene current.
    fn load_requested_scenes(&mut self) {
        for (id, path) in iron_scenes::take_load_requests() {
            let text = self
                .root
                .as_deref()
                .and_then(|root| iron_core::resolve_asset(root, &path))
                .ok_or_else(|| "must be a relative path inside the game folder".to_string())
                .and_then(|file| std::fs::read_to_string(&file).map_err(|e| e.to_string()));
            let result = match text {
                Ok(text) => iron_scenes::finish_load(id, &text),
                Err(e) => {
                    // Drops the request.
                    let _ = iron_scenes::finish_load(id, "");
                    Err(e)
                }
            };
            if let Err(e) = result {
                tracing::error!(target: "script", "loading scene `{path}`: {e}");
            }
            for w in iron_scenes::take_warnings() {
                tracing::warn!(target: "script", "scene `{path}`: {w}");
            }
        }
    }

    /// The game folder, for scene files.
    pub fn set_root(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
    }

    /// Loads the manifest's `iron.startScene` (a single-mode load, like the scripts' own).
    pub fn start_scene(&mut self, scene: &mut Scene, ui: &mut UiTree, path: &str) {
        self.with_scene(scene, ui, || {
            iron_scenes::request_load(path, false);
        });
    }

    /// Objects the program destroyed: components on them and the engine's data for them go too.
    /// Runs with the scene current.
    fn pump_destroyed(&mut self) {
        let gone = iron_engine_api::objects::process_destroyed();
        self.objects_destroyed(&gone);
    }

    fn objects_destroyed(&mut self, gone: &[iron_scene::ObjectId]) {
        use iron_api::Handle;
        if gone.is_empty() {
            return;
        }
        let raws: Vec<u64> = gone.iter().map(|o| o.to_raw()).collect();
        self.host.destroy_on_objects(&raws);
        iron_render_api::destroyed(gone);
        for o in gone {
            iron_physics_api::object_destroyed(*o);
        }
        log_faults(self.host.settle());
    }

    /// The world panel roots the program mounted since the last call.
    pub fn take_panel_mounts(&mut self) -> Vec<(u64, iron_ui_native::Element)> {
        std::mem::take(&mut self.panel_mounts)
    }

    /// The windows a reload left without an owner (no key): the app closes them.
    pub fn take_reload_closes(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.reload_closes)
    }

    /// The window roots the program mounted since the last call (window handle, root element).
    pub fn take_window_mounts(&mut self) -> Vec<(u64, iron_ui_native::Element)> {
        std::mem::take(&mut self.window_mounts)
    }

    /// What follows any call into the program: mounts, queued component work, microtasks.
    pub fn after_script(&mut self, scene: &mut Scene, ui: &mut UiTree) {
        let _scope = iron_scene::api::enter(scene);
        ts_rt_microtasks_run();
        log_faults(self.host.settle());
        self.pump_destroyed();
        self.load_requested_scenes();
        // Scene events reach the program's listeners once the world is consistent.
        iron_scenes::dispatch();
        log_faults(self.host.settle());
        // `createRoot(win)`: the main window is the screen; other windows get their root from the app.
        let mut mounts: Vec<(u64, u64)> = iron_ts_ui::take_mounts();
        for (window, handle) in iron_ts_ui::take_window_mounts() {
            let w = iron_engine_api::window::WindowRef(window as u32);
            if !iron_engine_api::window::can_set_up(w) {
                tracing::warn!(target: "script", "createRoot(window {window}): that window belongs to another family; ignored");
                continue;
            }
            if window == 1 {
                mounts.push((0, handle));
            } else {
                self.window_mounts
                    .push((window, iron_ts_ui::element_from(handle)));
            }
        }
        for (target, handle) in mounts {
            tracing::debug!(target, handle, "mount");
            let element = iron_ts_ui::element_from(handle);
            if target == 0 {
                let errors = match self.remount.take() {
                    // After a reload every component renders again with its new code.
                    Some(ids) => self.rt.refresh(self.screen_root, element, &ids, ui),
                    None => self.rt.render(self.screen_root, element, ui),
                };
                for e in errors {
                    tracing::warn!(target: "ui", "{e}");
                }
            } else {
                self.panel_mounts.push((target, element));
            }
        }
        for e in self.rt.flush(ui) {
            tracing::warn!(target: "ui", "{e}");
        }
        for e in iron_ts_ui::take_errors() {
            tracing::error!(target: "script", "{e}");
        }
    }

    /// One fixed tick: `onFixedUpdate` for every component, then the physics world steps and
    /// contact hooks run.
    pub fn fixed_tick(&mut self, scene: &mut Scene, ui: &mut UiTree, dt: f64) {
        {
            let _scope = iron_scene::api::enter(scene);
            log_faults(self.host.fixed_update(dt, &active));
            iron_physics_api::step();
            log_faults(self.host.dispatch_physics(&active));
        }
        self.after_script(scene, ui);
    }

    /// One frame: the game clock, timers, then `onUpdate` and `onLateUpdate`.
    pub fn frame(&mut self, scene: &mut Scene, ui: &mut UiTree, dt: f64, now_ms: f64) {
        {
            let _scope = iron_scene::api::enter(scene);
            iron_engine_api::time::advance(dt);
            ts_rt_timers_advance(now_ms);
            ts_rt_microtasks_run();
            log_faults(self.host.update(dt, &active));
        }
        self.after_script(scene, ui);
    }

    /// Last-moment changes before drawing (`onPreRender`).
    pub fn pre_render(&mut self, scene: &mut Scene, ui: &mut UiTree) {
        {
            let _scope = iron_scene::api::enter(scene);
            log_faults(self.host.pre_render(&active));
            // Emitters and the listener follow their objects.
            iron_engine_api::audio_components::pre_render();
        }
        self.after_script(scene, ui);
    }

    /// Runs one UI event through the handlers and says whether one prevented its default action
    /// (the interaction layer asks while it handles the input). Follow with [`Native::settle`].
    pub fn ui_event(&mut self, scene: &mut Scene, e: &UiEvent) -> bool {
        let Some(mut ev) = to_event(e) else {
            return false;
        };
        set_target_kind(self.rt.host_kind(ev.target));
        let _scope = iron_scene::api::enter(scene);
        self.rt.dispatch_handlers(&mut ev)
    }

    /// Commits what the event handlers changed and runs what they queued.
    pub fn settle(&mut self, scene: &mut Scene, ui: &mut UiTree) {
        for e in self.rt.settle(ui) {
            tracing::warn!(target: "ui", "{e}");
        }
        self.after_script(scene, ui);
    }

    /// UI events delivered after the fact (tests; the app asks [`Native::ui_event`] while its
    /// interaction layer handles the input).
    #[allow(dead_code)]
    pub fn ui_events(&mut self, scene: &mut Scene, ui: &mut UiTree, events: &[UiEvent]) {
        for e in events {
            self.ui_event(scene, e);
            self.settle(scene, ui);
        }
    }

    /// An input event for the game's `input` API, in the argument form `__iron_input` takes
    /// (`keydown code key repeat`, `mousemove x y`, `gamepadbutton id name value`, ...).
    pub fn game_input(&mut self, scene: &mut Scene, ui: &mut UiTree, args: &[iron_core::Value]) {
        {
            let _scope = iron_scene::api::enter(scene);
            super::game_input::apply_native(args);
        }
        self.after_script(scene, ui);
    }

    /// Sounds the mixer finished: `onEnded` callbacks and `AudioSource`s hear about it.
    pub fn audio_ended(
        &mut self,
        scene: &mut Scene,
        ui: &mut UiTree,
        ids: &[iron_engine_api::audio::PlaybackRef],
    ) {
        {
            let _scope = iron_scene::api::enter(scene);
            iron_engine_api::audio::ended(ids);
        }
        self.after_script(scene, ui);
    }

    /// Focus or pause changed: `Application.onFocusChanged` closures and the components'
    /// `onApplicationFocus` / `onApplicationPause`.
    pub fn focus_changed(
        &mut self,
        scene: &mut Scene,
        ui: &mut UiTree,
        focused: bool,
        paused: bool,
    ) {
        {
            let _scope = iron_scene::api::enter(scene);
            let change = iron_engine_api::app::set_focus(focused, paused);
            if change.focus_changed {
                log_faults(self.host.application_focus(focused, &active));
            }
            if change.pause_changed {
                log_faults(self.host.application_pause(paused, &active));
            }
        }
        self.after_script(scene, ui);
    }

    /// Runs `f` with the scene current, then what follows a call into the program.
    pub fn with_scene<R>(
        &mut self,
        scene: &mut Scene,
        ui: &mut UiTree,
        f: impl FnOnce() -> R,
    ) -> R {
        let r = {
            let _scope = iron_scene::api::enter(scene);
            f()
        };
        self.after_script(scene, ui);
        r
    }

    /// The game closes: everything that holds closures of the program is dropped here, in an
    /// order that does not depend on the order thread-local destructors run in.
    pub fn shutdown(mut self) {
        drop(std::mem::replace(
            &mut self.host,
            NativeHost::new(Vec::new()),
        ));
        iron_engine_api::input::reset();
        iron_engine_api::assets::reset();
        iron_engine_api::app::reset();
        iron_engine_api::screen::reset();
        iron_engine_api::audio::reset();
        iron_engine_api::audio_components::reset();
        iron_engine_api::animation::reset();
        iron_engine_api::camera::reset();
        iron_render_api::reset();
        iron_physics_api::reset();
        iron_scenes::reset();
        iron_engine_api::window::reset();
    }

    /// The class names of the running game.
    #[allow(dead_code)]
    pub fn classes(&self) -> Vec<&'static str> {
        self.host.class_names()
    }

    /// Replaces the game with a newly compiled one, keeping the world. Fails (and keeps running
    /// the old game) when the new source does not compile.
    ///
    /// The scenes are saved as scene files (compiled components included, through their
    /// `@property` tables), the engine's per-thread state is reset (listeners hold closures of the
    /// old code), the new program runs its top level, and the scenes load again: components come
    /// back on their objects with their saved fields, and `onAwake` runs for them. Objects the
    /// new program's top level made are removed first, as the saved ones stand for them. The UI
    /// keeps its hook state (`Runtime::refresh`), except for components whose hook list changed.
    pub fn reload(&mut self, bundle: Bundle, scene: &mut Scene, ui: &mut UiTree) -> Result<()> {
        let generation = self.generation + 1;
        // Nothing is touched until this works.
        let game = Self::compile(&bundle, generation)?;
        let old_sigs = parse_signatures(&signatures_of(&self.game));
        let old_classes = self.host.registered_class_names();
        let (saved, active) = {
            let _scope = iron_scene::api::enter(scene);
            let n = iron_scenes::scene_count();
            let saved: Vec<String> = (0..n)
                .filter_map(|i| iron_scenes::serialize(iron_scenes::scene_at(i)))
                .collect();
            let active = (0..n).position(|i| iron_scenes::scene_at(i) == iron_scenes::active());
            (saved, active.unwrap_or(0))
        };
        // The old code's window listeners go; windows with a key stay for the new code to claim.
        self.reload_closes = iron_engine_api::window::reload_begin()
            .into_iter()
            .map(|w| w.0)
            .collect();
        // Tear down: the old components (no `onDestroy`: the program is being replaced), then
        // everything that holds closures or objects of the old code.
        drop(std::mem::replace(
            &mut self.host,
            NativeHost::new(Vec::new()),
        ));
        iron_engine_api::input::reset();
        iron_engine_api::assets::reset();
        iron_engine_api::app::reset();
        iron_engine_api::screen::reset();
        iron_engine_api::audio::reset();
        iron_engine_api::audio_components::reset();
        iron_engine_api::animation::reset();
        iron_engine_api::camera::reset();
        iron_render_api::reset();
        super::native_render::reconfigure();
        iron_physics_api::reset();
        iron_scenes::reset();
        *scene = Scene::new();
        self.game = game;
        self.generation = generation;
        self.no_spawn = no_spawn_classes(&bundle.source);
        self.bundle = bundle;
        {
            let _scope = iron_scene::api::enter(scene);
            iron_engine_api::time::reset();
            (self.env)();
            self.game.init().map_err(|e| anyhow!("{e}"))?;
            let host = NativeHost::new(self.game.classes().map_err(|e| anyhow!("{e}"))?);
            // The stand-in's `Drop` would wipe the registry the new host just filled.
            std::mem::forget(std::mem::replace(&mut self.host, host));
        }
        // What the top level made is replaced by the saved world.
        if !saved.is_empty() {
            let mut gone = Vec::new();
            for r in scene.roots().to_vec() {
                gone.extend(scene.destroy(r));
            }
            let _scope = iron_scene::api::enter(scene);
            {
                self.objects_destroyed(&gone);
                let mut handles = Vec::new();
                for (i, text) in saved.iter().enumerate() {
                    match iron_scenes::load_scene(text, i > 0) {
                        Ok(h) => handles.push(h),
                        Err(e) => {
                            tracing::error!(target: "script", "reload: restoring a scene: {e}")
                        }
                    }
                }
                if let Some(h) = handles.get(active) {
                    iron_scenes::set_active(*h);
                }
                for w in iron_scenes::take_warnings() {
                    tracing::warn!(target: "script", "reload: {w}");
                }
            }
        }
        // Classes that are new in this program get their starting object.
        let fresh: Vec<&'static str> = self
            .host
            .registered_class_names()
            .into_iter()
            .filter(|c| !old_classes.contains(c) && !self.no_spawn.iter().any(|n| n == c))
            .collect();
        self.spawn_classes(scene, &fresh);
        // Components of the UI whose hooks changed start over.
        let new_sigs = parse_signatures(&signatures_of(&self.game));
        self.remount = Some(
            old_sigs
                .iter()
                .filter(|(id, hooks)| new_sigs.get(id).is_some_and(|n| n != *hooks))
                .map(|(id, _)| *id)
                .collect(),
        );
        self.after_script(scene, ui);
        self.remount = None;
        Ok(())
    }
}

thread_local! {
    static TARGET_KIND: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// The element kind of the target of the event about to be dispatched (what `e.target.type`
/// answers: the compiled side asks for the current event's target only).
pub fn set_target_kind(kind: Option<String>) {
    iron_ts_ui::set_target_kind_resolver(|_| TARGET_KIND.with(|k| k.borrow().clone()));
    TARGET_KIND.with(|k| *k.borrow_mut() = kind.unwrap_or_default());
}

/// A UI event of the interaction layer as the native reconciler's event.
pub fn to_event(e: &UiEvent) -> Option<Event> {
    let &target = e.path.first()?;
    let mut ev = Event::new(e.kind.name(), target);
    ev.bubbles = !matches!(e.kind, EventKind::PointerEnter | EventKind::PointerLeave);
    ev.data = e.script_args().get(2).map_or_else(Vec::new, |v| match v {
        iron_core::Value::Array(a) => a.iter().filter_map(iron_core::Value::as_f64).collect(),
        _ => Vec::new(),
    });
    if matches!(e.kind, EventKind::KeyDown | EventKind::KeyUp) {
        ev.strings = vec![e.key.clone(), e.code.clone()];
    } else if let Some(v) = &e.value {
        ev.strings = vec![v.clone()];
    }
    Some(ev)
}

/// Classes the program registers with `{ spawn: false }` (they get no starting object): the
/// compiler erases `register(..)`, so the options are read from the source text.
fn no_spawn_classes(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = source[at..].find("register(") {
        let start = at + i + "register(".len();
        at = start;
        // The call ends at its matching parenthesis.
        let mut depth = 1;
        let mut end = start;
        for (j, c) in source[start..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + j;
                        break;
                    }
                }
                _ => {}
            }
        }
        let args = &source[start..end];
        let compact: String = args.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact.contains("spawn:false") {
            continue;
        }
        // `{ A, B }, { spawn: false }`: the names of the first object.
        if args.trim_start().starts_with('{')
            && let (Some(a), Some(b)) = (args.find('{'), args.find('}'))
        {
            for name in args[a + 1..b].split(',') {
                let name = name.split(':').next().unwrap_or("").trim();
                if !name.is_empty() {
                    out.push(name.to_string());
                }
            }
        } else if let Some(name) = args.split(',').next() {
            out.push(name.trim().to_string());
        }
    }
    out
}

/// The program's `__uiSignatures()` text (`Name:ID:hook,hook;...`), empty when it has no UI.
fn signatures_of(game: &Game) -> String {
    let Some(f) = game.function("__uiSignatures") else {
        return String::new();
    };
    // SAFETY: the compiled function takes nothing and returns a live string.
    let p = unsafe {
        std::mem::transmute::<usize, unsafe extern "C" fn() -> *const iron_ts_rt::Str>(f)()
    };
    if p.is_null() {
        return String::new();
    }
    String::from_utf16_lossy(&unsafe { iron_ts_rt::Str::from_raw(p) }.to_utf16())
}

/// Component id to its hook list.
fn parse_signatures(text: &str) -> std::collections::HashMap<u32, String> {
    text.split(';')
        .filter_map(|e| {
            let mut parts = e.splitn(3, ':');
            let _name = parts.next()?;
            let id = parts.next()?.parse().ok()?;
            Some((id, parts.next().unwrap_or("").to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_ui::ROOT;
    use iron_ui::events::Modifiers;

    fn bundle(src: &str) -> Bundle {
        Bundle {
            source: src.to_string(),
            ..Default::default()
        }
    }

    fn start(src: &str) -> (Native, Scene, UiTree) {
        let mut scene = Scene::new();
        let mut ui = UiTree::new(400.0, 300.0);
        let n = Native::load(bundle(src), &mut scene, &mut ui).expect("the game compiles");
        (n, scene, ui)
    }

    fn dump(tree: &UiTree, id: iron_core::NodeId) -> String {
        let kind = tree.kind(id).unwrap_or("?");
        if kind == "#text" {
            return format!("text:{}", tree.text(id).unwrap_or(""));
        }
        let kids: Vec<String> = tree.children(id).iter().map(|c| dump(tree, *c)).collect();
        format!("{kind}({})", kids.join(","))
    }

    // M9-API-610
    #[test]
    fn a_bundle_is_json_with_a_format_number_and_a_source() {
        let b = Bundle::parse(
            r#"{"format":1,"title":"t","entry":"src/main.ts","source":"let a = 1;","css":"game.css","assets":["assets/x.png"]}"#,
        )
        .unwrap();
        assert_eq!(b.source, "let a = 1;");
        assert_eq!(b.css.as_deref(), Some("game.css"));
        assert_eq!(b.assets, ["assets/x.png"]);
        assert!(Bundle::parse(r#"{"format":2,"source":""}"#).is_err());
        assert!(Bundle::parse(r#"{"format":1}"#).is_err());
        assert!(Bundle::parse("not json").is_err());
        assert!(is_native(Path::new("dist/game.irbundle")));
        assert!(!is_native(Path::new("dist/game.js")));
    }

    // M9-API-611
    #[test]
    fn a_compiled_component_gets_an_object_and_a_frame_every_update() {
        let (mut n, mut scene, mut ui) = start(
            "class Mover extends Component {\n\
               onUpdate(dt: number): void {\n\
                 const t = this.gameObject!.transform;\n\
                 const p = t.localPosition;\n\
                 t.localPosition = new Vector3(p.x + dt, p.y, p.z);\n\
               }\n\
             }\n\
             register(Mover);",
        );
        assert_eq!(n.classes(), ["Mover"]);
        let go = scene.find("Mover").expect("the starting object");
        for i in 0..3 {
            n.frame(&mut scene, &mut ui, 0.5, 16.0 * (i + 1) as f64);
        }
        assert_eq!(scene.local(go).unwrap().position.x, 1.5);
    }

    // M9-API-612
    #[test]
    fn timers_run_from_the_frames_clock() {
        let (mut n, mut scene, mut ui) = start(
            "class Late extends Component {\n\
               onStart(): void {\n\
                 setTimeout(() => {\n\
                   this.gameObject!.transform.localPosition = new Vector3(7, 0, 0);\n\
                 }, 100);\n\
               }\n\
             }\n\
             register(Late);",
        );
        let go = scene.find("Late").unwrap();
        n.frame(&mut scene, &mut ui, 0.016, 50.0);
        assert_eq!(scene.local(go).unwrap().position.x, 0.0);
        n.frame(&mut scene, &mut ui, 0.016, 150.0);
        assert_eq!(scene.local(go).unwrap().position.x, 7.0);
    }

    // M9-API-613
    #[test]
    fn a_mounted_root_is_drawn_and_a_click_reaches_its_handler() {
        let (mut n, mut scene, mut ui) = start(
            "function Counter(props: { start: number }): UiElement {\n\
               const [n, setN] = useState(props.start);\n\
               return (\n\
                 <box style={{ width: 100 }} onClick={() => setN(n + 1)}>\n\
                   <text>{\"clicks \" + n}</text>\n\
                 </box>\n\
               );\n\
             }\n\
             function boot(): number {\n\
               UiRoot.mount(0, <Counter start={2} />);\n\
               return 0;\n\
             }\n\
             const booted = boot();",
        );
        assert_eq!(dump(&ui, ROOT), "root(box(text(text:clicks 2)))");
        let target = ui.children(ROOT)[0];
        let click = UiEvent::new(EventKind::Click, vec![target], Modifiers::default());
        n.ui_events(&mut scene, &mut ui, &[click.clone(), click]);
        assert_eq!(dump(&ui, ROOT), "root(box(text(text:clicks 4)))");
    }

    // M9-API-614
    #[test]
    fn a_program_that_does_not_compile_is_an_error_not_a_panic() {
        let mut scene = Scene::new();
        let mut ui = UiTree::new(100.0, 100.0);
        let e = Native::load(bundle("let x: number = ;"), &mut scene, &mut ui).err();
        assert!(e.is_some_and(|e| e.to_string().contains("compiling the game")));
    }

    const RELOAD_GAME: &str = "class Counter extends Component {\n\
           @property count = 0;\n\
           onUpdate(dt: number): void {\n\
             this.count += STEP;\n\
             this.gameObject!.transform.localPosition = new Vector3(this.count, 0, 0);\n\
           }\n\
         }\n\
         register(Counter);\n\
         function Label(): UiElement {\n\
           const [n, setN] = useState(0);\n\
           return (\n\
             <box style={{ width: 100 }} onClick={() => setN(n + 1)}>\n\
               <text>{\"WORD\" + n}</text>\n\
             </box>\n\
           );\n\
         }\n\
         function boot(): number {\n\
           UiRoot.mount(0, <Label />);\n\
           return 0;\n\
         }\n\
         const booted = boot();";

    // M9-API-615
    #[test]
    fn a_reload_keeps_component_fields_ui_state_and_the_objects() {
        let v1 = RELOAD_GAME.replace("STEP", "1").replace("WORD", "n");
        let v2 = RELOAD_GAME.replace("STEP", "10").replace("WORD", "m");
        let (mut n, mut scene, mut ui) = start(&v1);
        for i in 0..3 {
            n.frame(&mut scene, &mut ui, 0.016, 16.0 * (i + 1) as f64);
        }
        let go = scene.find("Counter").unwrap();
        assert_eq!(scene.local(go).unwrap().position.x, 3.0);
        let target = ui.children(ROOT)[0];
        let click = UiEvent::new(EventKind::Click, vec![target], Modifiers::default());
        n.ui_events(&mut scene, &mut ui, &[click.clone(), click]);
        assert_eq!(dump(&ui, ROOT), "root(box(text(text:n2)))");
        let objects = scene.len();

        n.reload(bundle(&v2), &mut scene, &mut ui).expect("reloads");
        assert_eq!(scene.len(), objects, "no object was added or lost");
        // The UI is the new code's, with the old hook state.
        assert_eq!(dump(&ui, ROOT), "root(box(text(text:m2)))");
        n.frame(&mut scene, &mut ui, 0.016, 100.0);
        let go = scene.find("Counter").unwrap();
        // count 3 was saved; the new code adds 10.
        assert_eq!(scene.local(go).unwrap().position.x, 13.0);
    }

    // M9-API-617
    #[test]
    fn a_failed_build_note_names_its_error() {
        assert_eq!(
            parse_reload_note(r#"{"seq":3,"ok":false,"error":"line 4: nope"}"#).as_deref(),
            Some("line 4: nope")
        );
        assert_eq!(parse_reload_note(r#"{"seq":4,"ok":true}"#), None);
        assert_eq!(parse_reload_note("garbage"), None);
    }

    // M9-API-616
    #[test]
    fn a_reload_that_does_not_compile_keeps_the_old_game_running() {
        let v1 = RELOAD_GAME.replace("STEP", "1").replace("WORD", "n");
        let (mut n, mut scene, mut ui) = start(&v1);
        let e = n.reload(bundle("let x: number = ;"), &mut scene, &mut ui);
        assert!(e.is_err());
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        let go = scene.find("Counter").unwrap();
        assert_eq!(scene.local(go).unwrap().position.x, 1.0);
        assert_eq!(dump(&ui, ROOT), "root(box(text(text:n0)))");
    }

    // M9-API-800
    #[test]
    fn keys_reach_the_input_state_and_the_listeners_of_compiled_code() {
        let (mut n, mut scene, mut ui) = start(
            "class Pad extends Component {
               hits = 0;
               onStart(): void {
                 input.onKeyDown((code: string, repeat: boolean) => { this.hits += 1; });
               }
               onUpdate(dt: number): void {
                 let x = this.hits;
                 if (input.getKeyDown(\"Space\")) x += 10;
                 this.gameObject!.transform.localPosition = new Vector3(x, 0, 0);
               }
             }
             register(Pad);",
        );
        let go = scene.find("Pad").unwrap();
        n.frame(&mut scene, &mut ui, 0.016, 8.0); // onStart registers the listener
        let key = |k: &str| vec!["keydown".into(), k.into(), "".into(), false.into()];
        n.game_input(&mut scene, &mut ui, &key("Space"));
        n.game_input(&mut scene, &mut ui, &key("KeyA"));
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        assert_eq!(scene.local(go).unwrap().position.x, 12.0);
        n.frame(&mut scene, &mut ui, 0.016, 32.0);
        assert_eq!(
            scene.local(go).unwrap().position.x,
            2.0,
            "pressed for one frame"
        );
    }

    // M9-API-801
    #[test]
    fn prevent_default_in_a_handler_reaches_the_interaction_layer() {
        let (mut n, mut scene, mut ui) = start(
            "function App(): UiElement {
               return <box style={{ width: 100 }} onClick={(e: UiEvent) => { e.preventDefault(); }}><text>x</text></box>;
             }
             function boot(): number {
               UiRoot.mount(0, <App />);
               return 0;
             }
             const booted = boot();",
        );
        let target = ui.children(ROOT)[0];
        let click = UiEvent::new(EventKind::Click, vec![target], Modifiers::default());
        assert!(n.ui_event(&mut scene, &click));
        n.settle(&mut scene, &mut ui);
        let other = UiEvent::new(EventKind::DblClick, vec![target], Modifiers::default());
        assert!(!n.ui_event(&mut scene, &other));
    }

    // M9-API-802
    #[test]
    fn fixed_ticks_step_physics_and_move_the_objects() {
        let (mut n, mut scene, mut ui) = start(
            "class Drop extends Component {
               onStart(): void {
                 const ball = new GameObject(\"ball\", { position: new Vector3(0, 5, 0) });
                 ball.addComponent(SphereCollider, { radius: 0.5 });
                 ball.addComponent(Rigidbody, { mass: 1 });
               }
             }
             register(Drop);",
        );
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        for _ in 0..30 {
            n.fixed_tick(&mut scene, &mut ui, 1.0 / 60.0);
        }
        let ball = scene.find("ball").expect("the ball");
        let y = scene.world(ball).position.y;
        assert!(y < 4.9 && y > 0.0, "the ball fell: {y}");
    }

    // M9-API-803
    #[test]
    fn audio_commands_come_out_and_finished_sounds_run_on_ended() {
        use iron_engine_api::audio::{AudioCommand, PlaybackRef};
        let (mut n, mut scene, mut ui) = start(
            "class Ding extends Component {
               onStart(): void {
                 const p = audio.load(\"sfx/hit.wav\").play();
                 p.onEnded(() => {
                   this.gameObject!.transform.localPosition = new Vector3(5, 0, 0);
                 });
               }
             }
             register(Ding);",
        );
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        let cmds = iron_engine_api::audio::take_commands();
        assert!(matches!(cmds.first(), Some(AudioCommand::Load { .. })));
        let play = cmds
            .iter()
            .find_map(|c| match c {
                AudioCommand::Play { playback, .. } => Some(*playback),
                _ => None,
            })
            .expect("a play command");
        let go = scene.find("Ding").unwrap();
        assert_eq!(scene.local(go).unwrap().position.x, 0.0);
        n.audio_ended(&mut scene, &mut ui, &[play, PlaybackRef(99)]);
        assert_eq!(scene.local(go).unwrap().position.x, 5.0);
    }

    // M9-API-804
    #[test]
    fn screen_quality_and_focus_closures_run_and_spawn_false_classes_get_no_object() {
        let (mut n, mut scene, mut ui) = start(
            "class Watch extends Component {\n\
               onStart(): void {\n\
                 Screen.onChange(() => {\n\
                   this.gameObject!.transform.localPosition = new Vector3(Screen.width, 0, 0);\n\
                 });\n\
                 QualitySettings.onLevelChanged((level: number) => {\n\
                   this.gameObject!.transform.localScale = new Vector3(level + 1, 1, 1);\n\
                 });\n\
                 Application.onFocusChanged((focused: boolean) => {\n\
                   this.gameObject!.transform.localPosition = new Vector3(focused ? 1 : 2, 3, 0);\n\
                 });\n\
               }\n\
             }\n\
             class Quiet extends Component {}\n\
             register(Quiet, { spawn: false });\n\
             register(Watch);",
        );
        assert!(scene.find("Quiet").is_none(), "spawn: false");
        assert!(scene.find("Watch").is_some());
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        let go = scene.find("Watch").unwrap();
        n.with_scene(&mut scene, &mut ui, || {
            iron_engine_api::screen::update(800, 600, 1.0)
        });
        assert_eq!(scene.local(go).unwrap().position.x, 800.0);
        n.with_scene(&mut scene, &mut ui, || {
            iron_engine_api::screen::quality_api::set_quality_level(2.0)
        });
        assert_eq!(scene.local(go).unwrap().scale.x, 3.0);
        n.focus_changed(&mut scene, &mut ui, false, true);
        assert_eq!(scene.local(go).unwrap().position.x, 2.0);
    }

    // M9-API-805
    #[test]
    fn spawn_false_is_read_from_the_register_calls() {
        assert_eq!(
            no_spawn_classes(
                "register({ A, B }, { spawn: false });\nregister(C);\nregister(D, {spawn:false});"
            ),
            ["A", "B", "D"]
        );
        assert!(no_spawn_classes("register(A);").is_empty());
    }

    // M9-API-806
    #[test]
    fn scene_files_load_from_the_game_folder_and_assets_changed_runs_its_closure() {
        let dir = std::env::temp_dir().join(format!("iron-native-scene-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lvl.scene.json"),
            r#"{"format":"iron.scene","version":1,"name":"Second","objects":[]}"#,
        )
        .unwrap();
        let (mut n, mut scene, mut ui) = start(
            "class Loader extends Component {\n\
               onStart(): void {\n\
                 SceneManager.onSceneLoaded((s: Scene, mode: string) => {\n\
                   this.gameObject!.transform.localPosition = new Vector3(mode === \"additive\" ? 4 : 5, 0, 0);\n\
                 });\n\
                 Assets.onChanged((paths: string) => {\n\
                   this.gameObject!.transform.localScale = new Vector3(paths.length, 1, 1);\n\
                 });\n\
                 SceneManager.loadSceneAsync(\"lvl.scene.json\", \"additive\");\n\
               }\n\
             }\n\
             register(Loader);",
        );
        n.set_root(&dir);
        let go = scene.find("Loader").unwrap();
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        assert_eq!(scene.local(go).unwrap().position.x, 4.0);
        n.with_scene(&mut scene, &mut ui, || {
            iron_engine_api::assets::changed(["a.png".to_string()]);
        });
        assert_eq!(scene.local(go).unwrap().scale.x, 5.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M9-API-808
    #[test]
    fn only_registered_classes_start_and_an_event_target_knows_its_kind() {
        let (mut n, mut scene, mut ui) = start(
            "class Helper extends Component {\n\
               onStart(): void {\n\
                 this.gameObject!.transform.localPosition = new Vector3(9, 0, 0);\n\
               }\n\
             }\n\
             class Boss extends Component {\n\
               onStart(): void {\n\
                 this.gameObject!.addComponent(Helper);\n\
               }\n\
             }\n\
             register(Boss);\n\
             function App(): UiElement {\n\
               return <box style={{ width: 100 }} onClick={(e: UiEvent) => {\n\
                 if (e.target.type === \"box\") { UiRoot.mount(0, <text>box</text>); }\n\
               }}><text>x</text></box>;\n\
             }\n\
             function boot(): number {\n\
               UiRoot.mount(0, <App />);\n\
               return 0;\n\
             }\n\
             const booted = boot();",
        );
        assert!(
            scene.find("Helper").is_none(),
            "unregistered: no starting object"
        );
        n.frame(&mut scene, &mut ui, 0.016, 16.0);
        let go = scene.find("Boss").unwrap();
        assert_eq!(
            scene.local(go).unwrap().position.x,
            9.0,
            "addComponent works"
        );
        let target = ui.children(ROOT)[0];
        let click = UiEvent::new(EventKind::Click, vec![target], Modifiers::default());
        n.ui_events(&mut scene, &mut ui, &[click]);
        assert_eq!(dump(&ui, ROOT), "root(text(text:box))");
    }
}
