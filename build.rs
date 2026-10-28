//! Exposes the locked `regex` version as the `REGEX_VERSION` env var.

use std::{env, fs, path::Path};

fn main() {
    let lock_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock_path.display());

    let lock = fs::read_to_string(&lock_path).expect("Cargo.lock should be committed");
    let version = lock
        .split("[[package]]")
        .find_map(|pkg| {
            let mut lines = pkg.lines().map(str::trim);
            lines.find(|l| *l == r#"name = "regex""#)?;
            lines.find_map(|l| l.strip_prefix(r#"version = ""#)?.strip_suffix('"'))
        })
        .expect("regex should be in Cargo.lock");

    println!("cargo::rustc-env=REGEX_VERSION={version}");
}
