// M9-TS-1511
//
// The compiler embeds the `Ui` manifest (so JSX needs none from the caller); it must not drift
// from the one generated here. Regenerate:
// `cargo run -p iron_ts_ui --example manifest > iron_ts/src/lower/ui_manifest.json`.
#[test]
fn the_embedded_ui_manifest_equals_the_generated_one() {
    let embedded: serde_json::Value = serde_json::from_str(iron_ts::lower::UI_MANIFEST).unwrap();
    let generated: serde_json::Value = serde_json::from_str(&iron_ts_ui::manifest_json()).unwrap();
    assert_eq!(embedded, generated);
}
