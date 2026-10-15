//! Explicit, read-only local file boundary for QuestieDB raw-source staging.
//! Files and gameplay rows are never emitted. Source declarations stay unverified.
use azeroth_ingest::questiedb::{self, Entity, MAX_FILE_BYTES, SourceOrigin, StagingReport};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub const USAGE: &str = "stage-questiedb [--quest <file>] [--npc <file>] [--item <file>] [--object <file>] [--revision <40-lowercase-hex>] [--edition wow_forever] [--build <label>]; at least one raw file is required. JSON metadata only; source declarations remain unverified. See docs/data/QUESTIEDB_STAGING.md.";

#[derive(Debug)]
pub struct Options {
    raw: BTreeMap<Entity, PathBuf>,
    origin: SourceOrigin,
}
impl Options {
    /// Exact option/value pairs; at most four files and three declarations.
    /// Paths may contain non-UTF-8 bytes on platforms that support them.
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (pairs, remainder) = args.as_chunks::<2>();
        if args.len() > 14 || !remainder.is_empty() {
            return None;
        }
        let mut options = Self {
            raw: BTreeMap::new(),
            origin: SourceOrigin::default(),
        };
        for [flag, value] in pairs {
            let flag = flag.to_str()?;
            if value.is_empty() || value.as_encoded_bytes().starts_with(b"--") {
                return None;
            }
            let entity = match flag {
                "--quest" => Some(Entity::Quest),
                "--npc" => Some(Entity::Npc),
                "--item" => Some(Entity::Item),
                "--object" => Some(Entity::Object),
                _ => None,
            };
            if let Some(entity) = entity {
                if options.raw.insert(entity, PathBuf::from(value)).is_some() {
                    return None;
                }
                continue;
            }
            let slot = match flag {
                "--revision" => &mut options.origin.revision,
                "--edition" => &mut options.origin.edition,
                "--build" => &mut options.origin.build,
                _ => return None,
            };
            let text = value.to_str()?;
            if slot.is_some() || text.len() > 64 {
                return None;
            }
            *slot = Some(text.to_owned());
        }
        (!options.raw.is_empty()).then_some(options)
    }
}

#[derive(Debug)]
pub enum Error {
    Inspect,
    UnsafePath,
    FileKind,
    TooLarge,
    DuplicateInput,
    Read,
    Changed,
    Source(questiedb::Rejection),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inspect => f.write_str("Selected source file cannot be inspected"),
            Self::UnsafePath => {
                f.write_str("Symlink, reparse and parent-traversal paths are unsupported")
            }
            Self::FileKind => f.write_str("Selected source must be a regular file"),
            Self::TooLarge => f.write_str("Selected source exceeds the 8 MiB file limit"),
            Self::DuplicateInput => f.write_str("A source file was selected more than once"),
            Self::Read => f.write_str("Selected source file cannot be read"),
            Self::Changed => f.write_str("Selected source changed while it was being read"),
            Self::Source(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<questiedb::Rejection> for Error {
    fn from(error: questiedb::Rejection) -> Self {
        Self::Source(error)
    }
}

fn regular(meta: &Metadata) -> Result<(), Error> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT, as in the existing pack/replay boundary.
        if meta.file_attributes() & 0x400 != 0 {
            return Err(Error::UnsafePath);
        }
    }
    if meta.file_type().is_symlink() {
        return Err(Error::UnsafePath);
    }
    if !meta.is_file() {
        return Err(Error::FileKind);
    }
    if meta.len() > MAX_FILE_BYTES as u64 {
        return Err(Error::TooLarge);
    }
    Ok(())
}

/// Inspect every path component rather than following a selected link.
/// This follows the existing local pack/replay path policy.
fn checked_path(path: &Path) -> Result<(PathBuf, Metadata), Error> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| Error::Inspect)?
            .join(path)
    };
    let mut prefix = PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err(Error::UnsafePath);
        }
        prefix.push(part.as_os_str());
        let meta = fs::symlink_metadata(&prefix).map_err(|_| Error::Inspect)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(Error::UnsafePath);
            }
        }
        if meta.file_type().is_symlink() {
            return Err(Error::UnsafePath);
        }
        if prefix != absolute && !meta.is_dir() {
            return Err(Error::FileKind);
        }
    }
    let meta = fs::symlink_metadata(&absolute).map_err(|_| Error::Inspect)?;
    regular(&meta)?;
    Ok((absolute, meta))
}

fn unchanged(before: &Metadata, after: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return false;
        }
    }
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}

fn read(path: &Path, selected: &Metadata) -> Result<Vec<u8>, Error> {
    // Check the selection again before open, then inspect the opened handle.
    let (_, before) = checked_path(path)?;
    if !unchanged(selected, &before) {
        return Err(Error::Changed);
    }
    let mut file = File::open(path).map_err(|_| Error::Read)?;
    let opened = file.metadata().map_err(|_| Error::Inspect)?;
    regular(&opened)?;
    if !unchanged(&before, &opened) {
        return Err(Error::Changed);
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Read)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(Error::TooLarge);
    }
    let after = file.metadata().map_err(|_| Error::Inspect)?;
    let (_, selected_after) = checked_path(path)?;
    if bytes.len() as u64 != opened.len()
        || !unchanged(&opened, &after)
        || !unchanged(&opened, &selected_after)
    {
        return Err(Error::Changed);
    }
    Ok(bytes)
}

/// Reuse the existing bounded, component-checked local-source read boundary.
pub(crate) fn read_local_source(path: &Path) -> Result<Vec<u8>, Error> {
    let (absolute, selected) = checked_path(path)?;
    read(&absolute, &selected)
}

/// Stage only the selected raw files. No discovery, network, correction
/// execution, file writes or playable conversion occurs. All output is metadata.
pub fn report(options: &Options) -> Result<StagingReport, Error> {
    let mut paths = BTreeSet::new();
    let mut selected = Vec::new();
    for (&entity, path) in &options.raw {
        let (absolute, meta) = checked_path(path)?;
        let canonical = fs::canonicalize(&absolute).map_err(|_| Error::Inspect)?;
        if !paths.insert(canonical) {
            return Err(Error::DuplicateInput);
        }
        selected.push((entity, absolute, meta));
    }
    let mut tables = Vec::new();
    for (entity, path, meta) in selected {
        let bytes = read(&path, &meta)?;
        tables.push(questiedb::stage_raw(
            &bytes,
            entity,
            options.origin.clone(),
        )?);
    }
    Ok(questiedb::summarize(&tables, &[])?)
}
