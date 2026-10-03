//! Engineering laws — `CLAUDE.md` laws 6 and 7 held over the whole repository.
//!
//! **Role:** asserts that the checkout keeps the structural laws: production files at or under
//! 500 lines and test files at or under 1000, unit tests only in sibling files, no exemption
//! mechanism for either rule, the dependency direction between the website crates, and a
//! `failpoints` feature that only test builds compile. The crate firewalls (wgpu, browser crates,
//! the graphics category's map nouns, `#[wasm_bindgen` placement) are `cargo xtask verify
//! crate-tiers`.
//! **Position:** an integration binary of `api`, run by
//! `cargo xtask db test-it --test engineering_laws`; it needs no database. Every law is judged by
//! `repository_laws`, the same code `cargo xtask verify file-length` prints, so the gates and
//! this binary never disagree about the tree.
//! **Signals & state:** none; each case reads the checkout and asserts.
//! **Invariants:** the repository root comes from `CARGO_MANIFEST_DIR`; every case asserts that
//! its scan read something before it asserts that the scan found nothing, so a moved or empty
//! tree fails instead of passing.

use std::path::{Path, PathBuf};

use repository_laws::cargo_manifest::read_manifest;
use repository_laws::crate_dependencies::{
    API_RULE, DependencyFinding, FRONTEND_RULE, crate_dependency_findings, rule_findings,
    test_only_feature_findings,
};
use repository_laws::exemption_mechanisms::scan_exemption_mechanisms;
use repository_laws::file_length::scan_file_lengths;
use repository_laws::sibling_test_placement::scan_inline_test_modules;
use repository_laws::source_roots::{is_test_file, repository_relative};
use verification_core::{Pattern, scan};

/// The repository root, found above the API package's manifest folder.
fn repository_root() -> PathBuf {
    repository_layout::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
}

/// The manifests of the API: the application's and every API crate's under `crates/api/`.
fn api_manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = vec![root.join("apps/api/Cargo.toml")];
    let crates = root.join("crates/api");
    let mut crate_manifests: Vec<PathBuf> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", crates.display()))
        .map(|entry| entry.expect("directory entry").path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .collect();
    crate_manifests.sort();
    assert!(
        crate_manifests.len() >= 20,
        "found only {} API crate manifest(s) under crates/api/",
        crate_manifests.len()
    );
    manifests.extend(crate_manifests);
    manifests
}

/// Render dependency findings one per line.
fn rendered(findings: &[DependencyFinding]) -> String {
    findings
        .iter()
        .map(DependencyFinding::rendered)
        .collect::<Vec<_>>()
        .join("\n")
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

#[test]
fn engineering_laws_frontend_does_not_link_the_api() {
    let edges =
        rule_findings(&repository_root(), &FRONTEND_RULE).expect("the frontend manifest reads");
    assert!(edges.is_empty(), "{}", rendered(&edges));
}

#[test]
fn engineering_laws_api_depends_on_no_graphics_or_frontend_crate() {
    let root = repository_root();
    // The mission domain's edge to the mission crates sits in the missions crate.
    let missions = read_manifest(&root.join("crates/api/api_missions/Cargo.toml"))
        .expect("the missions crate manifest reads");
    assert!(
        missions
            .dependencies
            .iter()
            .any(|edge| edge.package == "mission_compiler" && !edge.is_dev_dependency()),
        "the manifest reader found no mission_compiler edge in api_missions, so it read nothing"
    );
    let edges = rule_findings(&root, &API_RULE).expect("the api manifest reads");
    assert!(edges.is_empty(), "{}", rendered(&edges));
    // The application's rule holds for every API crate it is assembled from.
    let forbidden: Vec<String> = api_manifests(&root)
        .iter()
        .flat_map(|path| {
            let manifest = read_manifest(path).expect("an API manifest reads");
            let relative = repository_relative(&root, path);
            manifest
                .dependencies
                .into_iter()
                .filter(|edge| API_RULE.forbidden_packages.contains(&edge.package.as_str()))
                .map(move |edge| {
                    format!(
                        "{relative}:{}: [{}] depends on {} — {}",
                        edge.line_no, edge.table, edge.package, API_RULE.reason
                    )
                })
        })
        .collect();
    assert!(forbidden.is_empty(), "{}", forbidden.join("\n"));
}

#[test]
fn engineering_laws_crate_directions_hold() {
    let root = repository_root();
    let all = crate_dependency_findings(&root).expect("every website manifest reads");
    assert!(all.is_empty(), "{}", rendered(&all));
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

    // Every API manifest that compiles the registry in (the application's and the API crates'
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

    // The deploy build compiles the server with no feature flag at all.
    let deploy = root.join("tools/commands/deployment/src");
    let sources = scan::walk_files(&[&deploy], |path| {
        path.extension().is_some_and(|extension| extension == "rs")
    })
    .expect("the deploy command sources walk");
    let build = Pattern::regex(r"cargo build[^\n]*-p api").unwrap();
    let builds = scan::matching_lines(&build, &sources).expect("the deploy sources read");
    assert!(
        !builds.is_empty(),
        "no deploy source builds api, so the flag check read nothing"
    );
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
