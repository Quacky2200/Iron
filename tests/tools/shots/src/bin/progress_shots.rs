// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `progress-shots [name ...] [--no-host-build] [--no-build]`: the picture of every example for
//! `docs/PROGRESS.md`. Each is a windowless bench run of the release host (`--bench`: an offscreen
//! target, no OS window, a fixed step of 1/60 s, so the game's clock reaches the same moment on any
//! machine) that saves its last frame, then is shrunk into `docs/progress/<name>.jpg` (3D scenes) or
//! `.png` (UI). The full-size renders stay in `tests/output/progress/` (git-ignored).

use std::path::Path;
use std::process::{Command, Stdio};

use image::imageops::FilterType;

const USAGE: &str = "usage: progress-shots [example ...] [--no-host-build] [--no-build]
(cd tools && cargo run -p platypus_shots --bin progress-shots -- [args])";

/// Frames before the picture (60 a second); 120 unless an effect needs longer to settle.
fn frames(name: &str) -> u32 {
    match name {
        // The shutters open 10 s in, then the eye adapts.
        "dark-room" => 900,
        // The car is inside the tunnel.
        "tunnel" => 420,
        // The doors have opened.
        "godrays-lift" => 360,
        // The camera is in the lamplit hall.
        "light-units" => 600,
        "crowd" => 240,
        "emoji" | "fishtank" => 300,
        n if n.starts_with("fog") || n.starts_with("godrays-") => 600,
        _ => 120,
    }
}

/// UI examples keep sharp edges as PNG; the rest are JPEG.
const PNG: &[&str] = &[
    "hello",
    "boxes",
    "text",
    "css",
    "form",
    "windows",
    "launcher",
    "md-reader",
    "shapes",
    "physics",
];
/// Showpieces get 800 px; the rest 640.
const WIDE: &[&str] = &[
    "crowd",
    "fog-park",
    "fog-world",
    "godrays-forest",
    "godrays-sunset",
    "godrays-clouds",
];
/// Examples that draw nothing worth a picture (hello only logs).
const SKIP: &[&str] = &["hello"];

/// JPEG quality of the shrunk pictures.
const QUALITY: u8 = 85;

/// Resizes `src` to `width` (bicubic, keeping the aspect) and saves it as `dst`'s type.
fn shrink(src: &Path, dst: &Path, width: u32) -> Result<(), String> {
    let img = image::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let height = ((f64::from(img.height()) * f64::from(width) / f64::from(img.width())).round()
        as u32)
        .max(1);
    let small = img.resize_exact(width, height, FilterType::CatmullRom);
    if dst.extension().is_some_and(|e| e == "jpg") {
        let file = std::fs::File::create(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(
            std::io::BufWriter::new(file),
            QUALITY,
        );
        enc.encode_image(&small.to_rgb8())
            .map_err(|e| format!("{}: {e}", dst.display()))
    } else {
        small
            .to_rgba8()
            .save(dst)
            .map_err(|e| format!("{}: {e}", dst.display()))
    }
}

fn main() {
    let args = platypus_shots::Args::parse(USAGE);
    let root = platypus_shots::repo_root();
    let host = match platypus_shots::release_host(&root, args.build_host) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let raw = root.join("tests").join("output").join("progress");
    let out = root.join("docs").join("progress");
    for d in [&raw, &out] {
        if let Err(e) = std::fs::create_dir_all(d) {
            eprintln!("error: {}: {e}", d.display());
            std::process::exit(1);
        }
    }
    let mut failed = 0;
    for (name, game) in args.games(&root, SKIP) {
        if let Err(e) = platypus_shots::build_game(&game, args.build_games) {
            println!("failed to build {name}: {e}");
            failed += 1;
            continue;
        }
        let frames = frames(&name);
        let dir = raw.join(&name);
        let _ = std::fs::remove_dir_all(&dir);
        let ok = Command::new(&host)
            .arg(&game)
            .args(["--bench", "--warmup"])
            .arg((frames - 1).to_string())
            .args(["--frames", "1", "--fixed-dt"])
            .arg((1.0f64 / 60.0).to_string())
            .args(["--size", "1280x720", "--bench-shots"])
            .arg(&dir)
            .arg("--bench-out")
            .arg(raw.join(format!("{name}.json")))
            .current_dir(&root)
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        let shot = std::fs::read_dir(&dir).ok().and_then(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|e| e == "png"))
        });
        let Some(shot) = shot.filter(|_| ok) else {
            println!("failed: {name}");
            failed += 1;
            continue;
        };
        let ext = if PNG.contains(&name.as_str()) {
            "png"
        } else {
            "jpg"
        };
        let dst = out.join(format!("{name}.{ext}"));
        let width = if WIDE.contains(&name.as_str()) {
            800
        } else {
            640
        };
        match shrink(&shot, &dst, width) {
            Ok(()) => println!("wrote {} ({frames} frames)", dst.display()),
            Err(e) => {
                println!("failed to shrink {name}: {e}");
                failed += 1;
            }
        }
    }
    std::process::exit(i32::from(failed > 0));
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-SHOT-01
    #[test]
    fn effects_that_settle_slowly_get_more_frames() {
        assert_eq!(frames("css"), 120);
        assert_eq!(frames("dark-room"), 900);
        assert_eq!(frames("godrays-lift"), 360, "before the godrays rule");
        assert_eq!(frames("godrays-forest"), 600);
        assert_eq!((frames("fog"), frames("fog-white-room")), (600, 600));
    }

    // M9-SHOT-02
    #[test]
    fn a_render_is_shrunk_to_the_width_keeping_its_aspect_as_jpeg_or_png() {
        let dir = std::env::temp_dir().join(format!("platypus-shots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("frame.png");
        image::RgbaImage::from_pixel(1280, 720, image::Rgba([200, 40, 40, 255]))
            .save(&src)
            .unwrap();
        for (name, width) in [("a.jpg", 640), ("b.png", 800)] {
            let dst = dir.join(name);
            shrink(&src, &dst, width).unwrap();
            let img = image::open(&dst).unwrap();
            assert_eq!((img.width(), img.height()), (width, width * 9 / 16));
            let px = img.to_rgb8().get_pixel(10, 10).0;
            assert!(px[0].abs_diff(200) < 4 && px[1].abs_diff(40) < 4, "{px:?}");
        }
        let jpeg = std::fs::read(dir.join("a.jpg")).unwrap();
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "a JPEG file");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
