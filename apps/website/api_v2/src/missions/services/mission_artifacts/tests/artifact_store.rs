//! Artifact compilation's gate and the findings a refusal reports.

use super::{MAX_REPORTED_FINDINGS, reported_findings};

const SRC: &str = include_str!("../artifact_store.rs");

fn production() -> &'static str {
    SRC.split("#[cfg(test)]").next().unwrap()
}

/// An artifact compiles through the catalogued gate against the current modpack's catalog, so an
/// over-capacity version never becomes an artifact. The empty-catalog flatten would let one ship.
///
/// RED: swap back to the no-argument flatten, or drop the catalog snapshot.
#[test]
fn artifacts_compile_through_the_catalogued_gate() {
    let production = production();
    assert!(production.contains("load_catalog_snapshot(connection)"));
    assert!(production.contains(
        "flatten_to_mod_document_with_catalog(mission, payload.as_bytes(), &snapshot.catalog)"
    ));
    let stripped = production.replace("flatten_to_mod_document_with_catalog", "");
    assert!(!stripped.contains("flatten_to_mod_document("));
}

/// The compile's findings are lifted before the document is serialized (they are not part of
/// the bytes) and stored with the artifact; a finding never refuses the compile.
#[test]
fn compile_findings_are_stored_with_the_artifact_and_never_refuse() {
    let production = production();
    let collapsed = production.split_whitespace().collect::<Vec<_>>().join(" ");
    let lifted = collapsed
        .find("document .diagnostics .iter()")
        .expect("findings are read off the document");
    let serialized = collapsed
        .find("serde_json::to_vec(&document)")
        .expect("the bytes are serialized");
    assert!(
        lifted < serialized,
        "findings must be lifted before serialization"
    );
    let logged = production
        .split("for finding in &document.diagnostics {")
        .nth(1)
        .and_then(|rest| rest.split("\n            }").next())
        .expect("every finding of a new artifact is logged");
    assert!(logged.contains("tracing::warn!") && !logged.contains("ApiError"));
}

#[test]
fn refusals_report_each_distinct_finding_once_with_the_full_count() {
    let findings: Vec<String> = (0..30)
        .flat_map(|slot| {
            let finding = format!("/slots/{slot}/uid: \"\" is shorter than 1 character");
            [finding.clone(), finding]
        })
        .collect();
    let (count, unique) = reported_findings(findings);
    assert_eq!(count, 30);
    assert_eq!(unique[0], "/slots/0/uid: \"\" is shorter than 1 character");
    assert!(
        MAX_REPORTED_FINDINGS < count,
        "the cap bounds the body, the count stays whole"
    );
}
