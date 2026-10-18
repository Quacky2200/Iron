//! `sine_bench [frames]`: one frame of the sine-circle demo is the curve (361 points a degree,
//! Catmull-Rom smoothed to 1,083 points) flattened to the list `render.stroke` takes. Compared:
//! QuickJS computing it, QuickJS computing it and sending it to Rust through `__iron_op`
//! (the old way), bun, and the native code `iron_ts` generated (computing it, then Rust reading
//! the array in place).
use iron_core::Value;
use iron_script::{QuickJsRuntime, ScriptRuntime};
use iron_ts_rt::{Array, link_anchor, ts_rt_reset};
use iron_ts_tests::sine::*;
use std::process::Command;
use std::time::Instant;

fn quickjs(frames: usize, bundle: &str, send: bool) -> (f64, f64) {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("sine", bundle).expect("bundle");
    let t = Instant::now();
    let v = rt
        .call_global("__sine", &[Value::Number(frames as f64), Value::Bool(send)])
        .expect("run");
    // The ops are queued per frame; moving them out is part of the crossing.
    let ops = rt.drain_ops();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    if send {
        assert_eq!(ops.len(), frames, "one render.stroke op per frame");
    }
    let Value::Number(sum) = v else {
        panic!("{v:?}")
    };
    (ms, sum)
}

fn bun(frames: usize, dir: &std::path::Path) -> Option<(f64, f64)> {
    let out = Command::new("bun")
        .arg(dir.join("bench/sine_bench.ts"))
        .arg(frames.to_string())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let line = text.lines().last()?;
    let num = |k: &str| -> Option<f64> {
        let i = line.find(&format!("\"{k}\":"))? + k.len() + 3;
        line[i..].split([',', '}']).next()?.parse().ok()
    };
    Some((num("ms")?, num("checksum")?))
}

fn native(frames: usize, read_in_place: bool) -> (f64, f64) {
    let tags = [TAG_pi, TAG_normal, TAG_doubleSine, TAG_doubleCos];
    let mut sum = 0.0;
    let mut engine_side: Vec<f32> = Vec::new();
    let t = Instant::now();
    for f in 0..frames {
        let (ty, r, fr, d, rot, cx, cy) = (
            tags[(f / 50) % 4],
            100 + f % 120,
            10 + f % 70,
            5 + f % 60,
            (f * 3) % 360,
            504 + f % 40,
            364 + f % 30,
        );
        let flat: *mut Array = unsafe {
            ts_sine_curveFlat(
                ty, r as f64, fr as f64, d as f64, rot as f64, cx as f64, cy as f64,
            )
        };
        let n = unsafe { (*flat).len() };
        let slice = unsafe { std::slice::from_raw_parts((*flat).data as *const f64, n) };
        if read_in_place {
            // What the engine would do: read straight from script memory into its own buffer.
            engine_side.clear();
            engine_side.extend(slice.iter().map(|v| *v as f32));
        }
        sum += slice[n - 2] + slice[n - 1] + n as f64;
        // A frame arena: everything the frame allocated is garbage once Rust has copied the points.
        ts_rt_reset();
    }
    (t.elapsed().as_secs_f64() * 1000.0, sum)
}

fn main() {
    assert_ne!(link_anchor(), 0);
    unsafe { ts_sine_init() };
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle_path = std::env::temp_dir().join("iron_ts_sine_bundle.js");
    let st = Command::new("bun")
        .arg("build")
        .arg(dir.join("bench/sine_bench.ts"))
        .args(["--format=iife", "--target=browser", "--outfile"])
        .arg(&bundle_path)
        .status()
        .expect("bun build");
    assert!(st.success());
    let bundle = std::fs::read_to_string(&bundle_path).unwrap();
    println!(
        "{frames} frames of the sine circle: 361 points a degree, 1,083 after the spline, 2,166 numbers sent"
    );
    let best = |name: &str, f: &mut dyn FnMut() -> Option<(f64, f64)>| {
        let runs: Vec<_> = (0..3).filter_map(|_| f()).collect();
        let Some(&(ms, sum)) = runs.iter().min_by(|a, b| a.0.total_cmp(&b.0)) else {
            println!("{name:>26}: unavailable");
            return;
        };
        println!(
            "{name:>26}: {ms:9.2} ms total  {:8.3} ms/frame  checksum {sum:.4}",
            ms / frames as f64
        );
    };
    best("quickjs, compute", &mut || {
        Some(quickjs(frames, &bundle, false))
    });
    best("quickjs, compute + op", &mut || {
        Some(quickjs(frames, &bundle, true))
    });
    best("bun, compute", &mut || bun(frames, dir));
    best("native, compute", &mut || Some(native(frames, false)));
    best("native, compute + copy", &mut || Some(native(frames, true)));
}
