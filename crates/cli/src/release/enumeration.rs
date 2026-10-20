//! Exact shipped identities from verified packs, independent of acceptance claims.
use super::*;
use std::io::Write;

pub const MAX_PACKS: usize = 16;
pub const MAX_OUTPUT_BYTES: u64 = 128 * 1024 * 1024;

pub struct Arguments {
    pub root: PathBuf,
    pub packs: Vec<AssetPath>,
    pub output: AssetPath,
}
impl Arguments {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (mut root, mut output, mut packs) = (None, None, Vec::new());
        let (pairs, remainder) = args.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        for pair in pairs {
            let value = pair[1].to_str()?;
            if value.is_empty() || value.starts_with("--") {
                return None;
            }
            match pair[0].to_str()? {
                "--root" if root.is_none() => root = Some(PathBuf::from(value)),
                "--output" if output.is_none() => output = Some(AssetPath::new(value).ok()?),
                "--pack" if packs.len() < MAX_PACKS => packs.push(AssetPath::new(value).ok()?),
                _ => return None,
            }
        }
        if packs.is_empty() || packs.iter().collect::<BTreeSet<_>>().len() != packs.len() {
            return None;
        }
        packs.sort();
        Some(Self {
            root: root?,
            output: output?,
            packs,
        })
    }
}
#[derive(Serialize)]
struct PackEnumeration {
    manifest: FileRef,
    dataset_revision: String,
    catalog_sha256: String,
    sources: Vec<SourceRecord>,
    accepted_origins: u64,
    accepted_unknown_fields: u64,
    remaining_gaps: u64,
    relabelled_evidence: u64,
}
#[derive(Serialize)]
struct Enumeration {
    schema_version: u32,
    target: release::Target,
    origin_policy: FileRef,
    packs: Vec<PackEnumeration>,
    /// These keys and bindings are produced only by the core canonical mapper.
    records: azeroth_core::release_party::ConcreteCatalog,
    absent_record_domains: Vec<String>,
}
#[derive(Serialize)]
pub struct Summary {
    pub bytes: u64,
    pub sha256: String,
    pub records_by_domain: BTreeMap<String, usize>,
    pub missing_provenance: usize,
    pub accepted_unknown_fields: u64,
    pub remaining_gaps: u64,
}
struct LimitedWriter<W> {
    inner: W,
    bytes: u64,
    maximum: u64,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.maximum.saturating_sub(self.bytes) {
            return Err(std::io::Error::other("enumeration output exceeds bound"));
        }
        let count = self.inner.write(bytes)?;
        self.bytes += count as u64;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
pub fn run(args: &Arguments) -> Result<Summary> {
    let root = root_path(&args.root)?;
    let mut budget = Budget::default();
    let policy_bytes = budget.read(&root.join(ORIGIN_POLICY_PATH), 64 * 1024)?;
    let policy = OriginPolicy::decode(&policy_bytes)?;
    let target = release::decode_world_data_target(
        &budget.read(&root.join(WORLD_DATA_TARGET_PATH), 64 * 1024)?,
    )?;
    let mut records: azeroth_core::release_party::ConcreteCatalog = BTreeMap::new();
    let (mut packs, mut revisions) = (Vec::new(), BTreeSet::new());
    for path in &args.packs {
        let directory = root.join(path.as_str());
        let bundle = crate::pack::v2::read_verified(&directory)?;
        if bundle.content.reference.fixture {
            return Err("Release enumeration refuses fixture content");
        }
        let game = &bundle.content.reference.game;
        if game.edition.as_str() != target.edition
            || game.build.as_str() != format!("{}.{}", target.version, target.build)
            || game.interface.as_str() != target.interface.to_string()
        {
            return Err("Enumerated pack differs from selected release target");
        }
        let revision = bundle
            .content
            .reference
            .payload
            .revision
            .as_str()
            .to_owned();
        if !revisions.insert(revision.clone()) {
            return Err("Duplicate enumerated dataset revision");
        }
        let mut rows = azeroth_core::content_v2::records::content_records(&bundle.content)?;
        if let Some(matrix) = bundle.party_support {
            if matrix.target.as_ref() != Some(&target) {
                return Err("Party enumeration differs from selected release target");
            }
            rows.insert(
                "party".into(),
                matrix.bind(bundle.catalog_sha256.as_str(), &rows)?,
            );
        }
        for (domain, entries) in rows {
            for (identity, sources) in entries {
                if records
                    .entry(domain.clone())
                    .or_default()
                    .insert(identity, sources)
                    .is_some()
                {
                    return Err("Duplicate canonical shipped record");
                }
            }
        }
        if records.values().map(BTreeMap::len).sum::<usize>() > release::MAX_RECORDS {
            return Err("Enumerated records exceed release bound");
        }
        let classification =
            azeroth_core::origin_policy::classify(&bundle.content, &bundle.sources, Some(&policy))?;
        let manifest_path = directory.join("manifest.json");
        let raw = budget.read(&manifest_path, crate::pack::MAX_MANIFEST_BYTES)?;
        packs.push(PackEnumeration {
            manifest: FileRef {
                path: relative(&root, &manifest_path)?,
                bytes: raw.len() as u64,
                sha256: hash(&raw).try_into().map_err(|_| "Invalid manifest hash")?,
            },
            dataset_revision: revision,
            catalog_sha256: bundle.catalog_sha256.as_str().into(),
            sources: bundle.sources,
            accepted_origins: classification.accepted_origins,
            accepted_unknown_fields: classification.accepted_unknowns,
            remaining_gaps: classification.remaining_gaps,
            relabelled_evidence: classification.relabelled_evidence,
        });
    }
    let absent_record_domains = release::DOMAINS
        .into_iter()
        .filter(|d| records.get(*d).is_none_or(BTreeMap::is_empty))
        .map(str::to_owned)
        .collect();
    let enumeration = Enumeration {
        schema_version: 1,
        target,
        origin_policy: FileRef {
            path: AssetPath::new(ORIGIN_POLICY_PATH).expect("constant"),
            bytes: policy_bytes.len() as u64,
            sha256: hash(&policy_bytes)
                .try_into()
                .map_err(|_| "Invalid policy hash")?,
        },
        packs,
        records,
        absent_record_domains,
    };
    let output = root.join(args.output.as_str());
    checked_path(output.parent().ok_or("Enumeration parent missing")?, true)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|_| "Enumeration output must be new")?;
    let mut writer = LimitedWriter {
        inner: std::io::BufWriter::new(file),
        bytes: 0,
        maximum: MAX_OUTPUT_BYTES,
    };
    if serde_json::to_writer(&mut writer, &enumeration).is_err()
        || writer.write_all(b"\n").is_err()
        || writer.flush().is_err()
    {
        drop(writer);
        let _ = fs::remove_file(&output);
        return Err("Enumeration output exceeds bound or cannot be written");
    }
    let bytes = writer.bytes;
    drop(writer);
    let output_bytes = Budget::default().read(&output, MAX_OUTPUT_BYTES as usize)?;
    Ok(Summary {
        bytes,
        sha256: hash(&output_bytes),
        records_by_domain: release::DOMAINS
            .into_iter()
            .map(|d| {
                (
                    d.to_owned(),
                    enumeration.records.get(d).map_or(0, BTreeMap::len),
                )
            })
            .collect(),
        missing_provenance: enumeration
            .records
            .values()
            .flat_map(BTreeMap::values)
            .filter(|s| s.is_empty())
            .count(),
        accepted_unknown_fields: enumeration
            .packs
            .iter()
            .map(|p| p.accepted_unknown_fields)
            .sum(),
        remaining_gaps: enumeration
            .packs
            .iter()
            .map(|p| p.remaining_gaps + p.relabelled_evidence)
            .sum(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn args(values: &[&str]) -> Option<Arguments> {
        Arguments::parse(&values.iter().map(OsString::from).collect::<Vec<_>>())
    }
    #[test]
    fn options_are_explicit_bounded_and_output_is_new() {
        for values in [
            vec![],
            vec!["--root"],
            vec!["--root", ".", "--pack", "../bad", "--output", "out.json"],
            vec![
                "--root", ".", "--pack", "a", "--pack", "a", "--output", "out.json",
            ],
            vec!["--root", ".", "--pack", "a", "--output", "--bad"],
            vec!["--unknown", "x"],
        ] {
            assert!(args(&values).is_none());
        }
        let valid = args(&[
            "--root", ".", "--pack", "b", "--pack", "a", "--output", "out.json",
        ])
        .unwrap();
        assert_eq!(valid.packs[0].as_str(), "a");
        let mut writer = LimitedWriter {
            inner: Vec::new(),
            bytes: 0,
            maximum: 3,
        };
        writer.write_all(b"abc").unwrap();
        assert!(writer.write_all(b"d").is_err());
        writer.flush().unwrap();
    }
    #[test]
    fn real_authored_pack_enumeration_uses_canonical_ids_and_preserves_unknowns() {
        let root = std::env::temp_dir().join(format!(
            "nav-enum-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let repo = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
        fs::create_dir_all(root.join("release/v1")).unwrap();
        for file in [ORIGIN_POLICY_PATH, WORLD_DATA_TARGET_PATH] {
            fs::copy(repo.join(file), root.join(file)).unwrap();
        }
        crate::pack::v2::build(&repo.join("content/authored/slice-v1"), &root.join("pack"))
            .unwrap();
        let options = Arguments {
            root: root.clone(),
            packs: vec![AssetPath::new("pack").unwrap()],
            output: AssetPath::new("out.json").unwrap(),
        };
        let summary = run(&options).unwrap();
        assert!(summary.accepted_unknown_fields > 0);
        assert_eq!(summary.remaining_gaps, 0);
        assert_eq!(summary.missing_provenance, 0);
        let first = fs::read(root.join("out.json")).unwrap();
        let value: Value = serde_json::from_slice(&first).unwrap();
        let bundle = crate::pack::v2::read_verified(&root.join("pack")).unwrap();
        let canonical =
            azeroth_core::content_v2::records::content_records(&bundle.content).unwrap();
        for (domain, rows) in canonical {
            assert_eq!(
                value["records"][&domain],
                serde_json::to_value(rows).unwrap()
            );
        }
        assert!(
            value["absent_record_domains"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "quests")
        );
        assert!(run(&options).is_err());
        assert_eq!(fs::read(root.join("out.json")).unwrap(), first);
        fs::remove_file(root.join("out.json")).unwrap();
        run(&options).unwrap();
        assert_eq!(fs::read(root.join("out.json")).unwrap(), first);
        let target_file = root.join(WORLD_DATA_TARGET_PATH);
        let target_bytes = fs::read(&target_file).unwrap();
        let mut target: Value = serde_json::from_slice(&target_bytes).unwrap();
        target["build"] = serde_json::json!("1.60.1.99999");
        fs::write(&target_file, serde_json::to_vec(&target).unwrap()).unwrap();
        assert!(
            run(&Arguments {
                root: root.clone(),
                packs: options.packs.clone(),
                output: AssetPath::new("wrong-target.json").unwrap()
            })
            .is_err()
        );
        fs::write(&target_file, target_bytes).unwrap();
        crate::pack::v2::build(
            &repo.join("content/authored/slice-v1"),
            &root.join("duplicate-pack"),
        )
        .unwrap();
        assert!(
            run(&Arguments {
                root: root.clone(),
                packs: vec![
                    AssetPath::new("pack").unwrap(),
                    AssetPath::new("duplicate-pack").unwrap()
                ],
                output: AssetPath::new("duplicate.json").unwrap()
            })
            .is_err()
        );
        let fixture_input = root.join("fixture-input");
        fs::create_dir(&fixture_input).unwrap();
        fs::copy(
            repo.join("fixtures/contracts/v2/reference-content.json"),
            fixture_input.join("catalog.json"),
        )
        .unwrap();
        fs::copy(
            repo.join("fixtures/content-pack/sources.json"),
            fixture_input.join("sources.json"),
        )
        .unwrap();
        crate::pack::v2::build(&fixture_input, &root.join("fixture-pack")).unwrap();
        assert!(
            run(&Arguments {
                root: root.clone(),
                packs: vec![AssetPath::new("fixture-pack").unwrap()],
                output: AssetPath::new("fixture.json").unwrap()
            })
            .is_err()
        );
        fs::write(root.join("pack/undeclared.json"), b"{}").unwrap();
        assert!(
            run(&Arguments {
                output: AssetPath::new("bad.json").unwrap(),
                ..options
            })
            .is_err()
        );
        assert!(!root.join("bad.json").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
