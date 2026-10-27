// SPDX-License-Identifier: Apache-2.0

//! The multi-window platform path, driven by the fake platform (no display, no GPU).

use std::collections::BTreeSet;
use std::sync::Arc;

use platypus_platform::{
    FakePlatform, InputEvent, Os, PlatformApp, Window, WindowCommand, WindowHost, WindowKey,
    WindowKind, WindowSpec,
};

#[derive(Default)]
struct Rec {
    commands: Vec<WindowCommand>,
    opened: Vec<(WindowKey, (u32, u32))>,
    failed: Vec<(WindowKey, String)>,
    /// Events in `window_input`, with their window.
    inputs: Vec<(WindowKey, InputEvent)>,
    closed: Vec<WindowKey>,
    refuse_close: BTreeSet<WindowKey>,
    to_show: Vec<WindowKey>,
    frames: u32,
    events_done: u32,
}

impl PlatformApp for Rec {
    fn window_ready(&mut self, _window: Arc<Window>) {
        unreachable!("the fake platform has no OS window");
    }
    fn input(&mut self, _event: InputEvent) {
        unreachable!("this app overrides window_input");
    }
    fn frame(&mut self) {
        self.frames += 1;
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
    fn window_opened(&mut self, key: WindowKey, host: WindowHost) {
        let WindowHost::Virtual { width, height, .. } = host else {
            panic!("the fake platform only makes virtual windows");
        };
        self.opened.push((key, (width, height)));
    }
    fn window_open_failed(&mut self, key: WindowKey, error: String) {
        self.failed.push((key, error));
    }
    fn window_input(&mut self, key: WindowKey, event: InputEvent) {
        self.inputs.push((key, event));
    }
    fn close_requested(&mut self, key: WindowKey) -> bool {
        !self.refuse_close.contains(&key)
    }
    fn window_closed(&mut self, key: WindowKey) {
        self.closed.push(key);
    }
    fn events_done(&mut self) {
        self.events_done += 1;
    }
    fn take_windows_to_show(&mut self) -> Vec<WindowKey> {
        std::mem::take(&mut self.to_show)
    }
}

fn k(n: u32) -> WindowKey {
    WindowKey(n)
}

fn spec(kind: WindowKind, parent: Option<WindowKey>) -> WindowSpec {
    WindowSpec {
        title: format!("{kind:?}"),
        size: (200.0, 100.0),
        kind,
        parent,
        ..WindowSpec::default()
    }
}

fn platform() -> FakePlatform<Rec> {
    FakePlatform::new(Rec::default(), WindowSpec::main("Game", 800.0, 600.0), 2.0)
}

// M9-API-1441
#[test]
fn one_window_apps_get_main_window_events_through_input() {
    #[derive(Default)]
    struct OneWindow {
        got: Vec<InputEvent>,
    }
    impl PlatformApp for OneWindow {
        fn window_ready(&mut self, _w: Arc<Window>) {}
        fn input(&mut self, event: InputEvent) {
            self.got.push(event);
        }
        fn frame(&mut self) {}
    }
    let mut p = FakePlatform::new(
        OneWindow::default(),
        WindowSpec::main("Game", 100.0, 100.0),
        1.0,
    );
    p.send(WindowKey::MAIN, InputEvent::Focused(true));
    p.send(k(9), InputEvent::Focused(true));
    assert_eq!(p.app().got, vec![InputEvent::Focused(true)]);
    // Closing the main window still reaches `input`, as it did before windows.
    assert!(p.user_close(WindowKey::MAIN));
    assert_eq!(p.app().got.last(), Some(&InputEvent::CloseRequested));
    assert!(p.exited());
}

// M9-API-1442
#[test]
fn open_commands_open_windows_and_report_them() {
    let mut p = platform();
    assert_eq!(p.app().opened, vec![(WindowKey::MAIN, (1600, 1200))]);
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
    p.step();
    // 200 x 100 logical at scale 2.
    assert_eq!(p.app().opened.last(), Some(&(k(2), (400, 200))));
    let w = p.window(k(2)).unwrap();
    assert_eq!(w.plan.owner, Some(WindowKey::MAIN));
    assert_eq!(w.plan.kind, WindowKind::Tool);
    assert_eq!(p.tree().parent(k(2)), Some(WindowKey::MAIN));
    // The same key twice fails instead of replacing the window.
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Dialog, None)));
    p.step();
    assert_eq!(p.app().failed.len(), 1);
    assert_eq!(p.window(k(2)).unwrap().plan.kind, WindowKind::Tool);
}

// M9-API-1443
#[test]
fn windows_stay_hidden_until_the_app_says_they_were_drawn() {
    let mut p = platform();
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Normal, None)));
    p.step();
    assert!(!p.window(k(2)).unwrap().shown);
    p.step();
    assert!(
        !p.window(k(2)).unwrap().shown,
        "frames alone do not show it"
    );
    p.app().to_show.push(k(2));
    p.step();
    assert!(p.window(k(2)).unwrap().shown);
}

// M9-API-1444
#[test]
fn a_cancelled_close_request_keeps_the_window() {
    let mut p = platform();
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
    p.step();
    p.app().refuse_close.insert(k(2));
    assert!(!p.user_close(k(2)));
    assert!(p.window(k(2)).is_some() && p.app().closed.is_empty());
    p.app().refuse_close.clear();
    assert!(p.user_close(k(2)));
    assert!(p.window(k(2)).is_none());
    assert_eq!(p.app().closed, vec![k(2)]);
    assert!(!p.exited());
}

// M9-API-1444
#[test]
fn closing_the_main_window_exits() {
    let mut p = platform();
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
    p.step();
    p.app().refuse_close.insert(WindowKey::MAIN);
    assert!(!p.user_close(WindowKey::MAIN));
    assert!(!p.exited());
    p.app().refuse_close.clear();
    assert!(p.user_close(WindowKey::MAIN));
    assert!(p.exited());
    // Its tool windows closed first.
    assert_eq!(p.app().closed, vec![k(2)]);
    // A script `Close(MAIN)` is not how the main window ends.
    let mut p = platform();
    p.app().commands.push(WindowCommand::Close(WindowKey::MAIN));
    p.step();
    assert!(!p.exited() && p.window(WindowKey::MAIN).is_some());
}

// M9-API-1445
#[test]
fn closing_a_parent_closes_its_children_first() {
    let mut p = platform();
    p.app().commands.extend([
        WindowCommand::Open(k(2), spec(WindowKind::Tool, None)),
        WindowCommand::Open(k(3), spec(WindowKind::Tool, Some(k(2)))),
        WindowCommand::Open(k(4), spec(WindowKind::Tool, None)),
    ]);
    p.step();
    assert_eq!(p.keys(), vec![k(1), k(2), k(3), k(4)]);
    p.app().commands.extend([
        WindowCommand::Close(k(2)),
        // Already gone with its parent: harmless.
        WindowCommand::Close(k(3)),
    ]);
    p.step();
    assert_eq!(p.app().closed, vec![k(3), k(2)]);
    assert_eq!(p.keys(), vec![k(1), k(4)]);
}

// M9-API-1446
#[test]
fn setters_reach_the_window() {
    let mut p = platform();
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
    p.step();
    p.app().inputs.clear();
    p.app().commands.extend([
        WindowCommand::SetTitle(k(2), "Renamed".into()),
        WindowCommand::SetSize(k(2), 300.0, 150.0),
        WindowCommand::SetPosition(k(2), 40.0, 50.0),
        WindowCommand::SetKind(k(2), WindowKind::Dialog),
        WindowCommand::SetMinimized(k(2), true),
    ]);
    p.step();
    let w = p.window(k(2)).unwrap();
    assert_eq!(w.spec.title, "Renamed");
    assert_eq!((w.spec.size, w.physical), ((300.0, 150.0), (600, 300)));
    assert_eq!(w.position, (40.0, 50.0));
    assert!(w.minimized);
    // The plan was recomputed for the new kind.
    assert_eq!(w.plan.kind, WindowKind::Dialog);
    assert_eq!(w.plan.x11_type, Some("dialog"));
    assert!(!w.plan.tool_caption);
    // The OS would answer with the new size and position.
    assert_eq!(
        p.app().inputs,
        vec![
            (
                k(2),
                InputEvent::Resized {
                    width: 600,
                    height: 300
                }
            ),
            (k(2), InputEvent::Moved { x: 80, y: 100 }),
        ]
    );
    let mut flags = spec(WindowKind::Dialog, None);
    flags.resizable = false;
    flags.always_on_top = true;
    p.app().commands.push(WindowCommand::SetFlags(k(2), flags));
    p.step();
    let w = p.window(k(2)).unwrap();
    assert!(!w.plan.resizable && w.plan.level_on_top);
}

#[test]
fn the_os_decides_what_a_style_means() {
    let mut p = platform().os(Os::MacOs);
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Child, None)));
    p.step();
    let w = p.window(k(2)).unwrap();
    assert_eq!(
        w.plan.kind,
        WindowKind::Tool,
        "no embedded children on macOS"
    );
    assert_eq!(w.plan.embed_in, None);
}

// M9-API-1448
#[test]
fn focus_moves_between_windows_and_events_done_runs_once_per_step() {
    let mut p = platform();
    p.app().commands.extend([
        WindowCommand::Open(k(2), spec(WindowKind::Tool, None)),
        WindowCommand::Open(k(3), spec(WindowKind::Tool, None)),
    ]);
    p.step();
    p.focus(k(2));
    p.app().inputs.clear();
    p.focus(k(3));
    assert_eq!(
        p.app().inputs,
        vec![
            (k(2), InputEvent::Focused(false)),
            (k(3), InputEvent::Focused(true)),
        ]
    );
    assert!(p.window(k(3)).unwrap().focused && !p.window(k(2)).unwrap().focused);
    // A focus command from the app moves it too.
    p.app().commands.push(WindowCommand::Focus(k(2)));
    p.step();
    assert!(p.window(k(2)).unwrap().focused);
    let (frames, done) = (p.app().frames, p.app().events_done);
    p.step();
    p.step();
    assert_eq!(p.app().frames - frames, 2);
    assert_eq!(p.app().events_done - done, 2);
}

// M9-API-1450
#[test]
fn platforms_without_secondary_windows_fail_the_open_with_the_exact_error() {
    for (os, name) in [(Os::Android, "android"), (Os::Ios, "ios"), (Os::Web, "web")] {
        let mut p = platform().os(os);
        p.app()
            .commands
            .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
        p.step();
        let want = format!("Cannot open Window due to platform restrictions (platform: {name})");
        assert_eq!(p.app().failed, vec![(k(2), want)]);
        // Never opened; the main window is untouched.
        assert!(p.window(k(2)).is_none());
        assert!(p.window(WindowKey::MAIN).is_some());
        assert_eq!(p.app().opened.len(), 1);
        assert!(!os.supports_windows());
    }
}

// M9-API-1451
#[test]
fn desktop_platforms_open_windows_without_errors() {
    for os in [Os::Windows, Os::MacOs, Os::X11, Os::Wayland, Os::Other] {
        assert!(os.supports_windows());
        let mut p = platform().os(os);
        p.app()
            .commands
            .push(WindowCommand::Open(k(2), spec(WindowKind::Normal, None)));
        p.step();
        assert!(p.app().failed.is_empty(), "{os:?}");
        assert!(p.window(k(2)).is_some());
        assert!(
            platypus_platform::plan(&spec(WindowKind::Normal, None), os)
                .error
                .is_none()
        );
    }
}

// M9-API-1452
#[test]
fn a_tool_window_on_wayland_opens_as_a_plain_window_with_a_specific_warning() {
    let mut p = platform().os(Os::Wayland);
    p.app()
        .commands
        .push(WindowCommand::Open(k(2), spec(WindowKind::Tool, None)));
    p.step();
    assert!(p.app().failed.is_empty());
    let w = p.window(k(2)).unwrap();
    assert_eq!(
        w.plan.warning.as_deref(),
        Some(
            "Cannot open Window due to platform restrictions: tool windows are not supported on Wayland"
        )
    );
}
