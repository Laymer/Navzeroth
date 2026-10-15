//! Offline bridge between reviewed admission policy and selected release bytes.
//! A matching hash identifies the reviewed document; it does not authenticate a grant.
use super::*;
use azeroth_core::{
    source_admission::{Registry, Source, Status},
    sources::SourceRecord,
};

pub(super) fn load(
    root: &Path,
    inventory: &Inventory,
    evidence: &Evidence,
    budget: &mut Budget,
) -> Result<BTreeMap<String, Source>> {
    let Some(reference) = &evidence.admission_registry else {
        return if inventory.sources.is_empty() && evidence.sources.is_empty() {
            Ok(BTreeMap::new())
        } else {
            Err("Accepted sources require a pinned admission registry")
        };
    };
    let registry = Registry::parse(&checked_ref_bytes(
        budget,
        root,
        reference,
        azeroth_core::source_admission::MAX_REGISTRY_BYTES,
    )?)
    .map_err(|_| "Invalid or incompatible admission registry")?;
    let mut accepted = BTreeMap::new();
    for requirement in &inventory.sources {
        let source = registry
            .sources
            .iter()
            .find(|s| s.id == requirement.id && s.revision == requirement.revision)
            .ok_or("Accepted source is absent from admission registry")?;
        if source.rights.status != Status::Admitted {
            return Err("Release source rights are not admitted");
        }
        let proof = evidence
            .sources
            .iter()
            .find(|s| s.id == source.id && s.revision == source.revision)
            .ok_or("Admitted source lacks exact release evidence")?;
        if source.origin.edition.as_deref() != Some(proof.origin_edition.as_str())
            || source.origin.build != proof.origin_build
        {
            return Err("Release source origin differs from admission");
        }
        for file in &proof.permission_evidence {
            budget.verify(root, file)?;
        }
        for grant in source
            .rights
            .evidence
            .iter()
            .chain(source.retrieval.access_evidence.iter())
        {
            if !proof
                .permission_evidence
                .iter()
                .any(|f| f.sha256.as_str() == grant.sha256)
            {
                return Err("Admission grant is not bound to checked permission evidence");
            }
        }
        if proof.files.is_empty() {
            return Err("Admitted release source has no original material");
        }
        for material in &proof.files {
            let permitted = source
                .files
                .iter()
                .find(|f| f.path == material.path.as_str())
                .ok_or("Original source material path is not admitted")?;
            if permitted.sha256 != material.sha256.as_str() || permitted.bytes != material.bytes {
                return Err("Original source material differs from admitted bytes");
            }
            budget.verify(root, material)?;
        }
        // Field permission applies to the original materials actually selected,
        // not to another permitted file absent from this release closure.
        let mut selected = source.clone();
        selected
            .files
            .retain(|f| proof.files.iter().any(|p| p.path.as_str() == f.path));
        accepted.insert(source.id.clone(), selected);
    }
    Ok(accepted)
}
pub(super) fn pack_source(
    record: &SourceRecord,
    admitted: &BTreeMap<String, Source>,
) -> Result<()> {
    let source = admitted
        .get(record.id.as_str())
        .ok_or("Pack source is not admitted")?;
    if record.publisher.as_str() != source.publisher
        || record.license.value().map(|v| v.as_str()) != source.rights.license_identifier.as_deref()
        || record.attribution.value().map(|v| v.as_str()) != source.rights.attribution.as_deref()
        || record.verified_revision.value().map(|v| v.as_str()) != Some(source.revision.as_str())
    {
        return Err("Pack publisher, license, attribution or revision differs from admission");
    }
    let permitted: BTreeSet<_> = source
        .files
        .iter()
        .flat_map(|f| f.fields.iter().map(String::as_str))
        .collect();
    let fields = record
        .allowed_fields
        .value()
        .ok_or("Pack allowed fields are unknown")?;
    if fields.is_empty() || fields.iter().any(|f| !permitted.contains(f.as_str())) {
        return Err("Pack fields exceed admitted source fields");
    }
    let permitted_transforms: BTreeSet<_> = source
        .files
        .iter()
        .map(|f| f.transformation.as_str())
        .collect();
    let transformations = record
        .transformations
        .value()
        .ok_or("Pack transformations are unknown")?;
    if transformations.is_empty()
        || transformations
            .iter()
            .any(|t| !permitted_transforms.contains(t.as_str()))
    {
        return Err("Pack transformations exceed admitted source transformations");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn transformations_require_known_exact_declarations_and_allow_multiple_selected_files() {
        let raw: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/content-pack/sources.json"
        ))
        .unwrap();
        let mut record = raw["sources"][0].clone();
        record["verified_revision"] = json!({"status":"known","value":"1".repeat(40)});
        let source: Source = serde_json::from_value(json!({"id":record["id"],"revision":"1".repeat(40),"publisher":record["publisher"],"origin":{"edition":null,"build":null},
            "rights":{"status":"admitted","license":"permissive","license_identifier":record["license"]["value"],"reviewer":"synthetic","attribution":record["attribution"]["value"],"obligations":"synthetic","evidence":[]},
            "retrieval":{"method":"manual_local","max_total_bytes":2,"minimum_interval_ms":1000,"max_requests":2,"access_evidence":null},
            "files":[{"path":"a","url":"https://example.invalid/a","sha256":"0".repeat(64),"bytes":1,"fields":record["allowed_fields"]["value"],"transformation":"canonical_typed_serialization"},{"path":"b","url":"https://example.invalid/b","sha256":"0".repeat(64),"bytes":1,"fields":["second"],"transformation":"second_transform"}]})).unwrap();
        // load() validates registry/proofs and selects originals before this unit.
        let admitted = BTreeMap::from([(source.id.clone(), source)]);
        record["transformations"] =
            json!({"status":"known","value":["canonical_typed_serialization","second_transform"]});
        pack_source(&serde_json::from_value(record.clone()).unwrap(), &admitted).unwrap();
        for value in [
            json!({"status":"unknown"}),
            json!({"status":"known","value":["unreviewed_transform"]}),
        ] {
            record["transformations"] = value;
            assert!(
                pack_source(&serde_json::from_value(record.clone()).unwrap(), &admitted).is_err()
            );
        }
        record["transformations"] = json!({"status":"known","value":[]});
        assert!(serde_json::from_value::<SourceRecord>(record).is_err());
    }
}
