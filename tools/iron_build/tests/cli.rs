//! The `iron-build` program itself: which commands print the empty-field warnings.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn game(tag: &str, project: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("iron_build_cli_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("src/main.ts"), "export const a = 1;").unwrap();
    std::fs::write(d.join("project.json"), project).unwrap();
    d
}

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_iron-build"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

const EMPTY_COMPANY: &str = "\"company\" is empty (the exe's version info will have no company)";

// M9-TS-2724
#[test]
fn build_and_package_warn_about_empty_fields_but_check_does_not() {
    let g = game("warn", r#"{ "name": "g", "company": "" }"#);
    let parent = g.parent().unwrap().to_path_buf();
    let games = parent.join(format!("iron_build_cli_games_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&games);
    std::fs::create_dir_all(&games).unwrap();

    let b = run(&["build", g.to_str().unwrap(), "--no-tsc"], &parent);
    assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
    let err = String::from_utf8_lossy(&b.stderr).into_owned();
    assert!(
        err.contains("warning: ") && err.contains("project.json: ") && err.contains(EMPTY_COMPANY),
        "{err}"
    );

    let host = env!("CARGO_BIN_EXE_iron-build");
    let out = g.join("out/g.exe");
    let p = run(
        &[
            "package",
            g.to_str().unwrap(),
            "--no-tsc",
            "--host",
            host,
            "--out",
            out.to_str().unwrap(),
        ],
        &parent,
    );
    let err = String::from_utf8_lossy(&p.stderr).into_owned();
    assert!(p.status.success(), "{err}");
    assert!(err.contains(EMPTY_COMPANY), "{err}");
    assert!(out.is_file());

    // `check` builds the same game and stays quiet about it.
    std::fs::rename(&g, games.join("g")).unwrap();
    let c = run(&["check", games.to_str().unwrap()], &parent);
    let (o, e) = (
        String::from_utf8_lossy(&c.stdout).into_owned(),
        String::from_utf8_lossy(&c.stderr).into_owned(),
    );
    assert!(c.status.success(), "{o}{e}");
    assert!(o.contains("1 built, 0 failed"), "{o}");
    assert!(!o.contains("is empty") && !e.contains("is empty"), "{o}{e}");
    let _ = std::fs::remove_dir_all(&games);
}
