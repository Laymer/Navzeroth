//! Check knowledge_v1 fact documents with the source pipeline's own parser:
//! `cargo run -p azeroth-core --example validate_knowledge -- <file|dir>...`.
use std::path::{Path, PathBuf};

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
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
    let (mut failed, mut facts) = (0, 0);
    for file in &files {
        let parsed = std::fs::read(file)
            .map_err(|_| "unreadable")
            .and_then(|b| azeroth_core::source_pipeline::Document::parse(&b, i64::MAX));
        match parsed {
            Ok(doc) => facts += doc.facts.len(),
            Err(e) => {
                failed += 1;
                println!("FAIL {}: {e}", file.display());
            }
        }
    }
    println!("{} documents, {facts} facts, {failed} failed", files.len());
    std::process::exit(i32::from(failed > 0));
}
