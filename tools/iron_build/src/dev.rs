//! `iron-build dev`: rebuild when a source, stylesheet or asset changes and tell the running app.
//!
//! The protocol (agreed with the app side, `docs/iron-build.md`): after every build `dist/.iron-reload`
//! is written, *after* `dist/game.irbundle`, holding `{"seq":N,"ok":true}` or, when the build failed (the
//! old `game.irbundle` stays), `{"seq":N,"ok":false,"error":"..."}`. The app watches that one file and
//! reloads the bundle when `seq` changes. The watcher polls modification times (every 200 ms), which
//! needs no dependency and works on every file system.

use crate::{Options, build, write_atomic};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Folders the watcher never looks into.
const SKIP: &[&str] = &["dist", "node_modules", "target", "types", ".git", ".iron"];

/// A number that changes when a watched file is added, removed or modified.
pub fn fingerprint(root: &Path) -> u128 {
    fn walk(dir: &Path, acc: &mut u128, count: &mut u128) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            if p.is_dir() {
                if !SKIP.contains(&name.as_str()) && !name.starts_with('.') {
                    walk(&p, acc, count);
                }
            } else if !name.ends_with(".meta") {
                let t = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_nanos());
                *acc = acc.wrapping_add(t.wrapping_mul(31).wrapping_add(name.len() as u128));
                *count += 1;
            }
        }
    }
    let (mut acc, mut count) = (0u128, 0u128);
    walk(root, &mut acc, &mut count);
    acc.wrapping_mul(1_000_003).wrapping_add(count)
}

/// Writes `dist/.iron-reload` for build number `seq`.
pub fn write_status(dist: &Path, seq: u64, result: &Result<(), String>) -> std::io::Result<()> {
    let v = match result {
        Ok(()) => serde_json::json!({ "seq": seq, "ok": true }),
        Err(e) => serde_json::json!({ "seq": seq, "ok": false, "error": e }),
    };
    write_atomic(&dist.join(".iron-reload"), v.to_string().as_bytes())
}

/// Builds once, then rebuilds on every change until the process is stopped.
pub fn run(root: &Path, opts: &Options) -> Result<(), String> {
    let mut seq = 0u64;
    let mut last = 0u128;
    loop {
        let now = fingerprint(root);
        if now != last {
            // Let an editor finish writing (and a second save follow) before building.
            std::thread::sleep(Duration::from_millis(100));
            last = fingerprint(root);
            seq += 1;
            let r = build(root, opts);
            match &r {
                Ok(rep) => println!(
                    "[{seq}] built {} files in {:?}{}",
                    rep.files.len(),
                    rep.time,
                    if rep.warnings.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", rep.warnings.join("; "))
                    }
                ),
                Err(e) => eprintln!("[{seq}] build failed:\n{e}"),
            }
            write_status(&root.join("dist"), seq, &r.map(|_| ())).map_err(|e| e.to_string())?;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-TS-1860
    #[test]
    fn a_status_file_says_ok_or_carries_the_error() {
        let d = std::env::temp_dir().join(format!("iron_build_dev_{}", crate::assets::new_guid()));
        std::fs::create_dir_all(&d).unwrap();
        write_status(&d, 3, &Ok(())).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.join(".iron-reload")).unwrap())
                .unwrap();
        assert_eq!(
            (v["seq"].as_u64(), v["ok"].as_bool()),
            (Some(3), Some(true))
        );
        write_status(&d, 4, &Err("boom".into())).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.join(".iron-reload")).unwrap())
                .unwrap();
        assert_eq!(
            (v["seq"].as_u64(), v["error"].as_str()),
            (Some(4), Some("boom"))
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-1861
    #[test]
    fn the_fingerprint_changes_with_sources_but_not_with_dist() {
        let d = std::env::temp_dir().join(format!("iron_build_fp_{}", crate::assets::new_guid()));
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::create_dir_all(d.join("dist")).unwrap();
        std::fs::write(d.join("src/a.ts"), "1").unwrap();
        let a = fingerprint(&d);
        std::fs::write(d.join("dist/game.css"), "x").unwrap();
        assert_eq!(fingerprint(&d), a);
        std::fs::write(d.join("src/b.ts"), "2").unwrap();
        assert_ne!(fingerprint(&d), a);
        let _ = std::fs::remove_dir_all(&d);
    }
}
