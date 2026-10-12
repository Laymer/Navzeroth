//! Bounded file handling around the pure coverage report.
use azeroth_core::{
    contract::MAX_DATASET_BYTES,
    coverage::{ContentReport, content_report_from_json},
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const MAX_FILES: usize = 128;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

/// One explicitly selected file, or a directory's datasets/*.json (not imagery).
/// No symlinks, recursion, execution, or arbitrary diagnostics from input.
pub fn reports(input: &Path) -> Result<Vec<ContentReport>, &'static str> {
    let meta = std::fs::symlink_metadata(input).map_err(|_| "Content input cannot be read")?;
    let files: Vec<PathBuf> = if meta.file_type().is_file() {
        vec![input.to_owned()]
    } else if meta.file_type().is_dir() {
        let datasets = input.join("datasets");
        let root = if datasets.exists() {
            datasets
        } else {
            input.to_owned()
        };
        if !std::fs::symlink_metadata(&root)
            .map_err(|_| "Content directory cannot be read")?
            .file_type()
            .is_dir()
        {
            return Err("Content directory must be a regular directory");
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(root).map_err(|_| "Content directory cannot be read")? {
            let entry = entry.map_err(|_| "Content directory cannot be read")?;
            if entry.path().extension().is_some_and(|ext| ext == "json") {
                if paths.len() == MAX_FILES {
                    return Err("Too many content files");
                }
                paths.push(entry.path());
            }
        }
        paths.sort();
        paths
    } else {
        return Err("Content input must be a regular file or directory");
    };
    if files.is_empty() {
        return Err("No content datasets found");
    }
    let mut total = 0usize;
    let mut reports = Vec::new();
    for path in files {
        let meta = std::fs::symlink_metadata(&path).map_err(|_| "Content file cannot be read")?;
        if !meta.file_type().is_file() {
            return Err("Content file must be a regular file");
        }
        if meta.len() > MAX_DATASET_BYTES as u64 {
            return Err("Content file exceeds size limit");
        }
        let file = std::fs::File::open(path).map_err(|_| "Content file cannot be read")?;
        let mut bytes = Vec::new();
        file.take(MAX_DATASET_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Content file cannot be read")?;
        if bytes.len() > MAX_DATASET_BYTES {
            return Err("Content file exceeds size limit");
        }
        total += bytes.len();
        if total > MAX_TOTAL_BYTES {
            return Err("Content input exceeds total size limit");
        }
        let text = String::from_utf8(bytes).map_err(|_| "Content file must be UTF-8")?;
        reports.push(content_report_from_json(&text));
    }
    Ok(reports)
}

pub fn human(reports: &[ContentReport]) -> String {
    let mut output = String::new();
    for (index, report) in reports.iter().enumerate() {
        output.push_str(&format!(
            "Dataset {}: {} ({} errors, {} gaps)\n",
            index + 1,
            if report.passes { "valid" } else { "invalid" },
            report.errors.len(),
            report.gaps.len()
        ));
        if let Some(header) = &report.dataset {
            output.push_str(&format!(
                "  revision: {}; class: {:?}; edition: {}; build: {}\n",
                header.revision.as_str(),
                header.coverage_class,
                header.edition.as_str(),
                header.build.as_str()
            ));
        }
        if let Some(coverage) = &report.coverage {
            for (collection, counts) in &coverage.collections {
                output.push_str(&format!("  {collection:?}: {}\n", counts.total));
            }
        }
        for error in &report.errors {
            output.push_str(&format!("  error: {}\n", error.code()));
        }
        for gap in &report.gaps {
            output.push_str(&format!("  gap: {}\n", gap.code()));
        }
        if let azeroth_core::coverage::ReportOutcome::DecodeFailed { code, .. } = report.outcome {
            output.push_str(&format!("  decode failed: {code:?}\n"));
        }
    }
    output
}
