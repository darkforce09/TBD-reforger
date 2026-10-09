//! Unit tests for the build output folder layout of [`crate::build_output`]: the purpose
//! subfolder names, where they land, and the retired root-level names.

use super::*;

/// Entries cargo (and the tools that share a target directory with it) writes directly inside a
/// `CARGO_TARGET_DIR`: the built-in profile folders, the shared build, package, documentation and
/// temporary folders, the bookkeeping files, and the folders of the cargo plugins this workspace
/// and its editors run.
const CARGO_OWNED_ENTRY_NAMES: &[&str] = &[
    "debug",
    "release",
    "doc",
    "package",
    "tmp",
    "build",
    "deps",
    "examples",
    "incremental",
    ".fingerprint",
    ".rustc_info.json",
    "CACHEDIR.TAG",
    ".cargo-lock",
    ".package-cache",
    ".future-incompat-report.json",
    "flycheck0",
    "nextest",
    "llvm-cov-target",
    "criterion",
    "wasm-bindgen",
    // This repository's own stamp beside cargo's entries.
    ".tbd-build-abi",
];

/// Every name this repository's tools create directly inside a target directory: the fixed purpose
/// subfolders, a slice gate's frontend folder for an example slice, and the wave driver's run lane.
fn names_tools_create() -> Vec<String> {
    let mut names: Vec<String> = PURPOSE_SUBFOLDERS.iter().map(|s| s.to_string()).collect();
    names.push(format!("{GATE_SLICE_FRONTEND_PREFIX}T-1"));
    names.push(RUN_TARGET_SUBDIR.to_string());
    names
}

/// Profile folder names the workspace manifest can make cargo write: `dev` and `test` write
/// `debug`, `bench` writes `release`, and a custom `[profile.<name>]` writes `<name>`.
fn workspace_profile_folders() -> Vec<String> {
    let manifest = repository_root::find_repository_root()
        .expect("repository root")
        .join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let mut folders = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("[profile.") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| *c != ']' && *c != '.')
            .collect();
        folders.push(name.clone());
        folders.push(
            match name.as_str() {
                "dev" | "test" => "debug",
                "bench" => "release",
                other => other,
            }
            .to_string(),
        );
    }
    folders
}

/// Every target triple the installed `rustc` knows; cargo writes a folder per triple it builds for.
fn rustc_target_triples() -> Vec<String> {
    let out = std::process::Command::new("rustc")
        .args(["--print", "target-list"])
        .output()
        .expect("rustc --print target-list must run: the collision proof needs the triple list");
    assert!(out.status.success(), "rustc --print target-list failed");
    let triples: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    assert!(
        triples.iter().any(|t| t == "wasm32-unknown-unknown"),
        "the triple list is not the one this workspace builds against: {} entries",
        triples.len()
    );
    triples
}

/// No purpose subfolder can be mistaken for, or overwrite, an entry cargo writes itself.
#[test]
fn purpose_subfolders_never_collide_with_cargo_entries() {
    let triples = rustc_target_triples();
    let profiles = workspace_profile_folders();
    for name in names_tools_create() {
        assert!(
            !CARGO_OWNED_ENTRY_NAMES.contains(&name.as_str()),
            "{name} is an entry cargo writes inside a target directory"
        );
        assert!(
            !triples.contains(&name),
            "{name} is a target triple cargo writes a folder for"
        );
        assert!(
            !profiles.contains(&name),
            "{name} is a profile folder of the workspace manifest"
        );
        assert!(
            !name.starts_with('.') && !name.contains('/'),
            "{name} must be one plain path component"
        );
    }
    let mut seen = names_tools_create();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), names_tools_create().len(), "duplicate names");
}

/// The names are pinned, and each one lands one level below `target/`, never beside it.
#[test]
fn purpose_subfolders_nest_inside_the_build_output_folder() {
    assert_eq!(BUILD_OUTPUT_FOLDER, "target");
    assert_eq!(
        PURPOSE_SUBFOLDERS,
        [
            "dev-api",
            "gate-trunk",
            "gate-dist-frontend",
            "gate-check",
            "gate-schema",
            "gate-api",
            "gate-frontend",
            "gate-tools",
            "ci",
            "dev-mcpd",
            "db-selftest",
        ]
    );
    assert_eq!(GATE_SLICE_FRONTEND_PREFIX, "gate-slice-frontend-");
    let root = Path::new("/repo");
    for name in names_tools_create() {
        let folder = build_output_subfolder(root, &name);
        assert_eq!(folder, Path::new("/repo/target").join(&name));
        assert_eq!(folder.parent(), Some(Path::new("/repo/target")));
        assert!(
            !is_retired_root_level_build_folder(&name),
            "{name} reads as a retired root-level folder"
        );
    }
    // The gate sweep selects by prefix: every gate folder carries it, the permanent ones do not.
    for name in names_tools_create() {
        let is_gate = name.starts_with(GATE_SUBFOLDER_PREFIX);
        let permanent = name == DEV_API_SUBFOLDER
            || name == CONTINUOUS_INTEGRATION_SUBFOLDER
            || name == MCP_DAEMON_SUBFOLDER
            || name == DATABASE_SELFTEST_SUBFOLDER
            || name == RUN_TARGET_SUBDIR;
        assert_ne!(is_gate, permanent, "{name}");
    }
}

/// The retired root-level names are recognised; live folders and slice folders are not.
#[test]
fn retired_root_level_folders_are_recognised() {
    for retired in [
        "target-dev-api",
        "target-ci",
        "target-dev-mcpd",
        "target-mk-db-selftest",
        "target-gate-check",
        "target-gate-trunk",
        "target-gate-schema-T422",
        "target-gate-slice-frontend-T-1",
        "dist-gate-frontend",
    ] {
        assert!(is_retired_root_level_build_folder(retired), "{retired}");
    }
    for kept in [
        "target",
        "target-T-454",
        "target-T-582-api",
        "target-container",
        "target-dev-api-notes",
        "dist",
        "gate-check",
    ] {
        assert!(!is_retired_root_level_build_folder(kept), "{kept}");
    }
}
