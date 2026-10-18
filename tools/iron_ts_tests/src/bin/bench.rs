//! `bench [steps]`: step() on a busy state for QuickJS (the engine's runtime), bun (a JIT
//! stand-in) and the native code `iron_ts` generated. All three print a checksum; they should agree.
use iron_core::Value;
use iron_script::{QuickJsRuntime, ScriptRuntime};
use iron_ts_rt::link_anchor;
use iron_ts_tests::sim::*;
use iron_ts_tests::*;
use std::process::Command;
use std::time::Instant;

const W: usize = 1022;
const H: usize = 750;

fn checksum(s: *mut SimState) -> f64 {
    unsafe {
        let st = &*s;
        let p = &*(*st.players).get::<*mut Player>(0);
        let mut sum = st.level * 1000.0
            + p.score
            + p.kills * 7.0
            + arr_len(st.bullets) as f64 * 13.0
            + arr_len(st.zombies) as f64 * 17.0
            + st.seed;
        for i in 0..arr_len(st.zombies) {
            let z = &*(*st.zombies).get::<*mut Zombie>(i);
            sum += z.x + z.y;
        }
        sum
    }
}

fn native(steps: usize) -> (f64, f64, usize) {
    unsafe {
        ts_init();
        let open = build_walls(W, H, &vec![vec![W as f64]; H]);
        let walls = walls_closure(open);
        let state = ts_createState(7.0, 6.0);
        let p = ts_addPlayer(state);
        ts_startLevel(state, walls);
        let gun = alloc::<Gun>();
        (*gun).name = TAG_MP5 as u64;
        (*gun).rounds = 1e9;
        (*gun).clip = 1e9;
        (*gun).damage = 40.0;
        (*gun).auto = 1;
        (*p).gun = gun;
        (*p).health = 1e9;
        (*state).zombiesLeft = 1e9;
        let input = inputs(&[new_input(0.0, 0.0, 900.0, 375.0, true)]);
        let t = Instant::now();
        for _ in 0..steps {
            ts_step(state, input, 1.0 / 60.0, walls);
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        (ms, checksum(state), iron_ts_rt::ts_rt_allocated())
    }
}

fn quickjs(steps: usize, bundle: &str) -> (f64, f64) {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("bench", bundle).expect("bundle");
    let t = Instant::now();
    let v = rt
        .call_global("__bench", &[Value::Number(steps as f64)])
        .expect("run");
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let Value::Number(sum) = v else {
        panic!("checksum: {v:?}")
    };
    (ms, sum)
}

fn bun(steps: usize, dir: &std::path::Path) -> Option<(f64, f64)> {
    let out = Command::new("bun")
        .arg(dir.join("bench/bench.ts"))
        .arg(steps.to_string())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v: serde_json_lite::Json = serde_json_lite::parse(text.lines().last()?)?;
    Some((v.get("ms")?, v.get("checksum")?))
}

/// Just enough JSON for `{"ms":1.5,"checksum":2}`.
mod serde_json_lite {
    pub struct Json(Vec<(String, f64)>);
    impl Json {
        pub fn get(&self, k: &str) -> Option<f64> {
            self.0.iter().find(|(n, _)| n == k).map(|(_, v)| *v)
        }
    }
    pub fn parse(s: &str) -> Option<Json> {
        let mut out = Vec::new();
        for part in s.trim().trim_matches(|c| c == '{' || c == '}').split(',') {
            let (k, v) = part.split_once(':')?;
            if let Ok(n) = v.trim().parse::<f64>() {
                out.push((k.trim().trim_matches('"').to_string(), n));
            }
        }
        Some(Json(out))
    }
}

fn main() {
    assert_ne!(link_anchor(), 0);
    let steps: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle_path = std::env::temp_dir().join("iron_ts_bench_bundle.js");
    let st = Command::new("bun")
        .args(["build"])
        .arg(dir.join("bench/bench.ts"))
        .args(["--format=iife", "--target=browser", "--outfile"])
        .arg(&bundle_path)
        .status()
        .expect("bun build");
    assert!(st.success());
    let bundle = std::fs::read_to_string(&bundle_path).unwrap();

    println!("step() x {steps}, 1 player (MP5 auto-fire), ~10 zombies, bullets in flight");
    // Best of three: the first run pays for page faults and (for bun) JIT warm-up.
    let best = |name: &str, f: &mut dyn FnMut() -> Option<(f64, f64)>| {
        let mut runs = Vec::new();
        for _ in 0..3 {
            if let Some(r) = f() {
                runs.push(r);
            }
        }
        if runs.is_empty() {
            println!("{name:>8}: unavailable");
            return;
        }
        let (ms, sum) = runs
            .iter()
            .cloned()
            .fold((f64::MAX, 0.0), |a, r| if r.0 < a.0 { r } else { a });
        println!(
            "{name:>8}: {ms:9.2} ms total  {:8.3} us/step  checksum {sum:.4}",
            ms * 1000.0 / steps as f64
        );
    };
    best("quickjs", &mut || Some(quickjs(steps, &bundle)));
    best("bun", &mut || bun(steps, dir));
    best("native", &mut || {
        let (ms, sum, _) = native(steps);
        Some((ms, sum))
    });
    let (_, _, bytes) = native(steps);
    println!(
        "arena used by one native run: {:.1} MiB",
        bytes as f64 / (1 << 20) as f64
    );
}
