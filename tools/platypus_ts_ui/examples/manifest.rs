// SPDX-License-Identifier: Apache-2.0

//! Prints the `Ui` manifest: `cargo run -p platypus_ts_ui --example manifest > platypus_ts/src/lower/ui_manifest.json`.
fn main() {
    println!("{}", platypus_ts_ui::manifest_json());
}
