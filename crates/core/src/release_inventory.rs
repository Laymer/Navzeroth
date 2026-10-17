//! Deterministic inventory investigation; never a source-admission or release gate.
//! Independently reviewed target enumeration is supplied separately from imports.
use crate::release::{
    Completeness, DOMAINS, FileRef, Inventory, MAX_RECORDS, MAX_RELEASE_BYTES, MAX_UNITS, Status,
    Target,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema_version: u32,
    pub inventory: Inventory,
    /// Human review of independent target scope, not an imported row count.
    pub enumeration_reviews: Vec<EnumerationReview>,
    pub records: Vec<Record>,
    pub scenarios: Vec<Scenario>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnumerationReview {
    pub domain: String,
    pub target: Target,
    pub reviewer: String,
    pub method: EnumerationMethod,
    pub evidence: Vec<FileRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnumerationMethod {
    IndependentTargetReview,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub domain: String,
    /// Exact release binding (canonical JSON-array string), never a guessed ID.
    pub record_id: String,
    pub source_id: String,
    pub source_revision: String,
    pub origin_edition: String,
    pub origin_build: Option<String>,
    pub target: Target,
    pub target_validation: Status,
    pub evidence: Vec<FileRef>,
    pub unresolved_conflicts: u32,
    pub unsupported_corrections: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub domain: String,
    pub procedure: String,
    pub required_record_ids: Vec<String>,
    pub validation: Status,
    pub evidence: Vec<FileRef>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub inventory_id: String,
    /// False even when comparisons pass: rights and actual pack validation are external.
    pub publication_authorized: bool,
    pub domains: Vec<DomainReport>,
    pub observation_queue: Vec<Observation>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct DomainReport {
    pub domain: String,
    pub independently_enumerated: bool,
    /// None means unknown, Some(0) requires reviewed explicit-empty scope.
    pub required_count: Option<usize>,
    pub imported_count: usize,
    pub evidenced_empty_units: Vec<String>,
    pub unknown_units: Vec<String>,
    pub missing_record_ids: Vec<String>,
    pub unexpected_record_ids: Vec<String>,
    pub blocked_record_ids: Vec<String>,
    pub source_comparisons: Vec<SourceComparison>,
    pub validation_requirements: Vec<String>,
    pub scenario_ids: Vec<String>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SourceComparison {
    pub source_id: String,
    pub source_revision: String,
    pub included_record_ids: Vec<String>,
    pub missing_record_ids: Vec<String>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Observation {
    pub domain: String,
    pub subject: String,
    pub reason: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096
}
fn domain(value: &str) -> bool {
    DOMAINS.contains(&value)
}
fn identities(ids: &[String]) -> bool {
    ids.iter().all(|id| nonempty(id)) && ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
}
fn files(files: &[FileRef]) -> bool {
    files.len() <= 256
        && files
            .iter()
            .all(|file| file.bytes > 0 && file.bytes <= 256 * 1024 * 1024)
}
impl Input {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_RELEASE_BYTES {
            return Err("inventory input exceeds byte limit");
        }
        let input: Self =
            serde_json::from_slice(bytes).map_err(|_| "invalid inventory input JSON")?;
        input.validate()?;
        Ok(input)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.inventory.schema_version != 1 {
            return Err("unsupported inventory input version");
        }
        if self.inventory.target != Some(Target::forever_70374()) {
            return Err("unsupported inventory target");
        }
        if !nonempty(&self.inventory.inventory_id)
            || self.inventory.domains.len() != DOMAINS.len()
            || self.records.len() > MAX_RECORDS
            || self.scenarios.len() > MAX_RECORDS
            || self.enumeration_reviews.len() > DOMAINS.len()
            || self.inventory.sources.len() > MAX_UNITS
        {
            return Err("invalid inventory bounds");
        }
        let mut domains = BTreeSet::new();
        let mut units = BTreeSet::new();
        let mut total = 0usize;
        let mut required_domains = BTreeMap::new();
        for d in &self.inventory.domains {
            if !domain(&d.domain)
                || !domains.insert(&d.domain)
                || d.units.is_empty()
                || !files(&d.enumeration_evidence)
            {
                return Err("invalid domain inventory");
            }
            let mut ids = BTreeSet::new();
            for unit in &d.units {
                if !nonempty(&unit.id) || !nonempty(&unit.scope) || !units.insert(&unit.id) {
                    return Err("invalid inventory unit");
                }
                if let Some(required) = &unit.required_record_ids {
                    total = total
                        .checked_add(required.len())
                        .ok_or("inventory capacity overflow")?;
                    if !identities(required)
                        || required.iter().any(|id| {
                            !ids.insert(id) || required_domains.insert(id, &d.domain).is_some()
                        })
                    {
                        return Err("duplicate or empty required identity");
                    }
                }
            }
        }
        if units.len() > MAX_UNITS || total > MAX_RECORDS {
            return Err("inventory capacity exceeded");
        }
        if total
            .checked_mul(self.inventory.sources.len())
            .is_none_or(|n| n > 2_500_000)
        {
            return Err("source comparison output capacity exceeded");
        }
        let mut sources = BTreeMap::new();
        for source in &self.inventory.sources {
            if !nonempty(&source.id)
                || source.revision.len() != 40
                || !source
                    .revision
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || sources.insert(&source.id, &source.revision).is_some()
            {
                return Err("invalid or duplicate source requirement");
            }
        }
        let mut reviews = BTreeSet::new();
        for review in &self.enumeration_reviews {
            if !domain(&review.domain)
                || !reviews.insert(&review.domain)
                || Some(&review.target) != self.inventory.target.as_ref()
                || !nonempty(&review.reviewer)
                || review.evidence.is_empty()
                || !files(&review.evidence)
            {
                return Err("invalid independent enumeration review");
            }
            let d = self
                .inventory
                .domains
                .iter()
                .find(|d| d.domain == review.domain)
                .unwrap();
            if d.enumeration_evidence != review.evidence {
                return Err("enumeration review evidence differs from inventory");
            }
        }
        let enumeration_hashes: BTreeSet<_> = self
            .enumeration_reviews
            .iter()
            .flat_map(|r| &r.evidence)
            .map(|f| f.sha256.as_str())
            .collect();
        let mut records = BTreeSet::new();
        let mut imported_domains = BTreeMap::new();
        for record in &self.records {
            if required_domains
                .get(&record.record_id)
                .is_some_and(|d| **d != record.domain)
                || imported_domains
                    .insert(&record.record_id, &record.domain)
                    .is_some_and(|d| *d != record.domain)
            {
                return Err("record identity belongs to a different domain");
            }
            if !domain(&record.domain)
                || !nonempty(&record.record_id)
                || !nonempty(&record.origin_edition)
                || record.origin_build.as_ref().is_some_and(|b| !nonempty(b))
                || Some(&record.target) != self.inventory.target.as_ref()
                || sources.get(&record.source_id) != Some(&&record.source_revision)
                || !records.insert((&record.domain, &record.record_id, &record.source_id))
                || !files(&record.evidence)
            {
                return Err("invalid imported record or source binding");
            }
            if record
                .evidence
                .iter()
                .any(|f| enumeration_hashes.contains(f.sha256.as_str()))
            {
                return Err("import evidence cannot serve as independent target enumeration");
            }
        }
        let mut scenarios = BTreeSet::new();
        let mut dependencies = 0usize;
        for scenario in &self.scenarios {
            dependencies = dependencies
                .checked_add(scenario.required_record_ids.len())
                .ok_or("scenario capacity overflow")?;
            if !domain(&scenario.domain)
                || !nonempty(&scenario.id)
                || !scenarios.insert(&scenario.id)
                || !nonempty(&scenario.procedure)
                || scenario.required_record_ids.is_empty()
                || !identities(&scenario.required_record_ids)
                || !files(&scenario.evidence)
            {
                return Err("invalid scenario");
            }
        }
        if dependencies > MAX_RECORDS {
            return Err("scenario capacity exceeded");
        }
        Ok(())
    }
}

/// Frozen investigation requirements, not inferred facts or availability claims.
pub fn requirements(domain: &str) -> &'static [&'static str] {
    match domain {
        "quests" => &[
            "stages_objectives",
            "prerequisites_branches",
            "eligibility",
            "repeat_reset",
            "sharing",
            "rewards",
        ],
        "equipment" => &[
            "variants_slots_handedness",
            "attributes_effects",
            "requirements",
            "configuration_restrictions",
        ],
        "acquisition" => &[
            "reward_choices",
            "vendors_crafting_drops",
            "quantities_costs",
            "availability_prerequisites",
            "loot_entitlement",
        ],
        "geography" => &[
            "regions_instances_floors",
            "coordinate_frames",
            "calibrated_locations",
            "imagery_rights",
        ],
        "travel" => &[
            "directed_connections_endpoints",
            "access_unlocks",
            "costs_waits",
            "geometry",
            "stopping_routes",
        ],
        "dungeons" => &[
            "instances_entrances_access",
            "preparation_chains",
            "travel_rewards",
            "evidenced_checklists",
        ],
        "class_roles" => &[
            "applicability_combinations",
            "documented_models",
            "proficiency_layout",
            "exclusions",
            "representative_cases",
        ],
        "party" => &[
            "sizes_workflows",
            "unequal_progress_sharing",
            "stale_members",
            "fairness_agreement",
            "advancement_review",
        ],
        "reminders" => &[
            "all_shipped_rules",
            "supporting_facts_conditions",
            "priority_suppression",
            "unknown_restricted_stale",
        ],
        _ => &[],
    }
}

pub fn compare(input: &Input) -> Result<Report, &'static str> {
    input.validate()?;
    let mut report = Report {
        report_version: 1,
        inventory_id: input.inventory.inventory_id.clone(),
        publication_authorized: false,
        domains: vec![],
        observation_queue: vec![],
    };
    let all_required: BTreeSet<_> = input
        .inventory
        .domains
        .iter()
        .flat_map(|d| &d.units)
        .filter_map(|u| u.required_record_ids.as_ref())
        .flatten()
        .collect();
    let all_imported: BTreeSet<_> = input.records.iter().map(|r| &r.record_id).collect();
    let all_blocked: BTreeSet<_> = input
        .records
        .iter()
        .filter(|r| {
            r.target_validation != Status::Pass
                || r.evidence.is_empty()
                || r.unresolved_conflicts > 0
                || r.unsupported_corrections > 0
        })
        .map(|r| &r.record_id)
        .collect();
    let mut by_source: BTreeMap<(&str, &str), BTreeSet<String>> = BTreeMap::new();
    for record in &input.records {
        by_source
            .entry((&record.domain, &record.source_id))
            .or_default()
            .insert(record.record_id.clone());
    }
    for name in DOMAINS {
        let d = input
            .inventory
            .domains
            .iter()
            .find(|d| d.domain == name)
            .unwrap();
        let reviewed = input.enumeration_reviews.iter().any(|r| r.domain == name);
        let known = reviewed
            && d.completeness == Completeness::Complete
            && d.units.iter().all(|u| u.required_record_ids.is_some());
        let required: BTreeSet<_> = d
            .units
            .iter()
            .filter_map(|u| u.required_record_ids.as_ref())
            .flatten()
            .cloned()
            .collect();
        let records: Vec<_> = input.records.iter().filter(|r| r.domain == name).collect();
        let imported: BTreeSet<_> = records.iter().map(|r| r.record_id.clone()).collect();
        let blocked: BTreeSet<_> = records
            .iter()
            .filter(|r| {
                r.target_validation != Status::Pass
                    || r.evidence.is_empty()
                    || r.unresolved_conflicts > 0
                    || r.unsupported_corrections > 0
            })
            .map(|r| r.record_id.clone())
            .collect();
        let missing: Vec<_> = required.difference(&imported).cloned().collect();
        let mut queue = |subject: &str, reason: &str| {
            report.observation_queue.push(Observation {
                domain: name.into(),
                subject: subject.into(),
                reason: reason.into(),
            })
        };
        if !known {
            queue(name, "independent_target_enumeration_required");
        }
        for id in &missing {
            queue(id, "missing_record");
        }
        for id in &blocked {
            queue(id, "target_validation_conflict_or_correction_required");
        }
        for id in imported.difference(&required) {
            queue(id, "record_outside_enumerated_scope");
        }
        let scenarios: Vec<_> = input
            .scenarios
            .iter()
            .filter(|s| s.domain == name)
            .collect();
        if scenarios.is_empty() {
            queue(name, "representative_scenarios_required");
        }
        for scenario in &scenarios {
            if scenario.required_record_ids.iter().any(|id| {
                !all_required.contains(id) || !all_imported.contains(id) || all_blocked.contains(id)
            }) {
                queue(&scenario.id, "scenario_content_dependency_unavailable");
            }
            if scenario.validation != Status::Pass || scenario.evidence.is_empty() {
                queue(&scenario.id, "scenario_observation_required");
            }
        }
        let mut comparisons = Vec::new();
        for source in &input.inventory.sources {
            let included = by_source
                .remove(&(name, source.id.as_str()))
                .unwrap_or_default();
            comparisons.push(SourceComparison {
                source_id: source.id.clone(),
                source_revision: source.revision.clone(),
                missing_record_ids: required.difference(&included).cloned().collect(),
                included_record_ids: included.into_iter().collect(),
            });
        }
        comparisons.sort_by(|a, b| {
            (&a.source_id, &a.source_revision).cmp(&(&b.source_id, &b.source_revision))
        });
        let mut empty: Vec<_> = d
            .units
            .iter()
            .filter(|u| reviewed && u.required_record_ids.as_ref().is_some_and(Vec::is_empty))
            .map(|u| u.id.clone())
            .collect();
        empty.sort();
        let mut unknown: Vec<_> = d
            .units
            .iter()
            .filter(|u| u.required_record_ids.is_none() || !reviewed)
            .map(|u| u.id.clone())
            .collect();
        unknown.sort();
        let mut scenario_ids: Vec<_> = scenarios.iter().map(|s| s.id.clone()).collect();
        scenario_ids.sort();
        report.domains.push(DomainReport {
            domain: name.into(),
            independently_enumerated: known,
            required_count: known.then_some(required.len()),
            imported_count: imported.len(),
            evidenced_empty_units: empty,
            unknown_units: unknown,
            missing_record_ids: missing,
            unexpected_record_ids: imported.difference(&required).cloned().collect(),
            blocked_record_ids: blocked.into_iter().collect(),
            source_comparisons: comparisons,
            validation_requirements: requirements(name).iter().map(|s| (*s).into()).collect(),
            scenario_ids,
        });
    }
    report.observation_queue.sort();
    report.observation_queue.dedup();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::SourceRequirement;

    fn input() -> Input {
        Input::parse(include_bytes!(
            "../../../release/v1/inventory-investigation.json"
        ))
        .unwrap()
    }
    fn evidence(hash: char) -> FileRef {
        serde_json::from_value(serde_json::json!({"path": format!("evidence/{hash}.txt"), "bytes": 1, "sha256": hash.to_string().repeat(64)})).unwrap()
    }
    fn known() -> Input {
        let mut i = input();
        for d in &mut i.inventory.domains {
            d.completeness = Completeness::Complete;
            d.enumeration_evidence = vec![evidence('a')];
            d.units[0].required_record_ids = Some(vec![]);
            i.enumeration_reviews.push(EnumerationReview {
                domain: d.domain.clone(),
                target: i.inventory.target.clone().unwrap(),
                reviewer: "independent-reviewer".into(),
                method: EnumerationMethod::IndependentTargetReview,
                evidence: d.enumeration_evidence.clone(),
            });
        }
        i.inventory.sources.push(SourceRequirement {
            id: "source-a".into(),
            revision: "1111111111111111111111111111111111111111".into(),
        });
        i
    }
    fn record(id: &str) -> Record {
        Record {
            domain: "quests".into(),
            record_id: id.into(),
            source_id: "source-a".into(),
            source_revision: "1111111111111111111111111111111111111111".into(),
            origin_edition: "original-edition".into(),
            origin_build: None,
            target: Target::forever_70374(),
            target_validation: Status::Pass,
            evidence: vec![evidence('b')],
            unresolved_conflicts: 0,
            unsupported_corrections: 0,
        }
    }
    #[test]
    fn unknown_is_not_zero_and_does_not_authorize_publication() {
        let report = compare(&input()).unwrap();
        assert_eq!(report.domains.len(), 9);
        assert!(
            report
                .domains
                .iter()
                .all(|d| d.required_count.is_none() && d.evidenced_empty_units.is_empty())
        );
        assert_eq!(report.observation_queue.len(), 18);
        assert!(!report.publication_authorized);
    }
    #[test]
    fn zero_requires_independent_review_not_import_counts() {
        let mut i = known();
        assert_eq!(compare(&i).unwrap().domains[0].required_count, Some(0));
        i.enumeration_reviews.clear();
        assert_eq!(compare(&i).unwrap().domains[0].required_count, None);
    }
    #[test]
    fn complete_declaration_with_unknown_unit_stays_unknown() {
        let mut i = known();
        i.inventory.domains[0].units[0].required_record_ids = None;
        assert_eq!(compare(&i).unwrap().domains[0].required_count, None);
    }
    #[test]
    fn deterministic_source_comparison_and_missing_queue() {
        let mut i = known();
        i.inventory.domains[0].units[0].required_record_ids = Some(vec!["q2".into(), "q1".into()]);
        i.records = vec![record("q1"), record("outside")];
        let first = compare(&i).unwrap();
        assert_eq!(first.domains[0].missing_record_ids, ["q2"]);
        assert_eq!(first.domains[0].unexpected_record_ids, ["outside"]);
        assert_eq!(
            first.domains[0].source_comparisons[0].missing_record_ids,
            ["q2"]
        );
        i.inventory.domains.reverse();
        i.records.reverse();
        i.enumeration_reviews.reverse();
        assert_eq!(first, compare(&i).unwrap());
    }
    #[test]
    fn conflicts_corrections_and_unverified_records_block_scenarios() {
        let mut i = known();
        i.inventory.domains[0].units[0].required_record_ids = Some(vec!["q1".into()]);
        i.records.push(record("q1"));
        i.scenarios.push(Scenario {
            id: "duo".into(),
            domain: "quests".into(),
            procedure: "operator procedure".into(),
            required_record_ids: vec!["q1".into()],
            validation: Status::Unverified,
            evidence: vec![],
        });
        for change in 0..4 {
            i.records[0] = record("q1");
            match change {
                0 => i.records[0].unresolved_conflicts = 1,
                1 => i.records[0].unsupported_corrections = 1,
                2 => i.records[0].target_validation = Status::Unverified,
                _ => i.records[0].evidence.clear(),
            }
            let report = compare(&i).unwrap();
            assert_eq!(report.domains[0].blocked_record_ids, ["q1"]);
            assert!(report.observation_queue.iter().any(
                |q| q.subject == "duo" && q.reason == "scenario_content_dependency_unavailable"
            ));
        }
    }
    #[test]
    fn rejects_mixed_target_unpinned_source_and_reused_enumeration_evidence() {
        let mut i = known();
        i.records.push(record("q1"));
        i.records[0].target.build = "70338".into();
        assert!(compare(&i).is_err());
        i.records[0] = record("q1");
        i.records[0].source_revision = "other".into();
        assert!(compare(&i).is_err());
        i.records[0] = record("q1");
        i.records[0].evidence = vec![evidence('a')];
        assert!(compare(&i).is_err());
    }
    #[test]
    fn strict_versions_fields_and_duplicates() {
        let mut value = serde_json::to_value(input()).unwrap();
        value["schema_version"] = 2.into();
        assert!(Input::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        value["schema_version"] = 1.into();
        value["extra"] = true.into();
        assert!(Input::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        let bytes = serde_json::to_string(&input()).unwrap().replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        );
        assert!(Input::parse(bytes.as_bytes()).is_err());
        let mut i = input();
        i.inventory.domains[1] = i.inventory.domains[0].clone();
        assert!(compare(&i).is_err());
    }
    #[test]
    fn supports_more_than_1024_records_and_refuses_overflow() {
        let mut i = known();
        i.inventory.domains[0].units[0].required_record_ids =
            Some((0..2048).map(|n| format!("quest-{n}")).collect());
        assert_eq!(
            compare(&i).unwrap().domains[0].missing_record_ids.len(),
            2048
        );
        i.inventory.domains[0].units[0].required_record_ids =
            Some((0..=MAX_RECORDS).map(|n| format!("q{n}")).collect());
        assert!(compare(&i).is_err());
        assert!(Input::parse(&vec![b' '; MAX_RELEASE_BYTES + 1]).is_err());
    }
    #[test]
    fn scenarios_resolve_cross_domain_dependencies_without_domain_substitution() {
        let mut i = known();
        i.inventory.domains[0].units[0].required_record_ids = Some(vec!["q1".into()]);
        i.records.push(record("q1"));
        i.scenarios.push(Scenario {
            id: "dungeon-chain".into(),
            domain: "dungeons".into(),
            procedure: "inspect preparation quest".into(),
            required_record_ids: vec!["q1".into()],
            validation: Status::Pass,
            evidence: vec![evidence('c')],
        });
        assert!(
            !compare(&i)
                .unwrap()
                .observation_queue
                .iter()
                .any(|q| q.subject == "dungeon-chain")
        );
        i.records[0].domain = "equipment".into();
        assert!(compare(&i).is_err());
    }
    #[test]
    fn empty_inventory_cannot_expand_unbounded_source_comparisons() {
        let mut i = known();
        i.inventory.sources = (0..=MAX_UNITS)
            .map(|n| SourceRequirement {
                id: format!("s{n}"),
                revision: "revision".into(),
            })
            .collect();
        assert!(compare(&i).is_err());
    }
    #[test]
    fn moving_source_revisions_refuse_and_unexpected_records_are_indexed() {
        let mut i = known();
        i.inventory.sources[0].revision = "main".into();
        assert!(compare(&i).is_err());
        i.inventory.sources[0].revision = "1".repeat(40);
        i.records = (0..2048)
            .map(|n| record(&format!("unexpected-{n}")))
            .collect();
        for n in 0..100 {
            i.inventory.sources.push(SourceRequirement {
                id: format!("unused-{n}"),
                revision: "2".repeat(40),
            });
        }
        let report = compare(&i).unwrap();
        assert_eq!(report.domains[0].source_comparisons.len(), 101);
        assert_eq!(report.domains[0].unexpected_record_ids.len(), 2048);
        assert_eq!(
            report.domains[0].source_comparisons[0]
                .included_record_ids
                .len(),
            2048
        );
    }
    #[test]
    fn all_nine_requirements_remain_explicit() {
        for name in DOMAINS {
            assert!(!requirements(name).is_empty());
        }
    }
}
