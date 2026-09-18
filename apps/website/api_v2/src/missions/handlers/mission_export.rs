//! The two derived-document reads on a mission: the strict export envelope an author downloads,
//! and the compiled mod document `/compiled` flattens live and holds to `mission.schema.json`.
//!
//! Both read a mission and serve a DERIVED document rather than editing one, which is why they
//! share a file: the compile is the same engine, and the schema is the same contract.

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::http::{HeaderName, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{MissionMakerUser, ServiceAuth};
use crate::missions::contract::schema_validators::validate_mission_document;
use crate::missions::models::mission::MissionVersion;
use crate::missions::services::cargo_catalog::load_cargo_phys_catalog;
use crate::missions::services::mission_compile::{
    COMPILE_DIAGNOSTICS_COUNT_HEADER, COMPILE_DIAGNOSTICS_RULES_HEADER, CompileError,
    CompileFinding, ModMissionDocument, compile_diagnostics_rules_header,
    flatten_to_mod_document_with_catalog,
};
use crate::missions::services::mission_document::build_mission_doc;
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::can_view;

/// `GET /api/v1/missions/:id/export` — strict export envelope download (mission_maker+).
///
/// @route GET /api/v1/missions/:id/export
pub async fn export_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_view(&maker.0, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let doc = build_mission_doc(&state.pool, &m).await?;
    let body = serde_json::to_vec_pretty(&doc)
        .map_err(|_| ApiError::internal("could not build mission export"))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"mission.json\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

/// Cap on the schema findings echoed to the caller. Every constraint in
/// `mission.schema.json` under `slots[]` is per-slot, so one systematic defect on a
/// large mission yields one finding per slot — without a cap the error body can
/// dwarf the document it is complaining about. The full count always ships, and the
/// full list always reaches the server log.
const MAX_REPORTED_FINDINGS: usize = 20;

/// Serialize the compiled document and hold it to `mission.schema.json` before it can
/// reach the mod.
///
/// The document is the entire website↔mod interface (TBD_MOD_DESIGN §2, "JSON is the
/// contract"), and it is **generated** — the caller is a game server that supplied
/// nothing but an id and can do nothing about a violation. So a violation is a
/// server-side defect and answers **500**, not 4xx: it is either bad stored editor
/// data or a flatten bug, and this handler cannot tell those apart. Reporting the
/// latter as a client/state error would let a real compile regression hide as "your
/// mission is misconfigured".
///
/// Returns the validated bytes so the response body is byte-identical to what was
/// checked — re-serializing a validated value would leave a gap for the two to drift.
///
/// @contract mission.schema.json#/
fn validated_compiled_body(
    mission_id: &str,
    doc: &ModMissionDocument,
) -> Result<Vec<u8>, ApiError> {
    let body =
        serde_json::to_vec(doc).map_err(|_| ApiError::internal("could not compile mission"))?;

    let findings = validate_mission_document(&body).map_err(|e| {
        tracing::error!(mission = %mission_id, error = %e, "mission schema failed to compile");
        ApiError::internal("mission validation unavailable")
    })?;
    if findings.is_empty() {
        return Ok(body);
    }

    // The schema reaches `slots[]` through both the top-level `properties` and the
    // per-schemaVersion `allOf` branch, so every slot finding arrives twice — and a
    // systematic defect produces one pair per slot, so this has to stay O(n) (a
    // `Vec::contains` scan here is quadratic on a 100k-slot mission).
    let mut seen: HashSet<String> = HashSet::with_capacity(findings.len());
    let unique: Vec<String> = findings
        .into_iter()
        .filter(|f| seen.insert(f.clone()))
        .collect();

    // The mod's own error path (TBD_MissionLoader.OnBackendFetchError) discards the
    // response body and fails over to its cached copy, so this log line — not the
    // JSON below — is what an operator actually reads.
    tracing::error!(
        mission = %mission_id,
        findings = unique.len(),
        detail = %unique.join("; "),
        "compiled mission document violates mission.schema.json",
    );

    let shown: Vec<&String> = unique.iter().take(MAX_REPORTED_FINDINGS).collect();
    Err(ApiError::with_details(
        StatusCode::INTERNAL_SERVER_ERROR,
        "compiled mission failed schema validation",
        json!({
            "schema": "mission.schema.json",
            "findingCount": unique.len(),
            "findings": shown,
        }),
    ))
}

/// `GET /api/v1/missions/:id/compiled` — the canonical mod document (service-token).
/// Runs the flatten engine live, then holds the result to `mission.schema.json` before serving
/// it — see [`validated_compiled_body`].
///
/// **Cargo capacity at compile.** Loads the same registry phys table Save uses
/// ([`load_cargo_phys_catalog`]) and compiles via [`flatten_to_mod_document_with_catalog`], so an
/// over-capacity stored version refuses here instead of shipping an empty-catalog no-op.
///
/// **The compile's structured diagnostics reach this boundary too, and are not dropped.** The body
/// cannot carry them: `mission.schema.json` closes the document root with
/// `additionalProperties: false` and [`validated_compiled_body`] holds the bytes to it, so a
/// `diagnostics` key would 500 the route for every mission. They ride ALONGSIDE the bytes instead —
/// two response headers ([`COMPILE_DIAGNOSTICS_COUNT_HEADER`] /
/// [`COMPILE_DIAGNOSTICS_RULES_HEADER`]) and one structured log line per finding.
///
/// The log line is the channel that actually matters here, and this file already says why:
/// "TBD_MissionLoader.OnBackendFetchError discards the response body ... so this log line is what an
/// operator actually reads". A finding is `warn!`, never `error!` — a diagnostic is not a refusal,
/// the document served is complete and valid, and logging it at `error` would train an operator to
/// ignore the level that means the route is broken.
///
/// @route GET /api/v1/missions/:id/compiled
/// @contract mission.schema.json#/
pub async fn get_compiled_mission(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    let Some(vid) = m.current_version_id else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    let v: Option<MissionVersion> = sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1")
        .bind(vid)
        .fetch_optional(&state.pool)
        .await?;
    let Some(v) = v else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    // Same phys table as validate_payload — empty catalog stays silent (never invent).
    let catalog = load_cargo_phys_catalog(&state.pool).await?;
    let doc =
        match flatten_to_mod_document_with_catalog(&m, v.json_payload.0.get().as_bytes(), &catalog)
        {
            Ok(doc) => doc,
            Err(CompileError::NoSlots) => return Err(ApiError::conflict("no placed slots")),
            Err(CompileError::Parse(detail)) => {
                return Err(unreadable_stored_payload(&id, &detail));
            }
        };
    // Lifted BEFORE `validated_compiled_body` consumes the document into bytes: the field is
    // `#[serde(skip)]`, so once `doc` is serialized the findings are unrecoverable (the same reason
    // `flatten_mod_document_json_full` lifts the kit substitutions where it does).
    let diagnostics = doc.diagnostics.clone();
    let body = validated_compiled_body(&id, &doc)?;

    for f in &diagnostics {
        tracing::warn!(
            mission = %id,
            rule = %f.rule_id,
            severity = %f.severity.as_str(),
            subject = %f.subject,
            subject_id = %f.subject_id.as_deref().unwrap_or(""),
            detail = %f.message,
            "compile diagnostic",
        );
    }

    let mut headers = compiled_diagnostics_response_headers(&diagnostics);
    headers.insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    Ok((headers, body).into_response())
}

/// Headers that ride alongside `/compiled` bytes for the compile's structured findings.
///
/// Count is always present — `0` included — so "the compile reported nothing" and "this build of
/// the API does not report" cannot look the same to a caller. Rules are omitted when nothing fired.
fn compiled_diagnostics_response_headers(diagnostics: &[CompileFinding]) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    if let Ok(v) = axum::http::HeaderValue::from_str(&diagnostics.len().to_string()) {
        headers.insert(HeaderName::from_static(COMPILE_DIAGNOSTICS_COUNT_HEADER), v);
    }
    if let Some(rules) = compile_diagnostics_rules_header(diagnostics) {
        // Rule ids are `&'static str` ASCII constants, so this cannot fail; the fallible form is
        // used anyway rather than an `expect` that would 500 the route over a header.
        if let Ok(v) = axum::http::HeaderValue::from_str(&rules) {
            headers.insert(HeaderName::from_static(COMPILE_DIAGNOSTICS_RULES_HEADER), v);
        }
    }
    headers
}

/// A stored payload the mission compiler cannot deserialise (`CompileError::Parse`).
///
/// **Still 500, deliberately** — the same argument [`validated_compiled_body`] makes: the caller is
/// a game server that supplied nothing but an id. Two things make this arm what it is:
///
/// 1. **The diagnosis survives.** The `serde` message names the offending key, and it reaches both
///    the log and the response body. Without it the only trace of a permanently-uncompilable
///    mission is the access log's bare `status=500`, and nobody can see WHICH key is wrong.
/// 2. **The state is essentially unreachable.** Every byte in `mission_versions.json_payload`
///    arrives through `mission_versions::validate_payload`, which runs the compiler's own
///    deserialiser, so a payload that cannot parse is a 400 at save. Reaching this branch means one
///    of exactly two things — the row was written around the API (direct SQL, a restore, a seed), or
///    the save-time precheck and this parse DISAGREE, which is a defect in the precheck. Both
///    deserve a 500. Answering 4xx would let that second case hide as "your mission is
///    misconfigured", the trap [`validated_compiled_body`] documents.
///
/// Recovery needs no hand-editing of stored JSON either way: saving a new version moves
/// `missions.current_version_id`, and the save boundary guarantees the replacement compiles.
fn unreadable_stored_payload(mission_id: &str, detail: &str) -> ApiError {
    // The mod's error path (TBD_MissionLoader.OnBackendFetchError) discards the response body and
    // fails over to its cached copy, so this log line — not the JSON below — is what an operator
    // actually reads.
    tracing::error!(
        mission = %mission_id,
        detail = %detail,
        "stored mission payload does not deserialise into the editor graph the compiler reads",
    );
    ApiError::with_details(
        StatusCode::INTERNAL_SERVER_ERROR,
        "could not compile mission",
        json!({
            "reason": "stored payload does not match the editor graph the compiler reads",
            "detail": detail,
        }),
    )
}

#[cfg(test)]
#[path = "tests/mission_export.rs"]
mod tests;
