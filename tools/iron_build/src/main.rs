//! `iron-build build|types|assets|dev|check`: see `docs/iron-build.md`.

use iron_build::{Options, Tsc, assets, build, dev, package, types_dts};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage:
  iron-build build <game> [--no-tsc] [--aot] [--cook]   build <game>/dist
  iron-build types [<game>] [--out file]                write types/iron.d.ts (default <game>/types/iron.d.ts)
  iron-build assets <folder>... [--prune]               write .meta files
  iron-build dev <game> [--no-tsc]                      rebuild on change, signal the app (dist/.iron-reload)
  iron-build package <game> [--host exe] [--out f] ...   one executable (see: iron-build package --help)
  iron-build check <examples folder> [--tsc] [--full]   build every game in it and print a table";

fn options(args: &[String]) -> Options {
    Options {
        no_tsc: args.iter().any(|a| a == "--no-tsc")
            || (args.first().is_some_and(|c| c == "check") && !args.iter().any(|a| a == "--tsc")),
        aot: args.iter().any(|a| a == "--aot"),
        cook: args.iter().any(|a| a == "--cook"),
        no_compile: args.iter().any(|a| a == "--no-compile"),
    }
}

fn positional(args: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in &args[1..] {
        if skip {
            skip = false;
        } else if a == "--out" {
            skip = true;
        } else if !a.starts_with("--") {
            out.push(a);
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let pos = positional(&args);
    let code = match cmd.as_str() {
        "build" => {
            let game = pos.first().map_or(".", |g| g.as_str());
            match build(Path::new(game), &options(&args)) {
                Ok(r) => {
                    println!(
                        "{}: {} file(s), css {}, {} asset(s), object {} bytes, {:?}",
                        r.config.name,
                        r.files.len(),
                        r.css_files.len(),
                        r.assets.len(),
                        r.object_bytes,
                        r.time
                    );
                    match r.tsc {
                        Tsc::Missing => eprintln!(
                            "note: tsc not found; the type check was skipped (install TypeScript to check types)"
                        ),
                        Tsc::Ok => println!("tsc: ok"),
                        _ => {}
                    }
                    for w in r.warnings {
                        eprintln!("warning: {w}");
                    }
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        "types" => {
            let out = args
                .iter()
                .position(|a| a == "--out")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    Path::new(pos.first().map_or(".", |g| g.as_str())).join("types/iron.d.ts")
                });
            match types_dts() {
                Ok(t) => {
                    if let Some(d) = out.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    match std::fs::write(&out, t) {
                        Ok(()) => {
                            println!("wrote {}", out.display());
                            println!(
                                "editors: add \"types\" to the \"include\" of tsconfig.json (and \"moduleDetection\": \"force\"); `iron-build build` needs neither"
                            );
                            0
                        }
                        Err(e) => {
                            eprintln!("error: {}: {e}", out.display());
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        "assets" => {
            let prune = args.iter().any(|a| a == "--prune");
            let folders: Vec<&str> = if pos.is_empty() {
                vec!["."]
            } else {
                pos.iter().map(|s| s.as_str()).collect()
            };
            for f in folders {
                match assets::update(Path::new(f), prune) {
                    Ok(r) => {
                        for c in r.created {
                            println!("{f}: + {c}.meta");
                        }
                        for (a, b) in r.moved {
                            println!("{f}: {a}.meta -> {b}.meta");
                        }
                        for o in r.orphans {
                            println!(
                                "{f}: {} {o}",
                                if prune {
                                    "deleted"
                                } else {
                                    "orphaned (its asset is gone; --prune deletes)"
                                }
                            );
                        }
                    }
                    Err(e) => eprintln!("error: {f}: {e}"),
                }
            }
            0
        }
        "package" => package::run_cli(&args[1..]),
        "dev" => {
            let game = pos.first().map_or(".", |g| g.as_str());
            match dev::run(Path::new(game), &options(&args)) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        "check" => check(
            Path::new(pos.first().map_or("examples", |g| g.as_str())),
            &options(&args),
        ),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}

/// Builds every game folder in `dir` and prints one row each; exit code 1 when one failed.
fn check(dir: &Path, opts: &Options) -> i32 {
    let mut games: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("package.json").exists())
                .collect()
        })
        .unwrap_or_default();
    games.sort();
    if games.is_empty() {
        eprintln!("no games in {}", dir.display());
        return 2;
    }
    println!(
        "{:<20} {:<6} {:>5} {:>4} {:>6}  first error",
        "game", "result", "files", "css", "assets"
    );
    let (mut ok, mut failed) = (0, 0);
    for g in games {
        let name = g
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        match build(&g, opts) {
            Ok(r) => {
                ok += 1;
                println!(
                    "{name:<20} {:<6} {:>5} {:>4} {:>6}",
                    "ok",
                    r.files.len(),
                    r.css_files.len(),
                    r.assets.len()
                );
            }
            Err(e) => {
                failed += 1;
                let first = e
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(110)
                    .collect::<String>();
                println!(
                    "{name:<20} {:<6} {:>5} {:>4} {:>6}  {first}",
                    "FAIL", "-", "-", "-"
                );
                if std::env::args().any(|a| a == "--full") {
                    for l in e.lines().skip(1) {
                        println!("    {l}");
                    }
                }
            }
        }
    }
    println!("{ok} built, {failed} failed");
    i32::from(failed > 0)
}
