//! Prints the `Ui` manifest: `cargo run -p iron_ts_ui --example manifest > iron_ts/src/lower/ui_manifest.json`.
fn main() {
    println!("{}", iron_ts_ui::manifest_json());
}
