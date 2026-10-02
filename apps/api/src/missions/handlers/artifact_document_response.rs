//! The response carrying an artifact's exact compiled bytes. The SHA-256 of the bytes is the
//! strong entity tag. The compile's findings ride alongside the bytes in two headers, because
//! `mission.schema.json` closes the document root and the bytes must stay exactly what was
//! validated: the finding count is always present (`0` included), so "the compile reported
//! nothing" and "this build does not report" never look alike, and the rule ids that fired are
//! listed once each in first-fired order, omitted when none did.

use axum::http::{HeaderMap, HeaderName, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::missions::services::mission_artifacts::artifact_store::ArtifactDocument;
use crate::missions::services::mission_compile::{
    COMPILE_DIAGNOSTICS_COUNT_HEADER, COMPILE_DIAGNOSTICS_RULES_HEADER,
};

/// The diagnostics headers for an artifact's stored findings.
pub(crate) fn diagnostics_headers(diagnostics: &Value) -> HeaderMap {
    let findings = diagnostics
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static(COMPILE_DIAGNOSTICS_COUNT_HEADER),
        HeaderValue::from(findings.len()),
    );
    let mut rules: Vec<&str> = Vec::new();
    for rule in findings
        .iter()
        .filter_map(|finding| finding["rule_id"].as_str())
    {
        if !rules.contains(&rule) {
            rules.push(rule);
        }
    }
    if !rules.is_empty()
        && let Ok(value) = HeaderValue::from_str(&rules.join(","))
    {
        headers.insert(
            HeaderName::from_static(COMPILE_DIAGNOSTICS_RULES_HEADER),
            value,
        );
    }
    headers
}

/// The artifact's bytes with their entity tag, content type and diagnostics headers.
pub(crate) fn artifact_document_response(document: ArtifactDocument) -> Response {
    let mut headers = diagnostics_headers(&document.diagnostics);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(tag) = HeaderValue::from_str(&format!("\"{}\"", document.sha256)) {
        headers.insert(header::ETAG, tag);
    }
    (headers, document.bytes).into_response()
}

#[cfg(test)]
#[path = "tests/artifact_document_response.rs"]
mod tests;
