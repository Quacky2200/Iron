//! `Renderer.isVisible` (Unity's): whether a renderer was seen in the last frame drawn, by a
//! camera or as a shadow caster whose shadow can fall in a view. Watching costs the renderer a
//! test per view, so only renderers a script has asked about are watched: the first question
//! starts the watch (and answers false until a frame has been drawn watching it). The host reads
//! [`watched_renderers`] before drawing and reports with [`set_seen_renderers`] after.

use std::cell::RefCell;
use std::collections::HashSet;

/// A renderer by its component's number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RendererId {
    Mesh(u32),
    Skinned(u32),
}

#[derive(Default)]
struct Sight {
    watched: HashSet<RendererId>,
    seen: HashSet<RendererId>,
}

thread_local! {
    static SIGHT: RefCell<Sight> = RefCell::new(Sight::default());
}

/// Whether `r` was seen in the last frame drawn; from now on it is watched.
pub(crate) fn is_visible(r: RendererId) -> bool {
    SIGHT.with(|s| {
        let mut s = s.borrow_mut();
        s.watched.insert(r);
        s.seen.contains(&r)
    })
}

/// The renderers scripts have asked about, for the host to watch while drawing.
pub fn watched_renderers() -> Vec<RendererId> {
    SIGHT.with(|s| s.borrow().watched.iter().copied().collect())
}

/// The host's report on the frame just drawn: the renderers it saw (others were not).
pub fn set_seen_renderers(seen: impl IntoIterator<Item = RendererId>) {
    SIGHT.with(|s| {
        let mut s = s.borrow_mut();
        s.seen.clear();
        s.seen.extend(seen);
    });
}

pub(crate) fn reset_visibility() {
    SIGHT.with(|s| *s.borrow_mut() = Sight::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-CUL-04
    #[test]
    fn a_renderer_is_watched_once_asked_and_visible_as_the_host_last_reported() {
        reset_visibility();
        let (a, b) = (RendererId::Mesh(3), RendererId::Skinned(3));
        assert!(
            watched_renderers().is_empty(),
            "nothing watched until asked"
        );
        assert!(!is_visible(a), "no frame drawn watching it yet");
        assert_eq!(watched_renderers(), [a]);
        set_seen_renderers([a]);
        assert!(is_visible(a));
        assert!(!is_visible(b), "a skinned renderer is numbered apart");
        set_seen_renderers([]);
        assert!(!is_visible(a));
        assert_eq!(watched_renderers().len(), 2);
        reset_visibility();
        assert!(watched_renderers().is_empty());
    }
}
