//! Prints what `#[iron_api]` generated for the demo namespace: the JSON manifest the compiler reads
//! and the TypeScript declarations.
//!
//! `cd tools && cargo run -p iron_demo_api --example manifest`
fn main() {
    println!(
        "=== manifest (JSON) ===\n{}\n",
        iron_demo_api::manifest_json()
    );
    println!(
        "=== declarations (.d.ts) ===\n{}",
        iron_demo_api::declarations()
    );
}
