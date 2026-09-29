use std::path::PathBuf;

use anyhow::Result;
use site::{BuildOptions, build, date};

/// Usage : `cargo run -p site --release [-- --offline]` depuis la racine du repo.
fn main() -> Result<()> {
    let offline = std::env::args().any(|arg| arg == "--offline");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    build(&BuildOptions {
        out_dir: root.join("dist"),
        root,
        fetch_feeds: !offline,
        year: date::current_year(),
    })?;
    println!("site généré dans dist/");
    Ok(())
}
