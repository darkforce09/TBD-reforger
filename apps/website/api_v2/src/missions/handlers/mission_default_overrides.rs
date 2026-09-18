//! Corpus-wide instrumentation: which mission defaults every author changes.
//!
//! The single most-used API across a large shipped mission corpus is typically the one that turns
//! a feature OFF, and the lesson drawn from that is blunt: if 43% of missions disable your default,
//! the default is wrong — make such toggles visible mission settings, not magic globals. Learning
//! that elsewhere took an outside analyst machine-parsing 171 PBOs. TBD owns its corpus in a
//! table, so "what fraction of missions override X" is a QUERY, and this is it: one READ-ONLY
//! aggregation over `mission_versions.json_payload`, reporting per authored default-bearing key
//! the fraction of missions whose LATEST version differs from the schema default, plus the value
//! histogram.

use axum::extract::State;
use axum::response::Json;
use chrono::Utc;
use serde_json::{Value, json};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::missions::models::mission::{MissionDefaultOverride, MissionDefaultValueBucket};

/// The canonical mission schema, read READ-ONLY at runtime to enumerate its `default`-bearing keys.
///
/// The SAME bytes `contract/schema_validators.rs` embeds for `validate_mission_document` and the
/// zone pass (that module's `MISSION_SCHEMA` is private, so this is a second `include_str!` of the
/// one canonical file, not a copy of the data). Reading it is the whole point: the schema OWNS the
/// defaults, so the key list is derived from it here and never hardcoded — a hardcoded list rots
/// the first time a `default` is added, removed or retuned in the schema.
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
/// The set is the **`$defs/zoneRules` properties that declare a `default`** — and only those — for
/// two reasons this function encodes rather than assumes:
///
///  * `$defs/flow` (briefingSeconds, safeStartSeconds, timeLimitSeconds, jip) and `$defs/settings`
///    (respawn, spectatorPolicy, nightVision) declare **no `default`** in the schema. Their
///    fallbacks live in `flatten.rs` (`FLOW_DEFAULT_*`), not the schema, so by the rule this
///    endpoint follows — enumerate FROM THE SCHEMA, keys that carry a schema default — they are
///    out. Walking the schema is what makes that a fact the code reads, not a claim it hardcodes.
///  * `$defs/radioNet.range` DOES carry `default: "short"`, but `radioPlan` is **derived at compile
///    from the ORBAT and authored nowhere** (`flatten.rs` `derive_radio_plan`: "there is no
///    `radioPlan` anywhere in the editor payload"). A key no author can store cannot be overridden
///    in a stored payload, so it has no honest aggregation; it is excluded BY CONSTRUCTION, since
///    it lives under `$defs/radioNet`, not the `$defs/zoneRules` this walk reads.
///
/// So the enumeration is: every `$defs/zoneRules` property with a `default`, authored in the stored
/// editor payload at `zones[].rules.<name>` (the `EditorPayload.zones` root array, each element's
/// `rules` object — `flatten.rs`). Add a defaulted zone rule to the schema and it appears here with
/// no code change; that is the anti-rot this design buys.
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
/// missions whose LATEST version overrides it, plus the value histogram.
///
/// ── The population and the "latest version" join ────────────────────────────────────────────────
/// "Latest version" is `missions.current_version_id` — the SAME tip the library, the overview and
/// the in-game browser read (`game_server_injection::ingest_list_missions`: `LEFT JOIN
/// mission_versions v ON v.id = m.current_version_id`). This handler does NOT invent a second
/// definition of latest; a mission with no current version, or a soft-deleted mission, is not in
/// the population.
///
/// The denominator (`missions_total`) is missions whose latest version authors **at least one
/// zone**, because a mission that draws no play area / objective cannot override a zone rule — it
/// would only dilute every fraction toward zero and hide the signal. The numerator counts a mission
/// ONCE if any of its zones authors a value for the key that differs from the schema default
/// (`rules ? key AND (rules -> key) IS DISTINCT FROM default`).
///
/// ── The SQL shape ───────────────────────────────────────────────────────────────────────────────
/// One aggregation per key beside the `jsonb_typeof(... -> 'editor' -> 'slots')` /
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
