//! Script hot reload: watches the bundle (and its CSS) for rebuilds by `bun build
//! --watch` and says when to reload.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// How long the files must stay unchanged before reloading: Bun writes the bundle and
/// the stylesheet separately, and large files in more than one write.
pub const SETTLE: Duration = Duration::from_millis(150);

pub struct BundleWatcher {
    // Kept alive: dropping it stops the watch.
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<notify::Event>>,
    files: Vec<PathBuf>,
    /// When the last change to a watched file was seen, until it's reported.
    changed: Option<Instant>,
}

impl BundleWatcher {
    /// Watches `files` (all in one folder) for changes.
    pub fn new(files: Vec<PathBuf>) -> notify::Result<Self> {
        let dir = files
            .first()
            .and_then(|f| f.parent())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let (tx, events) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        // The folder, not the files: Bun may replace them rather than write in place.
        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        Ok(Self {
            _watcher: watcher,
            events,
            files,
            changed: None,
        })
    }

    /// By name: only the files' own folder is watched.
    fn watched(&self, path: &Path) -> bool {
        self.files.iter().any(|f| f.file_name() == path.file_name())
    }

    /// Whether to reload now: a watched file changed and then settled for [`SETTLE`].
    pub fn poll(&mut self, now: Instant) -> bool {
        while let Ok(event) = self.events.try_recv() {
            let Ok(event) = event else { continue };
            if matches!(event.kind, EventKind::Access(_)) {
                continue;
            }
            if event.paths.iter().any(|p| self.watched(p)) {
                self.changed = Some(now);
            }
        }
        match self.changed {
            Some(t) if now.duration_since(t) >= SETTLE => {
                self.changed = None;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M4-HOT-10
    #[test]
    fn reports_a_settled_change_to_a_watched_file_once() {
        let dir = std::env::temp_dir().join(format!("iron-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let js = dir.join("game.js");
        std::fs::write(&js, "1").unwrap();
        let mut w = BundleWatcher::new(vec![js.clone(), dir.join("game.css")]).unwrap();

        // Another file in the folder doesn't count.
        std::fs::write(dir.join("other.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!w.poll(Instant::now() + SETTLE * 2));

        std::fs::write(&js, "2").unwrap();
        let mut seen = false;
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(20));
            let now = Instant::now();
            // Not before it has settled...
            if w.poll(now) {
                panic!("reported before settling");
            }
            if w.changed.is_some() {
                seen = true;
                break;
            }
        }
        assert!(seen, "no change event");
        // ...then once.
        let later = Instant::now() + SETTLE;
        assert!(w.poll(later));
        assert!(!w.poll(later + SETTLE));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
