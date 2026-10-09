//! Engineering laws — `CLAUDE.md` laws 6 and 7 held over the whole repository.
//!
//! **Role:** asserts that the checkout keeps the structural laws: production files at or under
//! 500 lines and test files at or under 1000, unit tests only in sibling files, no exemption
//! mechanism for either rule, and a `failpoints` feature that only test builds compile. The
//! dependency direction of every crate, the applications included (the category matrix, no edge
//! onto an application package, the shell crate order, the rendering-stack clause of the offline
//! service worker and the API crates), and the crate firewalls (wgpu, browser crates, the graphics
//! category's map nouns, `#[wasm_bindgen` placement) are `cargo xtask verify crate-tiers` and
//! `cargo xtask verify frontend-layering`.
//! **Position:** an integration binary of `api_server`, run by
//! `cargo xtask db test-it --test engineering_laws`; it needs no database. Every law is judged by
//! `repository_laws`, the same code `cargo xtask verify file-length` prints, so the gates and
//! this binary never disagree about the tree.
//! **Signals & state:** none; each case reads the checkout and asserts.
//! **Invariants:** the repository root comes from `CARGO_MANIFEST_DIR`; every case asserts that
//! its scan read something before it asserts that the scan found nothing, so a moved or empty
//! tree fails instead of passing.

use std::path::{Path, PathBuf};

use repository_laws::cargo_manifest::read_manifest;
use repository_laws::exemption_mechanisms::scan_exemption_mechanisms;
use repository_laws::file_length::scan_file_lengths;
use repository_laws::sibling_test_placement::scan_inline_test_modules;
use repository_laws::source_roots::{is_test_file, repository_relative};
use repository_laws::test_only_features::test_only_feature_findings;
use verification_core::{Pattern, scan};

/// The repository root, found above the API package's manifest folder.
fn repository_root() -> PathBuf {
    repository_root::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
}

/// The manifests of the API: every crate's under `crates/api/`, the server's among them, each
/// read once.
fn api_manifests(root: &Path) -> Vec<PathBuf> {
    let crates = root.join("crates/api");
    let mut manifests: Vec<PathBuf> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", crates.display()))
        .map(|entry| entry.expect("directory entry").path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .collect();
    manifests.sort();
    assert!(
        manifests.len() >= 20,
        "found only {} API crate manifest(s) under crates/api/",
        manifests.len()
    );
    manifests
}

#[test]
fn engineering_laws_production_files_stay_within_500_lines() {
    let scan = scan_file_lengths(&repository_root()).expect("the file-length walk runs");
    assert!(!scan.files.is_empty(), "the file-length walk read no file");
    let over: Vec<String> = scan
        .violations
        .iter()
        .filter(|violation| !violation.is_test_file())
        .map(|violation| violation.rendered())
        .collect();
    assert!(over.is_empty(), "{}", over.join("\n"));
}

#[test]
fn engineering_laws_test_files_stay_within_1000_lines() {
    let root = repository_root();
    let scan = scan_file_lengths(&root).expect("the file-length walk runs");
    assert!(
        scan.files
            .iter()
            .any(|file| is_test_file(&repository_relative(&root, file))),
        "the file-length walk read no test file"
    );
    let over: Vec<String> = scan
        .violations
        .iter()
        .filter(|violation| violation.is_test_file())
        .map(|violation| violation.rendered())
        .collect();
    assert!(over.is_empty(), "{}", over.join("\n"));
}

#[test]
fn engineering_laws_no_exemption_mechanism_exists() {
    let scan = scan_exemption_mechanisms(&repository_root()).expect("the exemption scan runs");
    assert!(
        scan.files_examined > 0,
        "the exemption scan examined no file"
    );
    let found: Vec<String> = scan.findings.iter().map(|f| f.rendered()).collect();
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn engineering_laws_unit_tests_live_in_sibling_files() {
    let scan = scan_inline_test_modules(&repository_root()).expect("the placement scan runs");
    assert!(
        scan.production_files > 0,
        "the placement scan read no production file"
    );
    let found: Vec<String> = scan.findings.iter().map(|f| f.rendered()).collect();
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// The failpoints count below sums edges over [`api_manifests`]; a manifest read twice would
/// double it, so the list holds the server's manifest exactly once and no path twice.
#[test]
fn engineering_laws_the_api_manifests_are_each_read_once() {
    let root = repository_root();
    let manifests = api_manifests(&root);
    let distinct: std::collections::BTreeSet<&PathBuf> = manifests.iter().collect();
    assert_eq!(distinct.len(), manifests.len(), "{manifests:#?}");
    let server = root.join("crates/api/api_server/Cargo.toml");
    assert_eq!(
        manifests.iter().filter(|path| **path == server).count(),
        1,
        "{manifests:#?}"
    );
}

#[test]
fn engineering_laws_failpoints_are_test_only() {
    let root = repository_root();
    // The feature belongs to the failpoints crate; the crate-anatomy law holds every other
    // manifest to enabling it from `[dev-dependencies]` only.
    let manifest = read_manifest(&root.join("crates/api/api_failpoints/Cargo.toml"))
        .expect("the failpoints manifest reads");
    let findings = test_only_feature_findings(&manifest, "failpoints");
    assert!(findings.is_empty(), "{}", findings.join("\n"));

    // Every API manifest that compiles the registry in (the server's and the other API crates'
    // that test their failpoints) enables the feature from `[dev-dependencies]` only.
    let mut enabling = 0;
    let mut normal: Vec<String> = Vec::new();
    for path in api_manifests(&root) {
        let manifest = read_manifest(&path).expect("an API manifest reads");
        for edge in manifest
            .dependencies
            .iter()
            .filter(|edge| edge.features.iter().any(|feature| feature == "failpoints"))
        {
            enabling += 1;
            if !edge.is_dev_dependency() {
                normal.push(format!(
                    "{}:{}: [{}] {} enables `failpoints` outside [dev-dependencies]",
                    repository_relative(&root, &path),
                    edge.line_no,
                    edge.table,
                    edge.package
                ));
            }
        }
    }
    assert!(
        enabling > 0,
        "no API manifest enables `failpoints`, so the scan read nothing"
    );
    assert!(normal.is_empty(), "{}", normal.join("\n"));

    // The deploy build compiles the server with no feature flag at all. The deploy renders the
    // server's build command from a `-p {} --bin {}` template, and its tests pin the rendered
    // `cargo build --release -p api_server --bin api-server` literally; the package name is
    // word-bounded, so no other package's build satisfies the read check. Every `cargo build`
    // line of the deploy sources, the templates included, is held to no feature flag.
    let deploy = root.join("tools/commands/deployment/src");
    let sources = scan::walk_files(&[&deploy], |path| {
        path.extension().is_some_and(|extension| extension == "rs")
    })
    .expect("the deploy command sources walk");
    let server_build = Pattern::regex(r"cargo build[^\n]*-p api_server\b").unwrap();
    let server_builds =
        scan::matching_lines(&server_build, &sources).expect("the deploy sources read");
    assert!(
        !server_builds.is_empty(),
        "no deploy source names the `-p api_server` build, so the flag check read nothing"
    );
    let build = Pattern::regex(r"cargo build\b").unwrap();
    let builds = scan::matching_lines(&build, &sources).expect("the deploy sources read");
    let flagged: Vec<String> = builds
        .iter()
        .filter(|hit| {
            hit.line.contains("--features")
                || hit.line.contains("--all-features")
                || hit.line.contains(" -F ")
        })
        .map(|hit| hit.rendered())
        .collect();
    assert!(
        flagged.is_empty(),
        "the deploy build passes a feature flag:\n{}",
        flagged.join("\n")
    );
}
