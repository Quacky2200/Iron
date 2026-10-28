// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `platypus-build bench <examples>`: every example timed by `platypus-app --bench` (windowless); the
//! results go to `docs/benchmarks.md` and `docs/benchmarks/results.json` (`docs/platypus-build.md`,
//! "Benchmarks").

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const USAGE: &str = "usage: platypus-build bench <examples folder> [--only a,b,..] [--host platypus-app.exe]
  [--no-host-build] [--frames N] [--out docs-folder] [--timeout seconds]
Builds the release host (cargo build --release -p platypus_app --bin platypus-app) and every example, then runs each
with `platypus-app <game> --bench` (no window). Scripted input and frame counts: tools/bench/inputs/<name>.json.";

/// One frame step for every run: games see the same time whatever their speed.
const FIXED_DT: &str = "0.016666666666666666";
const SIZE: &str = "1280x720";

struct Options {
    examples: PathBuf,
    only: Option<Vec<String>>,
    host: Option<PathBuf>,
    build_host: bool,
    frames: Option<u32>,
    out: Option<PathBuf>,
    timeout: Option<u64>,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        examples: PathBuf::new(),
        only: None,
        host: None,
        build_host: true,
        frames: None,
        out: None,
        timeout: None,
    };
    let mut it = args.iter();
    let mut examples = None;
    while let Some(a) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--only" => o.only = Some(value()?.split(',').map(str::to_owned).collect()),
            "--host" => o.host = Some(value()?.into()),
            "--out" => o.out = Some(value()?.into()),
            "--frames" => o.frames = Some(value()?.parse().map_err(|_| "--frames needs a number")?),
            "--timeout" => {
                o.timeout = Some(value()?.parse().map_err(|_| "--timeout needs seconds")?)
            }
            "--no-host-build" => o.build_host = false,
            "--help" | "-h" => return Err(USAGE.into()),
            s if !s.starts_with("--") && examples.is_none() => examples = Some(PathBuf::from(s)),
            _ => return Err(format!("unexpected argument {a}\n{USAGE}")),
        }
    }
    o.examples = examples.ok_or(USAGE)?;
    Ok(o)
}

pub fn run_cli(args: &[String]) -> i32 {
    match parse(args).and_then(|o| run(&o)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// A program's output, or why it failed (its last lines).
fn run_tool(cmd: &mut Command, what: &str) -> Result<(), String> {
    let out = cmd.output().map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let text =
        String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    Err(format!("{what} failed: {}", tail(&text, 4)))
}

fn tail(text: &str, lines: usize) -> String {
    let v: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    strip_ansi(&v[v.len().saturating_sub(lines)..].join(" | "))
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        match (esc, c) {
            (false, '\u{1b}') => esc = true,
            (true, c) if c.is_ascii_alphabetic() => esc = false,
            (true, _) => {}
            (false, c) => out.push(c),
        }
    }
    out
}

/// What one game did on one backend.
struct Outcome {
    report: Option<Value>,
    error: Option<String>,
}

fn run(o: &Options) -> Result<(), String> {
    // Not `canonicalize`: Windows' `\\?\` paths are not ones the build reads.
    let examples =
        std::path::absolute(&o.examples).map_err(|e| format!("{}: {e}", o.examples.display()))?;
    let repo = PathBuf::from(git(&examples, &["rev-parse", "--show-toplevel"])?);
    let inputs = repo.join("tools/bench/inputs");
    let out_dir = o.out.clone().unwrap_or_else(|| repo.join("docs"));
    let mut games: Vec<PathBuf> = crate::find_games(&examples);
    if let Some(only) = &o.only {
        games.retain(|g| {
            only.iter()
                .any(|n| g.file_name().is_some_and(|f| f == n.as_str()))
        });
    }
    if games.is_empty() {
        return Err(format!("no games in {}", examples.display()));
    }

    // The host, release only.
    if o.build_host {
        eprintln!("bench: cargo build --release -p platypus_app --bin platypus-app");
        run_tool(
            Command::new("cargo")
                .args([
                    "build",
                    "--release",
                    "-p",
                    "platypus_app",
                    "--bin",
                    "platypus-app",
                ])
                .env("CARGO_INCREMENTAL", "0")
                .current_dir(&repo),
            "cargo build --release -p platypus_app --bin platypus-app",
        )?;
    }
    let host = o
        .host
        .clone()
        .unwrap_or_else(|| repo.join("target/release").join(exe("platypus-app")));
    if !host.is_file() {
        return Err(format!(
            "{} not found (build it, or pass --host)",
            host.display()
        ));
    }

    let head_commit = git(&repo, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();

    let mut results = Vec::new();
    for g in &games {
        let name = g.file_name().unwrap().to_string_lossy().to_string();
        let input = inputs.join(format!("{name}.json"));
        let input = input.is_file().then_some(input);
        let spec: Value = input
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);

        eprintln!("bench: {name}: native build");
        let native = match crate::build(
            g,
            &crate::Options {
                no_tsc: true,
                ..Default::default()
            },
        ) {
            Ok(_) => bench_one(&host, g, input.as_deref(), o),
            Err(e) => Outcome {
                report: None,
                error: Some(format!("build failed: {}", tail(&e, 2))),
            },
        };
        match (&native.report, &native.error) {
            (Some(v), _) => eprintln!(
                "bench: {name}: {} fps, median {} ms",
                v["total"]["fps"], v["total"]["medianMs"]
            ),
            (None, Some(e)) => eprintln!("bench: {name}: {e}"),
            _ => {}
        }
        results.push(json!({
            "game": name,
            "note": spec["note"],
            "native": native.report,
            "nativeError": native.error,
        }));
    }
    let raw = out_dir.join("benchmarks");
    let json_path = raw.join("results.json");
    // `--only` re-runs some games: the others keep their last results.
    if o.only.is_some()
        && let Some(old) = std::fs::read_to_string(&json_path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        let mut merged: Vec<Value> = old["results"].as_array().cloned().unwrap_or_default();
        for r in results {
            match merged.iter_mut().find(|m| m["game"] == r["game"]) {
                Some(m) => *m = r,
                None => merged.push(r),
            }
        }
        merged.sort_by(|a, b| a["game"].as_str().cmp(&b["game"].as_str()));
        results = merged;
    }
    let machine = machine_info(&results);
    let all = json!({
        "head": head_commit,
        "fixedDt": FIXED_DT.parse::<f64>().unwrap_or(0.0),
        "size": SIZE,
        "machine": machine,
        "results": results,
    });
    std::fs::create_dir_all(&raw).map_err(|e| e.to_string())?;
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&all).unwrap() + "\n",
    )
    .map_err(|e| format!("{}: {e}", json_path.display()))?;
    let md_path = out_dir.join("benchmarks.md");
    let old = std::fs::read_to_string(&md_path).unwrap_or_default();
    std::fs::write(&md_path, merge_markdown(&old, &markdown(&all)))
        .map_err(|e| format!("{}: {e}", md_path.display()))?;
    eprintln!(
        "bench: wrote {} and {}",
        md_path.display(),
        json_path.display()
    );
    Ok(())
}

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// One `platypus-app --bench` run, bounded in time.
fn bench_one(host: &Path, game: &Path, input: Option<&Path>, o: &Options) -> Outcome {
    let out = std::env::temp_dir().join(format!("platypus-bench-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let mut cmd = Command::new(host);
    cmd.arg(game);
    cmd.args([
        "--bench",
        "--fixed-dt",
        FIXED_DT,
        "--size",
        SIZE,
        "--bench-out",
    ])
    .arg(&out);
    match (input, o.frames) {
        (Some(i), None) => {
            cmd.arg("--bench-input").arg(i);
        }
        (_, Some(n)) => {
            cmd.args(["--warmup", "30", "--frames", &n.to_string()]);
        }
        (None, None) => {}
    }
    let stderr = std::env::temp_dir().join(format!("platypus-bench-{}.log", std::process::id()));
    let log = std::fs::File::create(&stderr).ok();
    cmd.stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(log.map_or(Stdio::null(), Stdio::from))
        .env("RUST_LOG", "warn");
    let limit = Duration::from_secs(o.timeout.unwrap_or(600));
    let started = Instant::now();
    let result = match cmd.spawn() {
        Err(e) => Err(format!("could not start {}: {e}", host.display())),
        Ok(mut child) => loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if started.elapsed() > limit => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!("timed out after {} s", limit.as_secs()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => break Err(e.to_string()),
            }
        },
    };
    let log_text = std::fs::read_to_string(&stderr).unwrap_or_default();
    let _ = std::fs::remove_file(&stderr);
    let report = std::fs::read_to_string(&out)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let _ = std::fs::remove_file(&out);
    let errors = log_errors(&log_text);
    match (result, report) {
        (Ok(_), Some(mut r)) => {
            if !errors.is_empty() {
                r["errors"] = json!(errors);
            }
            Outcome {
                report: Some(r),
                error: None,
            }
        }
        (Ok(status), None) => Outcome {
            report: None,
            error: Some(format!(
                "exited ({status}) without a report: {}",
                errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| tail(&log_text, 2))
            )),
        },
        (Err(e), _) => Outcome {
            report: None,
            error: Some(e),
        },
    }
}

/// The first few distinct errors a run logged (a script failing every frame says so once).
fn log_errors(log: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in log.lines().map(strip_ansi) {
        if !l.contains("ERROR") {
            continue;
        }
        let msg = l.split_once("ERROR").map_or(l.as_str(), |(_, m)| m).trim();
        let msg: String = msg.chars().take(160).collect();
        if !out.contains(&msg) && out.len() < 3 {
            out.push(msg);
        }
    }
    out
}

/// CPU, memory and OS from the system, the GPU from the runs.
fn machine_info(results: &[Value]) -> Value {
    let ps = |q: &str| -> String {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", q])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default()
    };
    let (cpu, ram, os) = if cfg!(windows) {
        (
            ps("(Get-CimInstance Win32_Processor | Select-Object -First 1).Name"),
            ps("[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)"),
            ps("$o = Get-CimInstance Win32_OperatingSystem; \"$($o.Caption) $($o.Version)\""),
        )
    } else {
        (
            String::new(),
            String::new(),
            std::env::consts::OS.to_owned(),
        )
    };
    let gpu = results
        .iter()
        .find_map(|r| r["native"]["adapter"].as_str())
        .unwrap_or("unknown");
    json!({ "cpu": cpu, "ramGb": ram, "os": os, "gpu": gpu })
}

fn f(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn num(v: &Value, digits: usize) -> String {
    f(v).map_or_else(|| "-".into(), |x| format!("{x:.digits$}"))
}

/// The generated part of `docs/benchmarks.md`.
pub fn markdown(all: &Value) -> String {
    let m = &all["machine"];
    let mut s = String::new();
    s += &format!(
        "Machine: {} ({} GB RAM), {}, {}.\n\nCommit {}; every run {} px, a fixed step of 1/60 s per frame \
         (`--fixed-dt`), no window, no vsync or frame cap, release host.\n\n",
        m["cpu"].as_str().unwrap_or("?"),
        m["ramGb"].as_str().unwrap_or("?"),
        m["gpu"].as_str().unwrap_or("?"),
        m["os"].as_str().unwrap_or("?"),
        all["head"].as_str().unwrap_or("?"),
        all["size"].as_str().unwrap_or("?"),
    );
    s += "## Summary\n\n| Example | fps | median / p99 ms | script / sync ms |\n|---|---:|---:|---:|\n";
    let empty = Value::Null;
    let results = all["results"].as_array().map_or(&[][..], Vec::as_slice);
    for r in results {
        let n = r["native"].get("total").unwrap_or(&empty);
        let cell = |v: String| {
            if r["native"].is_null() {
                short_error(&r["nativeError"])
            } else {
                v
            }
        };
        s += &format!(
            "| [{g}](#{g}) | {} | {} | {} |\n",
            cell(num(&n["fps"], 0)),
            cell(format!(
                "{} / {}",
                num(&n["medianMs"], 2),
                num(&n["p99Ms"], 2)
            )),
            cell(format!(
                "{} / {}",
                num(&n["scriptMs"], 2),
                num(&n["syncMs"], 2)
            )),
            g = r["game"].as_str().unwrap_or("?"),
        );
    }
    s += "\n## Per example\n";
    for r in results {
        let g = r["game"].as_str().unwrap_or("?");
        s += &format!("\n### {g}\n\n");
        if let Some(note) = r["note"].as_str() {
            s += &format!("Input: {note}.\n\n");
        }
        let v = &r["native"];
        if v.is_null() {
            s += &format!("{}\n", short_error(&r["nativeError"]));
            continue;
        }
        s += "| fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |\n|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        let t = &v["total"];
        s += &format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} ms | {} |\n",
            num(&t["fps"], 0),
            num(&t["meanMs"], 2),
            num(&t["medianMs"], 2),
            num(&t["p95Ms"], 2),
            num(&t["p99Ms"], 2),
            num(&t["worstMs"], 2),
            num(&t["scriptMs"], 2),
            num(&t["syncMs"], 2),
            num(&t["renderMs"], 2),
            num(&t["otherMs"], 2),
            num(&v["startupMs"], 0),
            num(&v["peakMemoryMb"], 0),
        );
        if let Some(errs) = v["errors"].as_array() {
            let list: Vec<&str> = errs.iter().filter_map(Value::as_str).collect();
            s += &format!(
                "\nLogged errors: `{}`\n",
                list.join("`; `").replace('|', "/")
            );
        }
        if let Some(segs) = v["segments"].as_array().filter(|a| a.len() > 1) {
            s +=
                "\n| Segment | frames | fps | median / p99 | script |\n|---|---:|---:|---:|---:|\n";
            for n in segs {
                s += &format!(
                    "| {} | {} | {} | {} / {} | {} |\n",
                    n["name"].as_str().unwrap_or("?"),
                    num(&n["frames"], 0),
                    num(&n["fps"], 0),
                    num(&n["medianMs"], 2),
                    num(&n["p99Ms"], 2),
                    num(&n["scriptMs"], 2),
                );
            }
        }
    }
    s
}

fn short_error(e: &Value) -> String {
    let e = e.as_str().unwrap_or("not run");
    let e = e.replace('|', "/");
    if e.starts_with("n/a") {
        "n/a".into()
    } else {
        format!("failed: {}", e.chars().take(60).collect::<String>())
    }
}

const BEGIN: &str = "<!-- bench:begin (written by `platypus-build bench`) -->";
const END: &str = "<!-- bench:end -->";

/// Replaces the generated part of the page, keeping what is written around it.
pub fn merge_markdown(old: &str, generated: &str) -> String {
    if let (Some(a), Some(b)) = (old.find(BEGIN), old.find(END))
        && a < b
    {
        return format!("{}{BEGIN}\n{generated}{}", &old[..a], &old[b..]);
    }
    format!("# Benchmarks\n\n{BEGIN}\n{generated}{END}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-BEN-02
    #[test]
    fn the_page_keeps_hand_written_text_around_the_generated_part() {
        let first = merge_markdown("", "one\n");
        assert!(first.contains("one") && first.contains(BEGIN) && first.contains(END));
        let edited =
            first.replace("# Benchmarks\n", "# Benchmarks\n\nNotes by hand.\n") + "\nAfter.\n";
        let again = merge_markdown(&edited, "two\n");
        assert!(again.contains("Notes by hand.") && again.contains("After."));
        assert!(again.contains("two") && !again.contains("one"));
    }

    // M9-BEN-02
    #[test]
    fn the_table_shows_each_game_and_marks_failed_ones() {
        let all = json!({
            "machine": { "cpu": "CPU", "ramGb": "32", "os": "OS", "gpu": "GPU" },
            "head": "abc", "size": "1280x720",
            "results": [
                { "game": "clock",
                  "native": { "total": { "fps": 600.0, "medianMs": 1.5, "p99Ms": 3.0 } } },
                { "game": "launcher", "native": null, "nativeError": "build failed: x" }
            ]
        });
        let md = markdown(&all);
        assert!(
            md.contains("| [clock](#clock) | 600 | 1.50 / 3.00 |"),
            "{md}"
        );
        assert!(
            md.contains("| [launcher](#launcher) | failed: build failed: x |"),
            "{md}"
        );
    }
}
