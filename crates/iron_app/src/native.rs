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
//! Differences from the QuickJS path (also in `tools/iron_ts/TODO.md`): UI events reach handlers
//! after the interaction layer finished its default actions, so `preventDefault()` does not stop
//! them; the 3D renderer is not fed from the compiled programs' registries yet.

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
    /// Errors the UI reported, shown once.
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
    pub fn load(bundle: Bundle, scene: &mut Scene, ui: &mut UiTree) -> Result<Native> {
        let generation = 0;
        iron_ts_rt::ts_rt_set_console(Some(console_hook));
        register_kinds();
        let game = Self::compile(&bundle, generation)?;
        let mut rt = Runtime::new(FIRST_NODE);
        let screen_root = rt.create_root(iron_ui::ROOT);
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
        use iron_api::Handle;
        let _scope = iron_scene::api::enter(scene);
        for class in self.host.class_names() {
            // `register(.., { spawn: false })` is not expressible yet: every class starts.
            let go = iron_scene::ObjectId::create();
            go.set_name(class);
            if let Err(e) = self.host.spawn(class, Some(go.to_raw())) {
                tracing::error!(target: "script", "{e}");
            }
        }
    }

    /// What follows any call into the program: mounts, queued component work, microtasks.
    fn after_script(&mut self, scene: &mut Scene, ui: &mut UiTree) {
        let _scope = iron_scene::api::enter(scene);
        ts_rt_microtasks_run();
        log_faults(self.host.settle());
        for (target, handle) in iron_ts_ui::take_mounts() {
            tracing::debug!(target, handle, "mount");
            let element = iron_ts_ui::element_from(handle);
            if target == 0 {
                for e in self.rt.render(self.screen_root, element, ui) {
                    tracing::warn!(target: "ui", "{e}");
                }
            } else {
                tracing::warn!(target: "ui", "world panels are not wired to compiled games yet");
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
        }
        self.after_script(scene, ui);
    }

    /// UI events the interaction layer produced, delivered to the handlers.
    pub fn ui_events(&mut self, scene: &mut Scene, ui: &mut UiTree, events: &[UiEvent]) {
        for e in events {
            let Some(&target) = e.path.first() else {
                continue;
            };
            let mut ev = Event::new(e.kind.name(), target);
            ev.bubbles = !matches!(e.kind, EventKind::PointerEnter | EventKind::PointerLeave);
            ev.data = e.script_args().get(2).map_or_else(Vec::new, |v| match v {
                iron_core::Value::Array(a) => {
                    a.iter().filter_map(iron_core::Value::as_f64).collect()
                }
                _ => Vec::new(),
            });
            if matches!(e.kind, EventKind::KeyDown | EventKind::KeyUp) {
                ev.strings = vec![e.key.clone(), e.code.clone()];
            } else if let Some(v) = &e.value {
                ev.strings = vec![v.clone()];
            }
            {
                let _scope = iron_scene::api::enter(scene);
                self.rt.dispatch(&mut ev, ui);
            }
            self.after_script(scene, ui);
        }
    }

    /// The class names of the running game.
    #[allow(dead_code)]
    pub fn classes(&self) -> Vec<&'static str> {
        self.host.class_names()
    }

    /// Replaces the game with a newly compiled one. Fails (keeping the old game) when the new
    /// source does not compile.
    pub fn reload(&mut self, bundle: Bundle, _scene: &mut Scene, _ui: &mut UiTree) -> Result<()> {
        let generation = self.generation + 1;
        let game = Self::compile(&bundle, generation)?;
        let _ = game;
        bail!("hot reload of compiled games is not built yet ({generation})")
    }
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
}
