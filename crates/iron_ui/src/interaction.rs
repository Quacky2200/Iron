//! Focus, keyboard and default actions on top of [`Pointer`].
//!
//! [`Interaction::handle`] takes one input, dispatches the resulting events through
//! a callback (which returns whether the script prevented the default), and then runs
//! the default actions like a browser: pressing the primary button focuses the
//! nearest focusable element (or blurs), Tab / Shift-Tab move focus.

use iron_core::NodeId;

use crate::events::{EventKind, Modifiers, Pointer, UiEvent, update_state};
use crate::tree::{ROOT, UiTree};

/// One input for [`Interaction::handle`]. Positions and deltas are logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerLeave,
    PointerDown {
        button: u8,
        time_ms: f64,
    },
    PointerUp {
        button: u8,
    },
    Wheel {
        dx: f32,
        dy: f32,
    },
    /// `key` and `code` as in the DOM (`"a"` / `"KeyA"`).
    KeyDown {
        key: String,
        code: String,
        repeat: bool,
    },
    KeyUp {
        key: String,
        code: String,
    },
    Modifiers(Modifiers),
    /// The window lost focus: held buttons are forgotten.
    WindowBlur,
    /// `ref.focus()` from a script.
    Focus(NodeId),
    /// `ref.blur()` from a script: blurs the element if it has focus.
    Blur(NodeId),
    /// The tree changed: re-hit-test the pointer and drop focus from removed or hidden
    /// elements. Call once per frame after commits.
    Refresh,
}

/// Pointer, focus and keyboard state over one [`UiTree`].
#[derive(Debug, Default)]
pub struct Interaction {
    pointer: Pointer,
    focused: Option<NodeId>,
    modifiers: Modifiers,
    /// Whether the last focus-moving input was the keyboard, which decides
    /// `:focus-visible` for focus set by scripts (the browsers' heuristic).
    keyboard: bool,
}

impl Interaction {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pointer(&self) -> &Pointer {
        &self.pointer
    }

    /// The element with keyboard focus.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// The CSS cursor for the hovered element.
    pub fn cursor(&self, tree: &UiTree) -> &'static str {
        self.pointer.cursor(tree)
    }

    /// Handles one input. `dispatch` sends an event to scripts and returns whether its
    /// default action was prevented.
    pub fn handle(
        &mut self,
        tree: &mut UiTree,
        input: Input,
        dispatch: &mut dyn FnMut(&UiEvent) -> bool,
    ) {
        match input {
            Input::PointerMove { x, y } => send(self.pointer.move_to(tree, x, y), dispatch),
            Input::PointerLeave => send(self.pointer.leave(tree), dispatch),
            Input::PointerDown { button, time_ms } => {
                let mut pressed_nothing = button == 0;
                for event in self.pointer.down(tree, button, time_ms) {
                    let prevented = dispatch(&event);
                    if event.kind == EventKind::PointerDown && button == 0 {
                        pressed_nothing = false;
                        if !prevented {
                            let target = event.path.iter().copied().find(|id| focusable(tree, *id));
                            self.pointer_focus(tree, target, dispatch);
                        }
                    }
                }
                // A press on empty space (the root) blurs, like clicking the page body.
                if pressed_nothing {
                    self.pointer_focus(tree, None, dispatch);
                }
            }
            Input::PointerUp { button } => send(self.pointer.up(tree, button), dispatch),
            Input::Wheel { dx, dy } => send(self.pointer.wheel(tree, dx, dy), dispatch),
            Input::KeyDown { key, code, repeat } => {
                let prevented = self.key_event(
                    tree,
                    EventKind::KeyDown,
                    key.clone(),
                    code,
                    repeat,
                    dispatch,
                );
                if key == "Tab" && !prevented {
                    self.keyboard = true;
                    let next = tab_target(tree, self.focused, self.modifiers.shift);
                    if next.is_some() {
                        self.set_focus(tree, next, true, dispatch);
                    }
                }
            }
            Input::KeyUp { key, code } => {
                self.key_event(tree, EventKind::KeyUp, key, code, false, dispatch);
            }
            Input::Modifiers(m) => {
                self.modifiers = m;
                self.pointer.set_modifiers(m);
            }
            Input::WindowBlur => self.pointer.cancel(tree),
            // Like the DOM, focusing an element that can't take focus does nothing.
            Input::Focus(id) => {
                if focusable(tree, id) {
                    let visible = self.keyboard;
                    self.set_focus(tree, Some(id), visible, dispatch);
                }
            }
            Input::Blur(id) => {
                if self.focused == Some(id) {
                    self.set_focus(tree, None, false, dispatch);
                }
            }
            Input::Refresh => {
                send(self.pointer.refresh(tree), dispatch);
                if let Some(id) = self.focused
                    && !focusable(tree, id)
                {
                    self.set_focus(tree, None, false, dispatch);
                }
            }
        }
    }

    fn pointer_focus(
        &mut self,
        tree: &mut UiTree,
        target: Option<NodeId>,
        dispatch: &mut dyn FnMut(&UiEvent) -> bool,
    ) {
        self.keyboard = false;
        self.set_focus(tree, target, false, dispatch);
    }

    /// Sends a key event to the focused element. Returns whether it was prevented.
    fn key_event(
        &mut self,
        tree: &UiTree,
        kind: EventKind,
        key: String,
        code: String,
        repeat: bool,
        dispatch: &mut dyn FnMut(&UiEvent) -> bool,
    ) -> bool {
        let Some(focused) = self.focused else {
            return false;
        };
        let event = UiEvent {
            key,
            code,
            repeat,
            ..UiEvent::new(kind, tree.event_path(focused), self.modifiers)
        };
        dispatch(&event)
    }

    /// Moves focus to `target` (or nowhere), updating `:focus` / `:focus-visible` and
    /// sending `focusout` then `focusin`.
    fn set_focus(
        &mut self,
        tree: &mut UiTree,
        target: Option<NodeId>,
        visible: bool,
        dispatch: &mut dyn FnMut(&UiEvent) -> bool,
    ) {
        let old = self.focused;
        if old == target {
            // Re-focusing by keyboard still shows the ring.
            if let Some(id) = target {
                update_state(tree, id, |s| s.focus_visible = visible);
            }
            return;
        }
        self.focused = target;
        if let Some(id) = old.filter(|id| tree.contains(*id)) {
            update_state(tree, id, |s| {
                s.focus = false;
                s.focus_visible = false;
            });
            dispatch(&UiEvent {
                related: target,
                ..UiEvent::new(EventKind::FocusOut, tree.event_path(id), self.modifiers)
            });
        }
        if let Some(id) = target {
            update_state(tree, id, |s| {
                s.focus = true;
                s.focus_visible = visible;
            });
            dispatch(&UiEvent {
                related: old,
                ..UiEvent::new(EventKind::FocusIn, tree.event_path(id), self.modifiers)
            });
        }
    }
}

fn send(events: Vec<UiEvent>, dispatch: &mut dyn FnMut(&UiEvent) -> bool) {
    for event in &events {
        dispatch(event);
    }
}

/// Whether `id` can take focus: it has a `tabIndex`, isn't disabled and is displayed.
pub fn focusable(tree: &UiTree, id: NodeId) -> bool {
    tree.attrs(id)
        .is_some_and(|a| a.tab_index.is_some() && !a.disabled)
        && tree.is_displayed(id)
}

/// The Tab order: positive `tabIndex` values first (ascending, ties in tree order),
/// then `tabIndex` 0 in tree order. Negative values are skipped.
pub fn tab_order(tree: &UiTree) -> Vec<NodeId> {
    let mut order = Vec::new();
    let mut stack = vec![ROOT];
    while let Some(id) = stack.pop() {
        if id != ROOT {
            if !tree.is_displayed(id) {
                continue;
            }
            if let Some(attrs) = tree.attrs(id)
                && let Some(index) = attrs.tab_index
                && index >= 0
                && !attrs.disabled
            {
                order.push((index, id));
            }
        }
        stack.extend(
            tree.children(id)
                .iter()
                .rev()
                .filter(|c| !tree.is_generated(**c)),
        );
    }
    // Stable, so equal indexes keep tree order; 0 sorts after every positive index.
    order.sort_by_key(|(index, _)| if *index == 0 { i32::MAX } else { *index });
    order.into_iter().map(|(_, id)| id).collect()
}

/// The element Tab (or Shift-Tab with `backwards`) moves to from `from`, wrapping
/// around. From nothing (or an element outside the order) it starts at an end.
fn tab_target(tree: &UiTree, from: Option<NodeId>, backwards: bool) -> Option<NodeId> {
    let order = tab_order(tree);
    let last = order.len().checked_sub(1)?;
    let at = from.and_then(|f| order.iter().position(|id| *id == f));
    let next = match (at, backwards) {
        (Some(i), false) => (i + 1) % order.len(),
        (Some(i), true) => i.checked_sub(1).unwrap_or(last),
        (None, false) => 0,
        (None, true) => last,
    };
    Some(order[next])
}
