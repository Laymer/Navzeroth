//! Explicit synthetic receipt CLI round trip; no production key files selected.
use azeroth_cli::bundled_receipts::execute;
use azeroth_core::{
    release::{FileRef, Target},
    types::{AssetPath, Sha256Hex},
};
use azeroth_local_data::{bundled_receipts::*, receipts::public_key_hex};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn setup() -> Temp {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "anav-bundle-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    let h = |c: char| Sha256Hex::new(c.to_string().repeat(64)).unwrap();
    let payload = Payload {
        schema_version: 1,
        app_version: "1.0.0".into(),
        target: Target::forever_70374(),
        bundle_manifest_sha256: h('a'),
        production_sha256: h('b'),
        tested_source_commit: "c".repeat(40),
        review: FileRef {
            path: AssetPath::new("docs/reviews/synthetic.json").unwrap(),
            bytes: 10,
            sha256: h('d'),
        },
    };
    let policy = Policy {
        format: POLICY_FORMAT.into(),
        schema_version: 1,
        operator_controls_both_keys: true,
        signers: [
            (Role::Operator, "operator", 11u8),
            (Role::ReviewEvidence, "review-evidence", 22u8),
        ]
        .map(|(role, id, seed)| TrustedSigner {
            role,
            key_id: id.into(),
            public_key_hex: public_key_hex(&[seed; 32]),
            identity: "Synthetic operator controls both keys".into(),
            revoked: false,
        }),
    };
    std::fs::write(path.join("payload.json"), encode(&payload).unwrap()).unwrap();
    std::fs::write(path.join("expected.json"), encode(&payload).unwrap()).unwrap();
    std::fs::write(path.join("policy.json"), encode(&policy).unwrap()).unwrap();
    for (id, seed) in [("operator", 11), ("review-evidence", 22)] {
        std::fs::write(
            path.join(format!("{id}.key")),
            format!("{}\n", format!("{seed:02x}").repeat(32)),
        )
        .unwrap();
    }
    Temp(path)
}
fn args(root: &Path, pairs: &[(&str, &str)]) -> Vec<OsString> {
    pairs
        .iter()
        .flat_map(|(k, v)| {
            [
                OsString::from(k),
                if *k == "--key-id" {
                    OsString::from(v)
                } else {
                    root.join(v).into_os_string()
                },
            ]
        })
        .collect()
}

#[test]
fn selected_file_cli_signs_assembles_and_verifies_canonical_bytes() {
    let temp = setup();
    let root = &temp.0;
    for id in ["operator", "review-evidence"] {
        let selected = args(
            root,
            &[
                ("--payload", "payload.json"),
                ("--policy", "policy.json"),
                ("--key-id", id),
                ("--secret-key", &format!("{id}.key")),
            ],
        );
        let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .arg("bundle-receipt-sign")
            .args(&selected)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        assert!(!text.contains(&if id == "operator" {
            "0b".repeat(32)
        } else {
            "16".repeat(32)
        }));
        std::fs::write(root.join(format!("{id}.sig.json")), output.stdout).unwrap();
    }
    let assembled = execute(
        "bundle-receipt-assemble",
        &args(
            root,
            &[
                ("--payload", "payload.json"),
                ("--operator", "operator.sig.json"),
                ("--review-evidence", "review-evidence.sig.json"),
            ],
        ),
    )
    .unwrap();
    std::fs::write(root.join("receipt.json"), &assembled).unwrap();
    let selected = args(
        root,
        &[
            ("--receipt", "receipt.json"),
            ("--policy", "policy.json"),
            ("--expected", "expected.json"),
        ],
    );
    assert_eq!(
        execute("bundle-receipt-verify", &selected).unwrap(),
        assembled
    );
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("bundle-receipt-verify")
        .args(&selected)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, assembled);
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("bundle-receipt-assemble")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let mut stale: Payload = payload(&std::fs::read(root.join("expected.json")).unwrap()).unwrap();
    stale.target.build = "70450".into();
    std::fs::write(root.join("expected.json"), encode(&stale).unwrap()).unwrap();
    assert!(execute("bundle-receipt-verify", &selected).is_err());
}

#[test]
fn cli_refuses_implicit_duplicate_unknown_oversized_and_malformed_inputs() {
    let temp = setup();
    let root = &temp.0;
    assert!(execute("implicit-sign", &[]).is_err());
    assert!(execute("bundle-receipt-sign", &[]).is_err());
    let duplicate = args(
        root,
        &[
            ("--payload", "payload.json"),
            ("--policy", "policy.json"),
            ("--key-id", "operator"),
            ("--key-id", "operator"),
        ],
    );
    assert!(execute("bundle-receipt-sign", &duplicate).is_err());
    let unknown = args(
        root,
        &[
            ("--payload", "payload.json"),
            ("--policy", "policy.json"),
            ("--key-id", "operator"),
            ("--unknown", "operator.key"),
        ],
    );
    assert!(execute("bundle-receipt-sign", &unknown).is_err());
    let mut selected = args(
        root,
        &[
            ("--payload", "payload.json"),
            ("--policy", "policy.json"),
            ("--key-id", "operator"),
            ("--secret-key", "operator.key"),
        ],
    );
    selected[1] = "x".repeat(4097).into();
    assert!(execute("bundle-receipt-sign", &selected).is_err());
    let selected = args(
        root,
        &[
            ("--payload", "payload.json"),
            ("--policy", "policy.json"),
            ("--key-id", "operator"),
            ("--secret-key", "operator.key"),
        ],
    );
    std::fs::write(root.join("operator.key"), b"not a secret key").unwrap();
    assert!(execute("bundle-receipt-sign", &selected).is_err());
    std::fs::write(root.join("operator.key"), vec![b'a'; 1025]).unwrap();
    assert!(execute("bundle-receipt-sign", &selected).is_err());
    std::fs::write(root.join("payload.json"), vec![b' '; MAX_BYTES + 1]).unwrap();
    assert!(execute("bundle-receipt-sign", &selected).is_err());
}
