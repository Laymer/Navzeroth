//! Build only retained local sources. Never calls CMake, a shell or a downloader.
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "Offline patch no longer matches pinned source"
    );
    text.replacen(from, to, 1)
}

fn main() {
    let parser = PathBuf::from("../../vendor/wowdbdefs-rs");
    println!("cargo:rerun-if-changed=../../vendor/wowdbdefs-rs");
    let parser_manifest = fs::read(parser.join("PROVENANCE.json")).unwrap();
    let parser_files: serde_json::Value = serde_json::from_slice(&parser_manifest).unwrap();
    for entry in parser_files["files"].as_array().unwrap() {
        let bytes = fs::read(parser.join(entry["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            entry["sha256"].as_str().unwrap(),
            "Patched parser source hash mismatch"
        );
    }
    let mut sources: Vec<_> = fs::read_dir("src")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "rs"))
        .collect();
    sources.extend([
        PathBuf::from("build.rs"),
        PathBuf::from("native/offline.cpp"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("../../Cargo.lock"),
    ]);
    sources.sort();
    let mut source_hash = Sha256::new();
    source_hash.update(b"NAVZEROTH-LOCAL-DATA-SOURCE-V1\0");
    source_hash.update(parser_manifest);
    for source in sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let name = source.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(&source).unwrap().replace("\r\n", "\n");
        source_hash.update((name.len() as u64).to_le_bytes());
        source_hash.update(name.as_bytes());
        source_hash.update((text.len() as u64).to_le_bytes());
        source_hash.update(text.as_bytes());
    }
    println!(
        "cargo:rustc-env=NAVZEROTH_LOCAL_DATA_PROFILE_SHA256={:x}",
        source_hash.finalize()
    );
    // Projector identity: exactly the code that maps decoded tables to projection
    // records (including the core record types, validation and slot mapping).
    let mut projector = Sha256::new();
    projector.update(b"NAVZEROTH-LOCAL-PROJECTOR-V1\0");
    for source in ["src/projection.rs", "../core/src/local_data_projection.rs"] {
        println!("cargo:rerun-if-changed={source}");
        let text = fs::read_to_string(source).unwrap().replace("\r\n", "\n");
        projector.update((source.len() as u64).to_le_bytes());
        projector.update(source.as_bytes());
        projector.update((text.len() as u64).to_le_bytes());
        projector.update(text.as_bytes());
    }
    println!(
        "cargo:rustc-env=NAVZEROTH_PROJECTOR_PROFILE_SHA256={:x}",
        projector.finalize()
    );
    let metadata = PathBuf::from("vendor/wowdbdefs");
    println!("cargo:rerun-if-changed=vendor/wowdbdefs");
    let bytes = fs::read(metadata.join("UPSTREAM.json")).unwrap();
    let inventory: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        inventory["revision"],
        "b9a63096cdd99b26967f17f9515377ae62cadfd7"
    );
    let mut definitions = String::from("pub(crate) static DEFINITIONS: &[(&str, &str)] = &[\n");
    for entry in inventory["files"].as_array().unwrap() {
        let name = entry["path"].as_str().unwrap();
        let source = fs::read(metadata.join(name)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&source)),
            entry["sha256"].as_str().unwrap()
        );
        if let Some(name) = name
            .strip_prefix("definitions/")
            .and_then(|n| n.strip_suffix(".dbd"))
        {
            let path = fs::canonicalize(metadata.join(format!("definitions/{name}.dbd"))).unwrap();
            definitions.push_str(&format!("({name:?}, include_str!({:?})),\n", path));
        }
    }
    definitions.push_str("];\n");
    definitions.push_str(&format!(
        "pub const METADATA_SHA256: &str = {:?};\n",
        format!("{:x}", Sha256::digest(&bytes))
    ));
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("metadata.rs"),
        definitions,
    )
    .unwrap();
    let root = PathBuf::from("vendor/casclib");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("UPSTREAM.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["revision"],
        "38a34665624b8775bb875274b36191b21c38d97b"
    );
    println!("cargo:rerun-if-changed=vendor/casclib");
    println!("cargo:rerun-if-changed=native");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("casclib-offline");
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        let bytes = fs::read(root.join(name)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            file["sha256"].as_str().unwrap(),
            "Retained source hash mismatch"
        );
        let target = output.join(name);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    // All edits affect only OUT_DIR; retained originals remain byte-identical.
    let source = output.join("src/common/FileStream.cpp");
    let mut text = fs::read_to_string(&source).unwrap();
    for signature in [
        "static bool BaseFile_Create(TFileStream * pStream)\n{",
        "static bool BaseFile_Write(TFileStream * pStream, ULONGLONG * pByteOffset, const void * pvBuffer, DWORD dwBytesToWrite)\n{",
    ] {
        text = replace_once(
            &text,
            signature,
            &format!(
                "{signature}\n    // Navzeroth offline/read-only build.\n    SetCascError(ERROR_NOT_SUPPORTED); return false;"
            ),
        );
    }
    text = replace_once(
        &text,
        "    // Re-assemble the stream flags",
        "    // Navzeroth: disallow remote, partial, encrypted and master streams.\n    if(dwProvider != (STREAM_PROVIDER_FLAT | BASE_PROVIDER_FILE) || _tcschr(szFileName, '*') != NULL) { SetCascError(ERROR_NOT_SUPPORTED); return NULL; }\n    dwStreamFlags |= STREAM_FLAG_READ_ONLY;\n    // Re-assemble the stream flags",
    );
    fs::write(source, text).unwrap();
    let source = output.join("src/common/Directory.cpp");
    let text = fs::read_to_string(&source).unwrap();
    fs::write(source, replace_once(&text, "bool MakeDirectory(LPCTSTR szDirectory)\n{", "bool MakeDirectory(LPCTSTR szDirectory)\n{\n    // Navzeroth offline/read-only build.\n    SetCascError(ERROR_NOT_SUPPORTED); return false;")).unwrap();
    let source = output.join("src/CascCommon.h");
    let text = fs::read_to_string(&source).unwrap();
    fs::write(
        source,
        replace_once(
            &text,
            "#include \"hashes/sha1.h\"",
            "// Navzeroth: unused Overwatch SHA1 dependency omitted.",
        ),
    )
    .unwrap();
    let source = output.join("src/common/Common.cpp");
    let text = fs::read_to_string(&source).unwrap();
    let begin = text.find("void CascHash_SHA1(").unwrap();
    let end = begin + text[begin..].find("\n}").unwrap() + 2;
    fs::write(
        source,
        format!(
            "{}\n// Navzeroth: unused Overwatch SHA1 implementation omitted.\n{}",
            &text[..begin],
            &text[end..]
        ),
    )
    .unwrap();

    let cmake = fs::read_to_string(root.join("CMakeLists.txt")).unwrap();
    let sources = cmake
        .split("set(SRC_FILES")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap();
    let mut cpp = cc::Build::new();
    cpp.cpp(true)
        .include(output.join("src"))
        .warnings(false)
        .define("CASCLIB_NO_AUTO_LINK_LIBRARY", None)
        .define("CASCLIB_NODEBUG", None)
        .flag_if_supported("-std=c++11");
    let windows = env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows";
    if windows {
        cpp.define("UNICODE", None).define("_UNICODE", None);
    } else {
        cpp.define("LPDWORD", "PDWORD");
    } // Pinned upstream Linux portability typo.
    for name in sources.split_whitespace().filter(|s| s.ends_with(".cpp")) {
        if !name.contains("overwatch/")
            && !name.ends_with("sha1.cpp")
            && !name.ends_with("CascRootFile_OW.cpp")
        {
            cpp.file(output.join(name));
        }
    }
    cpp.file("native/offline.cpp")
        .compile("navzeroth_casc_offline");
    let mut c = cc::Build::new();
    c.warnings(false).include(output.join("src/zlib"));
    c.file(output.join("src/jenkins/lookup3.c"));
    for name in [
        "adler32", "crc32", "inffast", "inflate", "inftrees", "zutil",
    ] {
        c.file(output.join(format!("src/zlib/{name}.c")));
    }
    c.compile("navzeroth_casc_inflate");
    if windows {
        // Upstream provider objects retain these references; offline gate prevents use.
        println!("cargo:rustc-link-lib=wininet");
        println!("cargo:rustc-link-lib=ws2_32");
    }
}
