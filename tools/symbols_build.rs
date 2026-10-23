// SPDX-License-Identifier: Apache-2.0

// Shared by the `build.rs` of `iron_ts_rt` and `iron_ts_ui` (`include!`): writes `symbols.rs`, a
// function `symbols()` listing every `#[unsafe(no_mangle)] pub ... extern "C" fn` of the crate by
// name and address, so a JIT can resolve compiled code's imports with no hand-kept list.
//
// The scan is textual: an attribute line `#[unsafe(no_mangle)]` (or `#[no_mangle]`) marks the next
// `fn` as exported; the module path comes from the file (`src/a.rs` is `crate::a`, `src/lib.rs` is
// the crate root). Exports must be `pub` and not inside an inline `mod` or `cfg` block.
fn write_symbol_table() {
    use std::fmt::Write as _;
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    let mut out = String::from(
        "/// Every exported `extern \"C\"` function of this crate: name and address.\npub fn symbols() -> Vec<(&'static str, usize)> {\n    vec![\n",
    );
    for f in files {
        println!("cargo:rerun-if-changed={}", f.display());
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let path = if stem == "lib" {
            "crate".to_string()
        } else {
            format!("crate::{stem}")
        };
        let text = std::fs::read_to_string(&f).unwrap();
        let mut marked = false;
        let mut macro_name: Option<String> = None;
        for line in text.lines() {
            let t = line.trim();
            if let Some(r) = t.strip_prefix("macro_rules! ") {
                macro_name = Some(r.trim_end_matches('{').trim().to_string());
            }
            if t.starts_with("#[unsafe(no_mangle)]") || t.starts_with("#[no_mangle]") {
                marked = true;
            } else if marked && !t.starts_with("#[") && !t.starts_with("//") {
                marked = false;
                if t.contains("fn $") {
                    // Inside a `macro_rules!`: the names are in the invocations.
                    let m = macro_name.clone().expect("exported fn in a macro");
                    for inv in text.split(&format!("{m}!(")).skip(1) {
                        let body = inv.split(");").next().unwrap();
                        for w in body.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                            if w.starts_with("ts_rt_") || w.starts_with("ts_ui_") {
                                writeln!(out, "        (\"{w}\", {path}::{w} as *const () as usize),").unwrap();
                            }
                        }
                    }
                } else if let Some(at) = t.find("fn ") {
                    let name: String = t[at + 3..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    // A private export would not be nameable from here: fail loudly.
                    assert!(
                        t.starts_with("pub "),
                        "{}: exported fn `{name}` must be `pub`",
                        f.display()
                    );
                    writeln!(out, "        (\"{name}\", {path}::{name} as *const () as usize),").unwrap();
                }
            }
        }
    }
    out.push_str("    ]\n}\n");
    let dest = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("symbols.rs");
    std::fs::write(dest, out).unwrap();
}
