//! The artifact byte response: its entity tag, and the findings that ride alongside the bytes.

use serde_json::json;

use super::{artifact_document_response, diagnostics_headers};
use crate::missions::services::mission_artifacts::artifact_store::ArtifactDocument;
use crate::missions::services::mission_compile::{
    COMPILE_DIAGNOSTICS_COUNT_HEADER, COMPILE_DIAGNOSTICS_RULES_HEADER,
};

/// A clean compile still advertises its count, and names no rules.
///
/// RED: skip the count insert when the findings are empty.
#[test]
fn a_clean_artifact_carries_diagnostics_count_zero() {
    let headers = diagnostics_headers(&json!([]));
    assert_eq!(headers[COMPILE_DIAGNOSTICS_COUNT_HEADER], "0");
    assert!(headers.get(COMPILE_DIAGNOSTICS_RULES_HEADER).is_none());
}

#[test]
fn each_fired_rule_is_named_once_in_first_fired_order() {
    let findings = json!([
        { "rule_id": "zone_quantised", "severity": "warning" },
        { "rule_id": "loadout_substituted", "severity": "info" },
        { "rule_id": "zone_quantised", "severity": "warning" },
    ]);
    let headers = diagnostics_headers(&findings);
    assert_eq!(headers[COMPILE_DIAGNOSTICS_COUNT_HEADER], "3");
    assert_eq!(
        headers[COMPILE_DIAGNOSTICS_RULES_HEADER],
        "zone_quantised,loadout_substituted"
    );
}

#[test]
fn the_response_carries_the_bytes_their_tag_and_the_findings() {
    let response = artifact_document_response(ArtifactDocument {
        bytes: b"{}".to_vec(),
        sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".into(),
        diagnostics: json!([{ "rule_id": "zone_quantised" }]),
    });
    let headers = response.headers();
    assert_eq!(headers["content-type"], "application/json");
    assert_eq!(
        headers["etag"],
        "\"44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a\""
    );
    assert_eq!(headers[COMPILE_DIAGNOSTICS_COUNT_HEADER], "1");
    assert_eq!(headers[COMPILE_DIAGNOSTICS_RULES_HEADER], "zone_quantised");
}
