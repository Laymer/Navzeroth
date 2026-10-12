//! The import service (M2.2, M1.5): convergence, policy, freshness, fixture
//! flag, bounds, and error hygiene.
mod common;
use azeroth_core::{
    contract::{ContractKind, ProducerComponent},
    error::{AppError, ErrorCode},
    observation::{ClockTrust, ClockUncertainty, Freshness},
    types::{CharacterId, DatasetNamespace, Millis},
};
use azeroth_ingest::{
    anav1::{MAX_DOCUMENT_BYTES, MAX_TEXT_BYTES},
    import::{
        FileFormat, ImportOptions, import_file, import_saved_variables, import_text, normalize,
    },
    savedvars::MAX_FILE_BYTES,
};
use common::*;

fn trusted(now: i64, max_age_ms: u64) -> ImportOptions {
    options(Some(now), ClockTrust::Trusted, max_age_ms)
}

#[test]
fn text_file_and_saved_variables_converge_on_one_result() {
    let scratch = Scratch::new("converge");
    let text = transport("anav1-character-snapshot.txt");
    let sv = transport("savedvars-valid.lua");
    let sv_escapes = transport("savedvars-valid-escapes.lua");
    let options = trusted(1_791_792_000_000, 60_000);

    let from_text = import_text(&text, &options).unwrap();
    let from_text_file = import_file(
        &scratch.write("t.txt", &text),
        &FileFormat::Anav1Text,
        &options,
    )
    .unwrap();
    let sv_format = FileFormat::SavedVariables(location());
    let from_sv = import_saved_variables(&sv, &location(), &options).unwrap();
    let from_sv_file = import_file(&scratch.write("s.lua", &sv), &sv_format, &options).unwrap();
    let from_sv_escapes = import_saved_variables(&sv_escapes, &location(), &options).unwrap();
    // The same document reached through every route gives the identical result.
    assert_eq!(from_text, from_text_file);
    assert_eq!(from_text, from_sv);
    assert_eq!(from_text, from_sv_file);
    assert_eq!(from_text, from_sv_escapes);
    // The shared function is the one entry point all of them use.
    let document = azeroth_ingest::anav1::decode_text(&text).unwrap();
    assert_eq!(normalize(document, &options).unwrap(), from_text);
    // CRLF line endings change nothing, for the transport and for the file.
    let crlf_text = format!("\r\n{}\r\n", text.trim_end());
    assert_eq!(import_text(&crlf_text, &options).unwrap(), from_text);
    for original in [&sv, &sv_escapes] {
        let crlf = original.replace('\n', "\r\n");
        assert_ne!(&crlf, original);
        assert_eq!(
            import_saved_variables(&crlf, &location(), &options).unwrap(),
            from_text
        );
    }
}

#[test]
fn result_surfaces_fixture_namespace_and_producer_facts() {
    // CASE-015, import half: the caller can see this is a fixture in the demo namespace.
    let result = import_text(&transport("anav1-character-snapshot.txt"), &no_clock()).unwrap();
    assert_eq!(result.kind, ContractKind::CharacterSnapshot);
    assert!(result.fixture && result.is_fixture_or_demo());
    assert_eq!(result.namespace, DatasetNamespace::Demo);
    assert_eq!(result.build.edition.as_str(), "wow_forever");
    assert_eq!(result.build.build.as_str(), "UNVERIFIED_DEMO");
    assert_eq!(result.producer_component, ProducerComponent::TestFixture);
    assert_eq!(
        result.producer_session_id.as_ref().map(|s| s.as_str()),
        Some("demo-session-001")
    );
    assert_eq!(result.producer_sequence.map(|s| s.get()), Some(12));
    assert_eq!(
        result.created_at_utc_ms.map(|t| t.get()),
        Some(1_791_792_000_000)
    );
    let summary = &result.snapshots[&CharacterId::new("demo-character-paladin").unwrap()];
    assert_eq!(summary.snapshot_id.as_str(), "demo-snapshot-001");
    assert_eq!(
        summary.observed_at_utc_ms.map(|t| t.get()),
        Some(1_791_791_990_000)
    );

    // A fixture may use the reference namespace and stays a fixture.
    let wrapped = format!("ANAV1:{}", contract("reference-namespace-fixture.json"));
    let result = import_text(&wrapped, &no_clock()).unwrap();
    assert!(result.fixture && result.is_fixture_or_demo());
    assert_eq!(result.namespace, DatasetNamespace::Reference);

    // A document that is neither a fixture nor demo is not flagged. Built in memory only.
    let live = contract("reference-namespace-fixture.json")
        .replace("\"fixture\": true", "\"fixture\": false")
        .replace("synthetic_fixture", "addon_snapshot")
        .replace("\"test_fixture\"", "\"addon\"");
    let result = import_text(&format!("ANAV1:{live}"), &no_clock()).unwrap();
    assert!(!result.fixture && !result.is_fixture_or_demo());
    assert_eq!(result.producer_component, ProducerComponent::Addon);
}

#[test]
fn freshness_comes_from_the_callers_clock_and_is_never_assumed() {
    let text = transport("anav1-character-snapshot.txt");
    let key = CharacterId::new("demo-character-paladin").unwrap();
    let freshness =
        |options: &ImportOptions| import_text(&text, options).unwrap().snapshots[&key].freshness;
    let age = |ms| Millis::new(ms).unwrap();
    assert_eq!(
        freshness(&trusted(1_791_792_000_000, 60_000)),
        Freshness::Fresh {
            age_ms: age(10_000)
        }
    );
    assert_eq!(
        freshness(&trusted(1_791_792_000_000, 5_000)),
        Freshness::Stale {
            age_ms: age(10_000)
        }
    );
    // No clock, or one that is not trusted: uncertain, never fresh.
    let uncertain = Freshness::Uncertain {
        reason: ClockUncertainty::MissingOrUntrustedTime,
    };
    assert_eq!(freshness(&no_clock()), uncertain);
    assert_eq!(
        freshness(&options(
            Some(1_791_792_000_000),
            ClockTrust::Uncertain,
            60_000
        )),
        uncertain
    );
    // A clock earlier than the capture is reported as reversed, not as zero age.
    assert_eq!(
        freshness(&trusted(1_791_791_000_000, 60_000)),
        Freshness::Uncertain {
            reason: ClockUncertainty::ClockReversed
        }
    );
}

#[test]
fn party_snapshots_report_each_participant_in_deterministic_order() {
    let text = transport("anav1-party-snapshot.txt");
    let result = import_text(&text, &no_clock()).unwrap();
    assert_eq!(result.kind, ContractKind::PartySnapshot);
    let keys: Vec<_> = result
        .snapshots
        .keys()
        .map(|k| k.as_str().to_owned())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert!(keys.len() >= 2);
    assert!(
        result
            .snapshots
            .values()
            .all(|s| matches!(s.freshness, Freshness::Uncertain { .. }))
    );
    assert_eq!(import_text(&text, &no_clock()).unwrap(), result);
}

#[test]
fn accepted_kinds_are_a_caller_policy_checked_before_decoding() {
    let plan = format!("ANAV1:{}", contract("route-plan.json"));
    let err = import_text(&plan, &no_clock()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert_eq!(
        err.to_string(),
        "Exchange kind is not accepted by this transport"
    );
    // The caller can widen the policy; the result then carries no snapshots.
    let mut widened = no_clock();
    widened.accepted_kinds.insert(ContractKind::RoutePlan);
    let result = import_text(&plan, &widened).unwrap();
    assert_eq!(result.kind, ContractKind::RoutePlan);
    assert!(result.snapshots.is_empty());
    // An unaccepted kind with an otherwise invalid body is refused for its kind.
    let hollow = "ANAV1:{\"format\":\"azeroth-navigator\",\"kind\":\"reference_dataset\",\"schema_version\":1}";
    assert_eq!(
        import_text(hollow, &no_clock()).unwrap_err().to_string(),
        "Exchange kind is not accepted by this transport"
    );
    // An unknown kind is reported by the contract decoder.
    let unknown =
        "ANAV1:{\"format\":\"azeroth-navigator\",\"kind\":\"lua_chunk\",\"schema_version\":1}";
    assert_eq!(
        import_text(unknown, &no_clock()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn the_four_mib_ceiling_applies_even_to_kinds_the_contract_allows_more() {
    let mut options = no_clock();
    options
        .accepted_kinds
        .insert(ContractKind::ReferenceDataset);
    let padded = |bytes: usize| {
        let head = "{\"format\":\"azeroth-navigator\",\"kind\":\"reference_dataset\",\"schema_version\":1,\"x\":\"";
        let tail = "\"}";
        format!(
            "ANAV1:{head}{}{tail}",
            "a".repeat(bytes - head.len() - tail.len())
        )
    };
    // Exactly at the ceiling the transport lets the contract decide (it rejects the shape).
    let at_limit = import_text(&padded(MAX_DOCUMENT_BYTES), &options).unwrap_err();
    assert_eq!(at_limit.code, ErrorCode::InvalidInput);
    // One byte over: IMPORT_TOO_LARGE, before JSON parsing and before the kind policy.
    let over = import_text(&padded(MAX_DOCUMENT_BYTES + 1), &options).unwrap_err();
    assert_eq!(over.code, ErrorCode::ImportTooLarge);
    assert_eq!(
        import_text(&padded(MAX_DOCUMENT_BYTES + 1), &no_clock())
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    // The same ceiling holds for an export read out of a SavedVariables string.
    let sv = format!(
        "AzerothNavigatorDB = {{ [\"export\"] = '{}' }}",
        &padded(MAX_DOCUMENT_BYTES + 1)
    );
    assert_eq!(
        import_saved_variables(&sv, &location(), &options)
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
}

#[test]
fn json_depth_33_is_invalid_input_through_every_route() {
    let text = transport("anav1-json-depth-33.txt");
    assert_eq!(
        import_text(&text, &no_clock()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let sv = format!(
        "AzerothNavigatorDB = {{ [\"export\"] = '{}' }}",
        text.trim_end()
    );
    assert_eq!(
        import_saved_variables(&sv, &location(), &no_clock())
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn file_import_is_bounded_and_refuses_non_regular_files() {
    let scratch = Scratch::new("files");
    let text = FileFormat::Anav1Text;
    let sv = FileFormat::SavedVariables(location());
    let big = scratch.0.join("big.txt");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(MAX_TEXT_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(
        import_file(&big, &text, &no_clock()).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    let big_sv = scratch.0.join("big.lua");
    std::fs::File::create(&big_sv)
        .unwrap()
        .set_len(MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(
        import_file(&big_sv, &sv, &no_clock()).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    // A directory, a missing path and non UTF-8 bytes are structured errors.
    assert_eq!(
        import_file(&scratch.0, &text, &no_clock())
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    let missing = import_file(&scratch.0.join("missing.txt"), &text, &no_clock()).unwrap_err();
    assert_eq!(missing.code, ErrorCode::InvalidInput);
    assert!(missing.retryable);
    let binary = scratch.write("binary.txt", [0xff, 0xfe, 0x00]);
    assert_eq!(
        import_file(&binary, &text, &no_clock()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    // A byte order mark is rejected in both formats.
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(transport("anav1-character-snapshot.txt").as_bytes());
    assert!(import_file(&scratch.write("bom.txt", &bom), &text, &no_clock()).is_err());
    #[cfg(unix)]
    {
        let real = scratch.write("real.txt", transport("anav1-character-snapshot.txt"));
        let link = scratch.0.join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(import_file(&real, &text, &no_clock()).is_ok());
        assert_eq!(
            import_file(&link, &text, &no_clock()).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
}

fn assert_clean(error: &AppError, sentinels: &[&str]) {
    let shown = format!("{error} {error:?}");
    for sentinel in sentinels {
        assert!(
            !shown.contains(sentinel),
            "error echoes {sentinel}: {shown}"
        );
    }
}

#[test]
fn errors_do_not_echo_imported_content() {
    let scratch = Scratch::new("echo");
    let (a, b, c) = (
        "SENTINEL-private-alpha",
        "SENTINEL-private-beta",
        "SENTINEL-private-gamma",
    );
    let path_sentinel = "SENTINEL-private-path";
    let snapshot = contract("character-snapshot.json");
    let head = "\"format\":\"azeroth-navigator\",\"schema_version\":1";
    let texts = [
        format!("{a}:{{}}"),
        format!("ANAV1:{{{a}"),
        format!("ANAV1:{snapshot} {a}"),
        format!("ANAV1:{{{head},\"kind\":\"{a}\"}}"),
        format!("ANAV1:{{\"format\":\"{a}\",\"kind\":\"route_plan\",\"schema_version\":1}}"),
        format!("ANAV1:{{\"{a}\":1,{head},\"kind\":\"character_snapshot\"}}"),
        // A control character inside an identifier fails deep inside the contract.
        format!(
            "ANAV1:{}",
            snapshot.replace("demo-character-paladin", &format!("{b}\\u0001"))
        ),
    ];
    for text in &texts {
        assert_clean(&import_text(text, &no_clock()).unwrap_err(), &[a, b, c]);
        // The same content reached through a SavedVariables string is equally clean.
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        let sv = format!("AzerothNavigatorDB = {{ ['export'] = '{escaped}' }}");
        assert_clean(
            &import_saved_variables(&sv, &location(), &no_clock()).unwrap_err(),
            &[a, b, c],
        );
    }
    let sources = [
        format!("{a} = 1"),
        format!("T = {{ {a} = os.execute('{b}') }}"),
        format!("T = {{ ['{a}'] = '{b}' ..  '{c}' }}"),
        format!("T = '{a}\\q'"),
        format!("T = '{a}"),
        format!("T = {{ ['{a}'] = 1, ['{a}'] = 2 }}"),
        format!("T = 0x{a}"),
        format!("T = [[{c}]]"),
        format!("AzerothNavigatorDB = {{ ['export'] = '{a}' }}"),
        format!("AzerothNavigatorDB = {{ ['{a}'] = '{b}' }}"),
    ];
    for source in &sources {
        assert_clean(
            &import_saved_variables(source, &location(), &no_clock()).unwrap_err(),
            &[a, b, c],
        );
        if let Err(error) = azeroth_ingest::savedvars::parse(source) {
            assert_clean(&error, &[a, b, c]);
        }
    }
    // File errors never include the path.
    let missing = scratch.0.join(path_sentinel);
    for format in [
        FileFormat::Anav1Text,
        FileFormat::SavedVariables(location()),
    ] {
        assert_clean(
            &import_file(&missing, &format, &no_clock()).unwrap_err(),
            &[path_sentinel],
        );
    }
    let invalid_utf8 = scratch.write(&format!("{path_sentinel}-bytes"), [0xff]);
    assert_clean(
        &import_file(&invalid_utf8, &FileFormat::Anav1Text, &no_clock()).unwrap_err(),
        &[path_sentinel],
    );
}
