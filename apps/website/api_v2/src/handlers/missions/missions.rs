//! Mission export, compiled-document and instrumentation handlers.
//!
//! Four surfaces share one file because they all read a mission and serve a DERIVED document
//! rather than editing one: the strict export envelope download, the compiled mod document the
//! `/compiled` route flattens live and holds to `mission.schema.json`, the service-token mission
//! listing the in-game admin browser reads, and the corpus-wide default-override aggregation.
//!
//! The mission library, lifecycle, version and armory handlers live under
//! [`crate::missions::handlers`].

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::http::{HeaderName, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, MissionMakerUser, ServiceAuth};
use crate::missions::contract::schema_validators::validate_mission_document;
use crate::missions::handlers::mission_versions::load_cargo_phys_catalog;
use crate::missions::models::mission::{
    MissionDefaultOverride, MissionDefaultValueBucket, MissionVersion,
};
use crate::missions::services::mission_document::build_mission_doc;
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::can_view;
use crate::services::{
    COMPILE_DIAGNOSTICS_COUNT_HEADER, COMPILE_DIAGNOSTICS_RULES_HEADER, CompileError,
    CompileFinding, ModMissionDocument, compile_diagnostics_rules_header,
    flatten_to_mod_document_with_catalog, mission_terrain_key,
};

// --- export + compiled ---

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

/// One row of `GET /api/v1/ingest/missions`.
///
/// The field names are **camelCase on purpose** and are NOT the usual snake_case API
/// contract: they are read by `TBD_MissionListEntry`
/// (`apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionListLoader.c:3-9`), and
/// Enfusion's `JsonLoadContext` maps JSON keys onto class fields **by name**. A key the
/// struct does not declare is not an error there — it is simply invisible, so a
/// snake_case `slot_count` would parse to `0` for every mission with no warning
/// anywhere. Renamed explicitly rather than via a container attribute so the coupling is
/// visible on the field that has it (T-181.51).
#[derive(Debug, Serialize)]
pub struct IngestMissionListEntry {
    /// The mission UUID — the id the mod persists via `TBD_BackendConfig.SetMissionId`
    /// and then fetches at `GET /api/v1/missions/{id}/compiled`. NOT the compiled
    /// document's `meta.id`, which is a different (schema-shaped) id space.
    id: String,
    name: String,
    /// The compiled document's `meta.terrain`, from the one shared derivation — see
    /// [`website_map_engine::data::scenario::flatten::mission_terrain_key`].
    terrain: String,
    #[serde(rename = "slotCount")]
    slot_count: i64,
}

/// `GET /api/v1/ingest/missions` — every runnable mission, for the in-game admin browser
/// (service-token tier).
///
/// ── WHY THIS IS NOT `list_missions` ────────────────────────────────────────────────────
/// [`list_missions`] is **owner-scoped**: it takes an [`AuthUser`] and every branch of
/// `push_filters` binds `me = &user.discord_id` (mine / bookmarked / live-or-my-drafts). A
/// service token is a game server, not a person — it has no "me", so that handler cannot
/// simply be re-tiered. This one applies no owner filter at all, which is the correct
/// answer for a machine that has to be able to run any mission an admin names.
///
/// `slotCount` is the count of PLACED editor slots in the mission's current version, read
/// in SQL (`jsonb_array_length`) rather than by compiling each mission — a compile per row
/// would parse every payload in the library on one request. It is the same array the
/// flatten walks, so `slotCount == 0` predicts exactly the `409 no placed slots` the mod
/// would get from `/compiled`, which is what `TBD_FrameworkManager.SelectMissionByNumber`
/// warns on.
///
/// @route GET /api/v1/ingest/missions
pub async fn ingest_list_missions(
    State(state): State<AppState>,
    _svc: ServiceAuth,
) -> Result<Json<Value>, ApiError> {
    // LEFT JOIN: a mission with no saved version still belongs in the list (the admin can
    // see it exists and that it has 0 slots) — an INNER JOIN would make it vanish silently.
    // The `jsonb_typeof` guard is not decoration: `jsonb_array_length` RAISES on a
    // non-array, which would turn one malformed payload into a 500 for the whole list.
    let rows: Vec<(Uuid, String, String, String, i32)> = sqlx::query_as(
        "SELECT m.id, m.title, m.terrain::text, COALESCE(m.custom_terrain_name, ''), \
                CASE WHEN jsonb_typeof(v.json_payload -> 'editor' -> 'slots') = 'array' \
                     THEN jsonb_array_length(v.json_payload -> 'editor' -> 'slots') \
                     ELSE 0 END \
         FROM missions m \
         LEFT JOIN mission_versions v ON v.id = m.current_version_id \
         WHERE m.deleted_at IS NULL \
         ORDER BY m.title ASC, m.id ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let missions: Vec<IngestMissionListEntry> = rows
        .into_iter()
        .map(
            |(id, title, terrain, custom, slots)| IngestMissionListEntry {
                id: id.to_string(),
                name: title,
                terrain: mission_terrain_key(&terrain, &custom),
                slot_count: i64::from(slots),
            },
        )
        .collect();

    let count = missions.len();
    Ok(Json(json!({ "missions": missions, "count": count })))
}

// ── T-683: default-override instrumentation ─────────────────────────────────────────────────────
//
// WHICH MISSION DEFAULTS EVERY AUTHOR CHANGES. WOG's single most-used API across 171 shipped
// missions is the one that turns a feature OFF (`wog3_no_auto_long_range_radio`, 74/171), and
// `wog.md:1078` concludes: "if 43% of missions disable your default, the default is wrong — make
// such toggles visible mission settings, not magic globals." WOG needed an outside analyst to
// machine-parse 171 PBOs to learn that. TBD owns its corpus in a table, so "what fraction of
// missions override X" is a QUERY, and this is it: one READ-ONLY aggregation over
// `mission_versions.json_payload`, reporting per authored default-bearing key the fraction of
// missions whose LATEST version differs from the schema default, plus the value histogram.

/// The canonical mission schema, read READ-ONLY at runtime to enumerate its `default`-bearing keys.
///
/// The SAME bytes `contract/validate.rs` embeds for `validate_mission_document` / the T-581 zone
/// pass (that module's `MISSION_SCHEMA` is private, so this is a second `include_str!` of the one
/// canonical file, not a copy of the data). Reading it is the whole point of the ticket: the schema
/// OWNS the defaults, so the key list is derived from it here and never hardcoded — a hardcoded
/// list rots the first time a `default` is added, removed or retuned in the schema.
const MISSION_SCHEMA_SRC: &str =
    include_str!("../../../../../../packages/tbd-schema/schema/mission.schema.json");

/// One schema-declared default: where an author writes it in a stored payload, and its value.
struct SchemaDefaultKey {
    /// The `zoneRules` property name (e.g. `graceSeconds`), read straight from the schema.
    rules_key: String,
    /// The wire `key` reported to the client — the AUTHORED path `zones[].rules.<name>`.
    wire_key: String,
    /// The `default` value the schema declares for this key, verbatim.
    default_value: Value,
}

/// Enumerate the mission schema's `default`-bearing authored keys FROM `mission.schema.json`.
///
/// ── Which keys, and why exactly these ──────────────────────────────────────────────────────────
/// The ticket names "the flow/settings keys + zoneRules keys that carry a schema default". Read
/// against the live schema, that set resolves to the **`$defs/zoneRules` properties that declare a
/// `default`** — and only those — for two reasons this function encodes rather than assumes:
///
///  * `$defs/flow` (briefingSeconds, safeStartSeconds, timeLimitSeconds, jip) and `$defs/settings`
///    (respawn, spectatorPolicy, nightVision) declare **no `default`** in the schema. Their fallbacks
///    live in `flatten.rs` (`FLOW_DEFAULT_*`), not the schema, so by the ticket's own rule
///    ("enumerate FROM THE SCHEMA … keys that carry a schema default") they are out. Walking the
///    schema is what makes that a fact the code reads, not a claim it hardcodes.
///  * `$defs/radioNet.range` DOES carry `default: "short"` (the direct analog of WOG's long-range
///    radio toggle), but `radioPlan` is **derived at compile from the ORBAT and authored nowhere**
///    (`flatten.rs` `derive_radio_plan`: "there is no `radioPlan` anywhere in the editor payload").
///    A key no author can store cannot be overridden in a stored payload, so it has no honest
///    aggregation; it is excluded BY CONSTRUCTION, since it lives under `$defs/radioNet`, not the
///    `$defs/zoneRules` this walk reads.
///
/// So the enumeration is: every `$defs/zoneRules` property with a `default`, authored in the stored
/// editor payload at `zones[].rules.<name>` (the `EditorPayload.zones` root array, each element's
/// `rules` object — `flatten.rs`). Add a defaulted zone rule to the schema and it appears here with
/// no code change; that is the anti-rot the ticket is buying.
fn schema_default_keys() -> Result<Vec<SchemaDefaultKey>, ApiError> {
    let schema: Value = serde_json::from_str(MISSION_SCHEMA_SRC)
        .map_err(|e| ApiError::internal(format!("mission.schema.json is not valid JSON: {e}")))?;
    let props = schema
        .pointer("/$defs/zoneRules/properties")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ApiError::internal("mission.schema.json has no $defs/zoneRules/properties")
        })?;

    let mut keys: Vec<SchemaDefaultKey> = props
        .iter()
        .filter_map(|(name, spec)| {
            spec.get("default").map(|default_value| SchemaDefaultKey {
                rules_key: name.clone(),
                wire_key: format!("zones[].rules.{name}"),
                default_value: default_value.clone(),
            })
        })
        .collect();
    // Sort for a STABLE wire: this crate's `serde_json` is built without `preserve_order`, so
    // `Map` iteration order is unspecified. Sorting by key name makes `data[]` deterministic across
    // requests and processes (the client keys by `key`, so alphabetical is as good as any and does
    // not pretend to reproduce the schema's file order, which we cannot see here).
    keys.sort_by(|a, b| a.rules_key.cmp(&b.rules_key));
    Ok(keys)
}

/// `GET /api/v1/admin/mission-default-overrides` — per authored default key, the fraction of
/// missions whose LATEST version overrides it, plus the value histogram (T-683).
///
/// ── The population and the "latest version" join ────────────────────────────────────────────────
/// "Latest version" is `missions.current_version_id` — the SAME tip the library, the overview and
/// the in-game browser read (`ingest_list_missions` above: `LEFT JOIN mission_versions v ON
/// v.id = m.current_version_id`). This handler does NOT invent a second definition of latest; a
/// mission with no current version, or a soft-deleted mission, is not in the population.
///
/// The denominator (`missions_total`) is missions whose latest version authors **at least one
/// zone**, because a mission that draws no play area / objective cannot override a zone rule — it
/// would only dilute every fraction toward zero and hide the signal the ticket is after. The
/// numerator counts a mission ONCE if any of its zones authors a value for the key that differs
/// from the schema default (`rules ? key AND (rules -> key) IS DISTINCT FROM default`).
///
/// ── The SQL shape ───────────────────────────────────────────────────────────────────────────────
/// One aggregation per key beside the proven `jsonb_typeof(... -> 'editor' -> 'slots')` /
/// `jsonb_array_length` query `ingest_list_missions` already runs over this exact column — a second
/// query, not new machinery. Each unnests the latest version's `zones` array with
/// `jsonb_array_elements` and reads `zones[].rules.<key>`; the key name and default are BOUND as
/// parameters (never string-interpolated), and they originate from the schema, so there is no
/// injection surface. `IS DISTINCT FROM` gives correct JSON equality across number / string / bool
/// without per-type branches, and treats a missing key as "not an override".
///
/// READ-ONLY on both sides: no write, and the defaults are read from `mission.schema.json`, never
/// widened or edited (widening the schema flips this executor — hard stop).
///
/// @route GET /api/v1/admin/mission-default-overrides
pub async fn mission_default_overrides(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Value>, ApiError> {
    let keys = schema_default_keys()?;
    let mut data: Vec<MissionDefaultOverride> = Vec::with_capacity(keys.len());

    for key in &keys {
        // Denominator: distinct latest-version missions that author at least one zone. Counted once
        // here so every key shares the same population regardless of which zones set which rule.
        let missions_total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM missions m \
             JOIN mission_versions v ON v.id = m.current_version_id \
             WHERE m.deleted_at IS NULL \
               AND jsonb_typeof(v.json_payload -> 'zones') = 'array' \
               AND jsonb_array_length(v.json_payload -> 'zones') > 0",
        )
        .fetch_one(&state.pool)
        .await?;

        // Numerator: distinct missions where ANY authored zone sets this key to a non-default value.
        // The `->> 0` guard is not needed — `IS DISTINCT FROM` compares the jsonb values directly.
        let missions_overriding: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT m.id) FROM missions m \
             JOIN mission_versions v ON v.id = m.current_version_id \
             CROSS JOIN LATERAL jsonb_array_elements(v.json_payload -> 'zones') AS z(zone) \
             WHERE m.deleted_at IS NULL \
               AND jsonb_typeof(v.json_payload -> 'zones') = 'array' \
               AND jsonb_typeof(z.zone -> 'rules') = 'object' \
               AND (z.zone -> 'rules') ? $1 \
               AND (z.zone -> 'rules' -> $1) IS DISTINCT FROM $2::jsonb",
        )
        .bind(&key.rules_key)
        .bind(sqlx::types::Json(&key.default_value))
        .fetch_one(&state.pool)
        .await?;

        // Histogram: each DISTINCT authored value for the key, with the count of distinct missions
        // that authored it in any zone. A mission authoring the same value in two zones counts once
        // per value (count(DISTINCT m.id)); authoring two different values contributes to both bars.
        let buckets: Vec<(sqlx::types::Json<Value>, i64)> = sqlx::query_as(
            "SELECT (z.zone -> 'rules' -> $1) AS value, count(DISTINCT m.id) AS n \
             FROM missions m \
             JOIN mission_versions v ON v.id = m.current_version_id \
             CROSS JOIN LATERAL jsonb_array_elements(v.json_payload -> 'zones') AS z(zone) \
             WHERE m.deleted_at IS NULL \
               AND jsonb_typeof(v.json_payload -> 'zones') = 'array' \
               AND jsonb_typeof(z.zone -> 'rules') = 'object' \
               AND (z.zone -> 'rules') ? $1 \
             GROUP BY (z.zone -> 'rules' -> $1) \
             ORDER BY n DESC, (z.zone -> 'rules' -> $1)::text ASC",
        )
        .bind(&key.rules_key)
        .fetch_all(&state.pool)
        .await?;

        let histogram = buckets
            .into_iter()
            .map(|(value, count)| MissionDefaultValueBucket {
                value: value.0,
                count,
            })
            .collect();

        let override_fraction = if missions_total > 0 {
            missions_overriding as f64 / missions_total as f64
        } else {
            0.0
        };

        data.push(MissionDefaultOverride {
            key: key.wire_key.clone(),
            default_value: key.default_value.clone(),
            missions_total,
            missions_overriding,
            override_fraction,
            histogram,
        });
    }

    Ok(Json(json!({ "data": data, "generated_at": Utc::now() })))
}

/// `GET /api/v1/missions/:id/compiled` — the canonical mod document (service-token).
/// Runs the Phase 8 flatten engine live (gate G6 end-to-end), then holds the result to
/// `mission.schema.json` before serving it — see [`validated_compiled_body`].
///
/// **T-549 — cargo capacity at compile.** Loads the same registry phys table Save uses
/// ([`load_cargo_phys_catalog`]) and compiles via [`flatten_to_mod_document_with_catalog`], so
/// pre-T-416 over-capacity versions refuse here instead of shipping an empty-catalog no-op.
///
/// **T-690 — the compile's structured diagnostics reach this boundary too, and are not dropped.**
/// The body cannot carry them: `mission.schema.json` closes the document root with
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
    // T-690 — lifted BEFORE `validated_compiled_body` consumes the document into bytes: the field is
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

/// Headers that ride alongside `/compiled` bytes for the compile's structured findings (T-690).
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
/// a game server that supplied nothing but an id. What T-367 changed are the two things that were
/// actually wrong here:
///
/// 1. **The diagnosis was destroyed.** The arm read `Err(CompileError::Parse(_))` and dropped the
///    detail on the floor, so the only trace of a permanently-uncompilable mission was the access
///    log's bare `status=500`. The `serde` message names the offending key; it now reaches both the
///    log and the response body. Without it, "unrecoverable without hand-editing the stored JSON"
///    was literally true — nobody could see WHICH key was wrong.
/// 2. **The state was reachable.** Every byte in `mission_versions.json_payload` arrives through
///    [`validate_payload`], which since T-367 runs the compiler's own deserialiser, so a payload
///    that cannot parse is a 400 at save. This branch is no longer reachable through the API at all,
///    and reaching it now means one of exactly two things — the row was written around the API
///    (direct SQL, a restore, a seed), or the save-time precheck and this parse DISAGREE, which is a
///    defect in the precheck. Both deserve a 500. Answering 4xx would let that second case hide as
///    "your mission is misconfigured", the trap [`validated_compiled_body`] documents.
///
/// Recovery needs no hand-editing of stored JSON either way: saving a new version moves
/// `missions.current_version_id`, and the save boundary now guarantees the replacement compiles.
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
mod tests {
    use super::*;

    /// T-549 Class-R: live `/compiled` must load the Save phys catalog and compile through the
    /// catalogued gate — empty-catalog `flatten_to_mod_document` lets pre-T-416 over-capacity
    /// versions ship. RED: swap back to the no-arg flatten, or drop the catalog load.
    #[test]
    fn compiled_route_loads_cargo_phys_catalog() {
        const SRC: &str = include_str!("missions.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn get_compiled_mission(")
            .expect("get_compiled_mission must exist");
        let after = &production[start..];
        // Next pub async fn after this one (unreadable_stored_payload is private fn).
        let body = after
            .split("\nfn unreadable_stored_payload(")
            .next()
            .expect("get_compiled_mission must precede unreadable_stored_payload");
        assert!(
            body.contains("load_cargo_phys_catalog"),
            "/compiled must load registry phys into the catalog; got:\n{body}"
        );
        assert!(
            body.contains("flatten_to_mod_document_with_catalog("),
            "/compiled must call the catalogued compile gate; got:\n{body}"
        );
        // Isolate so a with_catalog import alone cannot false-green a no-arg call.
        let stripped = body.replace("flatten_to_mod_document_with_catalog", "");
        assert!(
            !stripped.contains("flatten_to_mod_document("),
            "/compiled must not call the empty-catalog no-arg flatten; got:\n{body}"
        );
    }

    /// T-690 Class-R: `/compiled` must SURFACE the compile's structured findings, not drop them.
    ///
    /// This is the defect the ticket is named after, at the one boundary where it is invisible:
    /// everything the compile learned was thrown away or flattened into a pass/fail, and here the
    /// caller is a game server that reads no body on failure — so a dropped finding leaves literally
    /// no trace anywhere. The pin reads the live handler body and asserts four things:
    ///
    /// 1. the findings are lifted BEFORE `validated_compiled_body` consumes the document (the field
    ///    is `#[serde(skip)]`; after serialization they are unrecoverable);
    /// 2. both headers are emitted (the "alongside the bytes" channel);
    /// 3. each finding reaches the log, which this file already documents as the only channel an
    ///    operator actually reads on this route; and
    /// 4. a finding is NOT converted into a refusal — no `ApiError` is constructed from one.
    ///
    /// RED: delete the `let diagnostics = doc.diagnostics.clone();` line, or the `tracing::warn!`.
    #[test]
    fn t690_compiled_route_surfaces_the_structured_diagnostics() {
        const SRC: &str = include_str!("missions.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn get_compiled_mission(")
            .expect("get_compiled_mission must exist");
        // Window must EXCLUDE the helper def — otherwise a route that no longer calls the helper
        // still greens on the definition sitting between get_compiled_mission and
        // unreadable_stored_payload (wave-136 F2).
        let body = production[start..]
            .split("\nfn compiled_diagnostics_response_headers(")
            .next()
            .expect("get_compiled_mission must precede compiled_diagnostics_response_headers");

        let lift = body
            .find("doc.diagnostics")
            .expect("/compiled must read the compile's findings off the document");
        let consume = body
            .find("validated_compiled_body(")
            .expect("/compiled must still hold the body to mission.schema.json");
        assert!(
            lift < consume,
            "the findings must be lifted BEFORE the document is serialized — `#[serde(skip)]` \
             means they cannot be recovered afterwards; got:\n{body}"
        );
        assert!(
            body.contains("compiled_diagnostics_response_headers(&diagnostics)"),
            "/compiled route body must call compiled_diagnostics_response_headers(&diagnostics); got:\n{body}"
        );
        let helper = production
            .split("fn compiled_diagnostics_response_headers(")
            .nth(1)
            .expect("compiled_diagnostics_response_headers must exist");
        assert!(
            helper.contains("COMPILE_DIAGNOSTICS_COUNT_HEADER")
                && helper.contains("COMPILE_DIAGNOSTICS_RULES_HEADER"),
            "/compiled must carry the findings alongside the bytes in both headers; got:\n{helper}"
        );
        assert!(
            body.contains("tracing::warn!"),
            "/compiled must log each finding — the mod discards the response body, so the log is \
             what an operator reads; got:\n{body}"
        );
        // A diagnostic is not a refusal: the findings loop must not mint an error.
        let loop_body = body
            .split("for f in &diagnostics {")
            .nth(1)
            .and_then(|t| t.split("\n    }").next())
            .expect("the per-finding loop must exist");
        assert!(
            !loop_body.contains("ApiError") && !loop_body.contains("return"),
            "a finding must not refuse the compile — the document is complete and valid; got:\n{loop_body}"
        );
    }

    /// T-763 Class-R: a clean compile's *response* carries `x-compile-diagnostics-count: 0`.
    ///
    /// T-690 pins the route's source shape (lift-before-serialize, both header constants, warn!,
    /// no refusal). That is necessary but not sufficient: a source scan cannot prove the assembled
    /// `Response` actually exposes the count header when the findings list is empty. This builds
    /// the response the same way the route does — via [`compiled_diagnostics_response_headers`] —
    /// and asserts the wire value.
    ///
    /// RED: skip the count insert when `diagnostics.is_empty()`.
    #[test]
    fn t763_compiled_clean_mission_response_carries_diagnostics_count_zero() {
        let headers = compiled_diagnostics_response_headers(&[]);
        let response = (headers, axum::body::Body::empty()).into_response();
        let count = response
            .headers()
            .get(COMPILE_DIAGNOSTICS_COUNT_HEADER)
            .and_then(|v| v.to_str().ok());
        assert_eq!(
            count,
            Some("0"),
            "clean /compiled must advertise x-compile-diagnostics-count: 0, not omit the header"
        );
        assert!(
            response
                .headers()
                .get(COMPILE_DIAGNOSTICS_RULES_HEADER)
                .is_none(),
            "rules header is omitted when nothing fired"
        );

        // wave-136 F2 — also pin the route, not only the helper. A bypass that inlines a HeaderMap
        // omitting the count on empty findings must RED here (and in t690's narrowed window).
        const SRC: &str = include_str!("missions.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn get_compiled_mission(")
            .expect("get_compiled_mission must exist");
        let route = production[start..]
            .split("\nfn compiled_diagnostics_response_headers(")
            .next()
            .expect("get_compiled_mission must precede compiled_diagnostics_response_headers");
        assert!(
            route.contains("compiled_diagnostics_response_headers(&diagnostics)"),
            "clean /compiled count pin requires the route to call the helper; got:\n{route}"
        );
    }
}
