//! Runs once, before `rustc` compiles this crate -- not as a side effect of
//! macro expansion, so there's no ordering to get right.
//!
//! Scans this crate's own `src/` for `style!`/`style_file!` declarations
//! (via a real syntax-tree walk, see `dappr-styles`), bundles + minifies
//! them together with Pico's default styles, and writes
//! `static/styles.min.css`.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir: PathBuf = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is always set by Cargo for build scripts")
        .into();

    // Only rerun when this crate's own source changes -- not on every build.
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").display()
    );

    let styles = dappr_styles::scan_dir(&manifest_dir).unwrap_or_else(|e| {
        panic!("dappr build.rs: failed scanning for style!/style_file! declarations: {e}")
    });

    // style_file!("scope", "some/file.css") points at a CSS file that lives
    // outside src/*.rs -- watch each one individually so editing just the
    // CSS (without touching the .rs file that declares it) still reruns
    // this script.
    for style in &styles {
        if let Some(css_file) = &style.css_file {
            println!("cargo:rerun-if-changed={}", css_file.display());
        }
    }

    let bundled = dappr_styles::bundle(&styles, Some(dappr_styles::pico_css()))
        .unwrap_or_else(|e| panic!("dappr build.rs: failed bundling styles: {e}"));

    let static_dir = manifest_dir.join("static");
    fs::create_dir_all(&static_dir).unwrap_or_else(|e| {
        panic!(
            "dappr build.rs: could not create {}: {e}",
            static_dir.display()
        )
    });
    fs::write(static_dir.join("styles.min.css"), bundled)
        .unwrap_or_else(|e| panic!("dappr build.rs: could not write static/styles.min.css: {e}"));
}
