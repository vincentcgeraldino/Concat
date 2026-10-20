// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Embeds every package under `packages/` into the binary.
//!
//! Each package is a folder named after its id, holding `effect.toml` and
//! optionally `fixtures.toml`. This script lists the folders and writes a
//! table of `include_str!`s, so adding a built-in effect is adding a folder:
//! nothing in Rust names it.

use std::fmt::Write as _;
use std::path::PathBuf;

/// The shader file the manifest names: the `[wgsl]` table's entry, or the
/// `[transition]` table's. None when the manifest names neither - a
/// package with only an FFmpeg chain has no shader.
fn shader_entry(manifest: &str) -> Option<String> {
    let value: toml::Value = toml::from_str(manifest).ok()?;
    for table in ["wgsl", "transition"] {
        if let Some(entry) = value
            .get(table)
            .and_then(|table| table.get("entry"))
            .and_then(|entry| entry.as_str())
        {
            return Some(entry.to_owned());
        }
    }
    None
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let packages = root.join("packages");
    println!("cargo:rerun-if-changed={}", packages.display());

    let mut ids: Vec<String> = std::fs::read_dir(&packages)
        .expect("packages/ exists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("effect.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();

    let mut table = String::from(
        "/// Every built-in package: its folder name, its manifest, its \
         fixtures if it has any, and its shader if it has one.\n\
         pub(crate) static BUILTIN_SOURCES: &[(&str, &str, Option<&str>, Option<&str>)] = &[\n",
    );
    for id in &ids {
        let dir = packages.join(id);
        let manifest = dir.join("effect.toml");
        let fixtures = dir.join("fixtures.toml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        // Only files that exist are watched: cargo counts a missing one as
        // changed on every build, which reran this script and rebuilt every
        // crate above it each time. One added later is still seen, through
        // the watch on `packages/` above.
        let fixtures = if fixtures.is_file() {
            println!("cargo:rerun-if-changed={}", fixtures.display());
            format!("Some(include_str!({:?}))", fixtures.display().to_string())
        } else {
            "None".to_owned()
        };
        // The shader is the file the manifest names, not a name this
        // script decides: a built-in whose shader is called something else
        // used to be embedded with no shader at all, and the catalogue
        // panicked on it when the app first asked for its effects.
        let manifest_text = std::fs::read_to_string(&manifest).expect("the manifest is there");
        let shader = shader_entry(&manifest_text).map(|entry| dir.join(entry));
        let shader = match shader {
            Some(shader) => {
                println!("cargo:rerun-if-changed={}", shader.display());
                format!("Some(include_str!({:?}))", shader.display().to_string())
            }
            None => "None".to_owned(),
        };
        writeln!(
            table,
            "    ({id:?}, include_str!({:?}), {fixtures}, {shader}),",
            manifest.display().to_string()
        )
        .expect("write");
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("builtins.rs");
    std::fs::write(out, table).expect("write builtins.rs");
}
