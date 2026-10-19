fn main() {
    println!("cargo:rerun-if-env-changed=NAVZEROTH_BUNDLE_IDENTITY_JSON");
    let identity = std::env::var("NAVZEROTH_BUNDLE_IDENTITY_JSON").unwrap_or_default();
    assert!(
        identity.len() <= 65536 && !identity.contains(['\r', '\n']),
        "bundle identity must use canonical single-line JSON"
    );
    println!("cargo:rustc-env=NAVZEROTH_BUNDLE_IDENTITY_JSON={identity}");
    tauri_build::build()
}
