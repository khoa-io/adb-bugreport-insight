//! Fixture diagnostic: prints how each entry in a built fixture is detected.
//!
//! Not a test — run by hand when a fixture's expected content types change:
//! `cargo run --offline -p bugreport-insight-mcp --example detect_fixture -- <zip>`

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/test-fixtures/good.zip".to_string());
    let archive = match bugreport_insight_core::Archive::open(&path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("cannot open {path}: {e}");
            std::process::exit(1);
        }
    };
    println!("{} — {} entries", path, archive.entry_count());
    for meta in archive.entries() {
        match archive.detect(meta.id) {
            Ok(d) => println!(
                "  {:<24} {:?} (basis {:?}, {} bytes sampled)",
                meta.path, d.content_type, d.basis, d.sample_bytes
            ),
            Err(e) => println!("  {:<24} <detect failed: {e}>", meta.path),
        }
    }
}
