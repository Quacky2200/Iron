//! Pointer interaction: turns pointer input into DOM-style events and keeps `:hover`
//! and `:active` up to date (DESIGN D6). Dispatching the events to scripts is the
//! caller's job: [`UiEvent::script_args`] are the arguments for `__iron_ui_event`,
//! which `@iron/react` runs through capture and bubble phases.

use iron_core::{NodeId, Value};

use crate::attrs::ElementState;
use crate::tree::UiTree;

/// Two presses closer than this (in time and distance) count as a double click.
pub const DOUBLE_CLICK_MS: f64 = 500.0;
pub const DOUBLE_CLICK_DISTANCE: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    PointerMove,
    PointerDown,
    PointerUp,
    PointerEnter,
    PointerLeave,
    Click,
    DblClick,
    ContextMenu,
    Wheel,
}

impl EventKind {
    /// The DOM event type.
    pub fn name(self) -> &'static str {
        match self {
            EventKind::PointerMove => "pointermove",
            EventKind::PointerDown => "pointerdown",
            EventKind::PointerUp => "pointerup",
            EventKind::PointerEnter => "pointerenter",
            EventKind::PointerLeave => "pointerleave",
            EventKind::Click => "click",
            EventKind::DblClick => "dblclick",
            EventKind::ContextMenu => "contextmenu",
            EventKind::Wheel => "wheel",
        }
    }
}

/// Held modifier keys, sent with every event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    /// shift = 1, ctrl = 2, alt = 4, meta = 8 (decoded by `@iron/react`).
    pub fn bits(self) -> u32 {
        self.shift as u32
            | (self.ctrl as u32) << 1
            | (self.alt as u32) << 2
            | (self.meta as u32) << 3
    }
}

/// An event for scripts. Fields follow the DOM's `PointerEvent` / `WheelEvent`.
#[derive(Clone, Debug, PartialEq)]
pub struct UiEvent {
    pub kind: EventKind,
    /// The target first, then its ancestors up to (not including) the root. `pointerenter`
    /// and `pointerleave` don't bubble, so their path is just the target.
    pub path: Vec<NodeId>,
    /// Pointer position in logical pixels.
    pub x: f32,
    pub y: f32,
    /// The button that changed: 0 primary, 1 middle, 2 secondary, 3 back, 4 forward.
    pub button: u8,
    /// Held buttons as a bitmask: 1 primary, 2 secondary, 4 middle, 8 back, 16 forward.
    pub buttons: u8,
    /// The click count for presses, releases and clicks.
    pub detail: u32,
    /// Wheel deltas in logical pixels; positive `y` scrolls down.
    pub delta_x: f32,
    pub delta_y: f32,
    pub modifiers: Modifiers,
}

impl UiEvent {
    pub fn target(&self) -> NodeId {
        self.path[0]
    }

    /// Arguments for `__iron_ui_event(type, path, data)`, where `data` is
    /// `[x, y, button, buttons, detail, deltaX, deltaY, modifiers]`.
    pub fn script_args(&self) -> [Value; 3] {
        let path = self
            .path
            .iter()
            .map(|id| Value::Number(id.0.into()))
            .collect();
        let data = [
            self.x as f64,
            self.y as f64,
            self.button as f64,
            self.buttons as f64,
            self.detail as f64,
            self.delta_x as f64,
            self.delta_y as f64,
            self.modifiers.bits() as f64,
        ];
        [
            self.kind.name().into(),
            Value::Array(path),
            Value::Array(data.into_iter().map(Value::Number).collect()),
        ]
    }
}

/// The DOM's `buttons` bit for a `button` number.
fn button_bit(button: u8) -> u8 {
    match button {
        0 => 1,
        1 => 4,
        2 => 2,
        b if b < 8 => 1 << b,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug)]
struct LastPress {
    target: NodeId,
    button: u8,
    time_ms: f64,
    x: f32,
    y: f32,
    count: u32,
}

/// The mouse pointer's state over a [`UiTree`]: what it hovers, which buttons are
/// held and where they were pressed.
///
/// Each method takes the tree, updates `:hover` / `:active` on it and returns the
/// events to dispatch, in order.
#[derive(Debug, Default)]
pub struct Pointer {
    pos: Option<(f32, f32)>,
    /// The hovered element's event path (target first).
    hovered: Vec<NodeId>,
    buttons: u8,
    /// Where each button (by number) was pressed.
    pressed: [Option<NodeId>; 5],
    /// The primary button's press path, which has `:active`.
    active: Vec<NodeId>,
    last_press: Option<LastPress>,
    clicks: u32,
    modifiers: Modifiers,
}

impl Pointer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// The pointer position, or `None` when it's outside the window.
    pub fn position(&self) -> Option<(f32, f32)> {
        self.pos
    }

    /// The element under the pointer.
    pub fn hovered(&self) -> Option<NodeId> {
        self.hovered.first().copied()
    }

    /// The CSS cursor for the hovered element.
    pub fn cursor(&self, tree: &UiTree) -> &'static str {
        tree.cursor(self.hovered())
    }

    /// The pointer moved to (x, y), in logical pixels.
    pub fn move_to(&mut self, tree: &mut UiTree, x: f32, y: f32) -> Vec<UiEvent> {
        self.pos = Some((x, y));
        let mut events = self.rehover(tree);
        if !self.hovered.is_empty() {
            events.push(self.event(EventKind::PointerMove, self.hovered.clone(), 0));
        }
        events
    }

    /// The pointer left the window.
    pub fn leave(&mut self, tree: &mut UiTree) -> Vec<UiEvent> {
        self.pos = None;
        self.rehover(tree)
    }

    /// Re-hit-tests at the current position, for when the tree changed under a still
    /// pointer (an element moved in or out). Call once per frame after commits.
    pub fn refresh(&mut self, tree: &mut UiTree) -> Vec<UiEvent> {
        self.active.retain(|id| tree.contains(*id));
        self.rehover(tree)
    }

    pub fn down(&mut self, tree: &mut UiTree, button: u8, time_ms: f64) -> Vec<UiEvent> {
        let mut events = self.rehover(tree);
        self.buttons |= button_bit(button);
        let Some(target) = self.hovered() else {
            return events;
        };
        let (x, y) = self.pos.unwrap_or_default();
        self.clicks = match self.last_press {
            Some(last)
                if last.target == target
                    && last.button == button
                    && time_ms - last.time_ms <= DOUBLE_CLICK_MS
                    && (last.x - x).hypot(last.y - y) <= DOUBLE_CLICK_DISTANCE =>
            {
                last.count + 1
            }
            _ => 1,
        };
        self.last_press = Some(LastPress {
            target,
            button,
            time_ms,
            x,
            y,
            count: self.clicks,
        });
        if let Some(slot) = self.pressed.get_mut(button as usize) {
            *slot = Some(target);
        }
        if button == 0 {
            self.set_active(tree, self.hovered.clone());
        }
        events.push(self.event(EventKind::PointerDown, self.hovered.clone(), button));
        events
    }

    pub fn up(&mut self, tree: &mut UiTree, button: u8) -> Vec<UiEvent> {
        let mut events = self.rehover(tree);
        self.buttons &= !button_bit(button);
        if button == 0 {
            self.set_active(tree, Vec::new());
        }
        let pressed = self.pressed.get_mut(button as usize).and_then(Option::take);
        if self.hovered.is_empty() {
            return events;
        }
        events.push(self.event(EventKind::PointerUp, self.hovered.clone(), button));

        // A click goes to the nearest element containing both the press and the release.
        let common = pressed.and_then(|down| {
            let down_path = tree.event_path(down);
            self.hovered.iter().position(|id| down_path.contains(id))
        });
        if let Some(i) = common {
            let path = self.hovered[i..].to_vec();
            match button {
                0 => {
                    events.push(self.event(EventKind::Click, path.clone(), button));
                    if self.clicks == 2 {
                        events.push(self.event(EventKind::DblClick, path, button));
                    }
                }
                2 => events.push(self.event(EventKind::ContextMenu, path, button)),
                _ => {}
            }
        }
        events
    }

    /// Wheel deltas in logical pixels, positive `dy` scrolling down.
    pub fn wheel(&mut self, tree: &mut UiTree, dx: f32, dy: f32) -> Vec<UiEvent> {
        let mut events = self.rehover(tree);
        if !self.hovered.is_empty() {
            let mut e = self.event(EventKind::Wheel, self.hovered.clone(), 0);
            e.delta_x = dx;
            e.delta_y = dy;
            events.push(e);
        }
        events
    }

    /// Forgets held buttons (the window lost focus, so releases may never arrive).
    pub fn cancel(&mut self, tree: &mut UiTree) {
        self.buttons = 0;
        self.pressed = [None; 5];
        self.set_active(tree, Vec::new());
    }

    fn event(&self, kind: EventKind, path: Vec<NodeId>, button: u8) -> UiEvent {
        let (x, y) = self.pos.unwrap_or_default();
        let detail = match kind {
            EventKind::PointerDown | EventKind::PointerUp | EventKind::Click => self.clicks,
            EventKind::DblClick => 2,
            _ => 0,
        };
        UiEvent {
            kind,
            path,
            x,
            y,
            button,
            buttons: self.buttons,
            detail,
            delta_x: 0.0,
            delta_y: 0.0,
            modifiers: self.modifiers,
        }
    }

    /// Hit-tests at the current position and moves `:hover`, returning
    /// `pointerleave`s (innermost first) then `pointerenter`s (outermost first).
    fn rehover(&mut self, tree: &mut UiTree) -> Vec<UiEvent> {
        let target = self.pos.and_then(|(x, y)| tree.hit_test(x, y));
        let path = target.map(|t| tree.event_path(t)).unwrap_or_default();
        if path == self.hovered {
            return Vec::new();
        }
        let old = std::mem::replace(&mut self.hovered, path);
        let mut events = Vec::new();
        for id in old.iter().filter(|id| !self.hovered.contains(id)) {
            if tree.contains(*id) {
                update_state(tree, *id, |s| s.hover = false);
                events.push(self.event(EventKind::PointerLeave, vec![*id], 0));
            }
        }
        for id in self.hovered.iter().rev().filter(|id| !old.contains(id)) {
            update_state(tree, *id, |s| s.hover = true);
            events.push(self.event(EventKind::PointerEnter, vec![*id], 0));
        }
        events
    }

    fn set_active(&mut self, tree: &mut UiTree, path: Vec<NodeId>) {
        for id in self.active.iter().filter(|id| !path.contains(id)) {
            update_state(tree, *id, |s| s.active = false);
        }
        for id in &path {
            update_state(tree, *id, |s| s.active = true);
        }
        self.active = path;
    }
}

fn update_state(tree: &mut UiTree, id: NodeId, f: impl FnOnce(&mut ElementState)) {
    let mut state = tree.state(id);
    f(&mut state);
    tree.set_state(id, state);
}
