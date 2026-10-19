//! Validate world-data shards: `cargo run -p azeroth-core --example validate_world_data -- <file|dir>...`.
//! A development aid for the generators; the CLI's `verify-world-data` is the checked path.
use std::path::{Path, PathBuf};

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    // Runner bookkeeping (.written/), fact documents and the run manifest are not shards.
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('.') || name == "facts" || name == "run-manifest.json" {
        return;
    }
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for entry in entries {
            collect(&entry, out);
        }
    } else if path.extension().is_some_and(|e| e == "json") {
        out.push(path.to_path_buf());
    }
}

fn main() {
    let mut files = Vec::new();
    for arg in std::env::args().skip(1) {
        collect(Path::new(&arg), &mut files);
    }
    let mut failed = 0;
    for file in &files {
        let result = std::fs::read(file)
            .map_err(|e| e.to_string())
            .and_then(|b| azeroth_core::world_data::parse(&b).map_err(|e| format!("{e:?}")));
        match result {
            Ok(data) => println!("ok {} {} records", file.display(), data.records.len()),
            Err(e) => {
                failed += 1;
                println!("FAIL {}: {e}", file.display());
            }
        }
    }
    println!("{} files, {failed} failed", files.len());
    std::process::exit(i32::from(failed > 0));
}
