//! One revision of a match's results report, decoded and validated as a whole.
//!
//! **Role:** turns a `POST /api/v1/ingest/match-results` body into a [`MatchResultsRevision`] or
//! a 400 that names the first offending entry, before anything is written.
//! **Position:** read by `handlers::match_results`; the validated value feeds
//! `services::match_results_ingest`.
//! **Signals & state:** none; pure decoding.
//! **Invariants:** unknown keys are refused at every level (flat counter keys on a player line
//! included); the report digest is the SHA-256 of the canonical JSON of the body without
//! `revision`; no two player lines, and no player line and removed line, share a key.
//! @contract match-telemetry.schema.json#/definitions/MatchResultsRevision

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use super::match_record::MissionOutcome;
use super::match_registration::source_match_id;
use super::telemetry_refusal::{check_object_keys, field_named_by, invalid_entry};
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::content_digest::canonical_sha256;
use http_url_guard::is_http_url;

/// Refusal code of a malformed results revision.
pub const INVALID_MATCH_RESULTS: &str = "INVALID_MATCH_RESULTS";

/// Match-level fields of a revision; absent fields keep the stored value.
/// @contract match-telemetry.schema.json#/definitions/MatchReport
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchReport {
    pub source_match_id: String,
    pub outcome: String,
    pub event_id: Option<String>,
    pub mission_id: Option<String>,
    pub terrain: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub winning_faction: Option<String>,
    pub aar_replay_url: Option<String>,
}

/// A complete scoreline; present on a line it replaces the stored counters.
/// @contract match-telemetry.schema.json#/definitions/PlayerCounters
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerCounters {
    pub kills: i64,
    pub deaths: i64,
    pub team_kills: i64,
    pub longest_kill_m: i64,
    pub vehicles_destroyed: i64,
    pub is_command: bool,
    pub command_win: Option<bool>,
}

/// One player line, keyed by `(arma_id, source_event_id)` within the match.
/// @contract match-telemetry.schema.json#/definitions/PlayerLine
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerLine {
    pub arma_id: String,
    pub role_played: String,
    pub source_event_id: String,
    pub counters: Option<PlayerCounters>,
}

/// A stored player line the revision deletes.
/// @contract match-telemetry.schema.json#/definitions/RemovedLine
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovedLine {
    pub arma_id: String,
    pub source_event_id: String,
}

/// A validated revision: trimmed keys, parsed outcome and attachment ids, and its digest.
#[derive(Debug, Clone)]
pub struct MatchResultsRevision {
    pub revision: i64,
    pub source_match_id: String,
    pub outcome: MissionOutcome,
    pub event_id: Option<Uuid>,
    pub mission_id: Option<Uuid>,
    pub report: MatchReport,
    pub players: Vec<PlayerLine>,
    pub removed_lines: Vec<RemovedLine>,
    pub report_sha256: String,
}

/// The answer to a results revision; `linked + unlinked == players` counts the submitted lines.
/// @contract match-telemetry.schema.json#/definitions/MatchResultsAnswer
#[derive(Debug, serde::Serialize)]
pub struct MatchResultsAnswer {
    pub match_id: Uuid,
    pub revision: i64,
    pub applied: bool,
    pub players: usize,
    pub linked: usize,
    pub unlinked: usize,
    pub unlinked_arma_ids: Vec<String>,
}

fn refusal(message: impl Into<String>, index: Option<usize>, field: Option<&str>) -> ApiError {
    invalid_entry(INVALID_MATCH_RESULTS, message, index, field)
}

fn decode_entry<T: for<'de> Deserialize<'de>>(
    value: &Value,
    what: &str,
    index: Option<usize>,
) -> Result<T, ApiError> {
    serde_json::from_value(value.clone()).map_err(|error| {
        let field = field_named_by(&error);
        refusal(format!("invalid {what}: {error}"), index, field.as_deref())
    })
}

fn parse_outcome(raw: &str) -> Result<MissionOutcome, ApiError> {
    match raw.trim() {
        "success" => Ok(MissionOutcome::Success),
        "failure" => Ok(MissionOutcome::Failure),
        "aborted" => Ok(MissionOutcome::Aborted),
        "pending" => Ok(MissionOutcome::Pending),
        _ => Err(refusal("invalid outcome", None, Some("outcome"))),
    }
}

fn parse_uuid(raw: &Option<String>, field: &str) -> Result<Option<Uuid>, ApiError> {
    match raw.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(text) => Uuid::parse_str(text)
            .map(Some)
            .map_err(|_| refusal(format!("{field} must be a UUID"), None, Some(field))),
    }
}

fn line_key(arma_id: &str, source_event_id: &str) -> (String, String) {
    (arma_id.trim().to_owned(), source_event_id.trim().to_owned())
}

fn check_line_key(arma_id: &str, source_event_id: &str, index: usize) -> Result<(), ApiError> {
    let arma_id = arma_id.trim();
    if arma_id.is_empty() || arma_id.len() > 128 {
        return Err(refusal(
            "arma_id must contain 1 to 128 bytes",
            Some(index),
            Some("arma_id"),
        ));
    }
    if source_event_id.trim().is_empty() {
        return Err(refusal(
            "source_event_id is required",
            Some(index),
            Some("source_event_id"),
        ));
    }
    Ok(())
}

fn check_counters(counters: &PlayerCounters, index: usize) -> Result<(), ApiError> {
    for (field, value) in [
        ("kills", counters.kills),
        ("deaths", counters.deaths),
        ("team_kills", counters.team_kills),
        ("longest_kill_m", counters.longest_kill_m),
        ("vehicles_destroyed", counters.vehicles_destroyed),
    ] {
        if value < 0 {
            return Err(refusal(
                format!("counters.{field} must not be negative"),
                Some(index),
                Some(field),
            ));
        }
    }
    Ok(())
}

/// Decode and validate a whole revision body, refusing it on the first invalid entry.
pub fn decode_results_revision(body: &Value) -> Result<MatchResultsRevision, ApiError> {
    check_object_keys(
        body,
        &["revision", "match", "players", "removed_lines"],
        &["revision", "match", "players"],
    )
    .map_err(|(field, message)| refusal(message, None, Some(&field)))?;
    let revision = body["revision"]
        .as_i64()
        .filter(|revision| *revision >= 1)
        .ok_or_else(|| {
            refusal(
                "revision must be an integer of at least 1",
                None,
                Some("revision"),
            )
        })?;
    let report: MatchReport = decode_entry(&body["match"], "match", None)?;
    let source_match_id = source_match_id(&report.source_match_id)
        .map_err(|message| refusal(message, None, Some("source_match_id")))?;
    let outcome = parse_outcome(&report.outcome)?;
    let event_id = parse_uuid(&report.event_id, "event_id")?;
    let mission_id = parse_uuid(&report.mission_id, "mission_id")?;
    if let Some(url) = report.aar_replay_url.as_deref().map(str::trim)
        && !url.is_empty()
        && !is_http_url(url)
    {
        return Err(refusal(
            "aar_replay_url must be an absolute http:// or https:// URL",
            None,
            Some("aar_replay_url"),
        ));
    }

    let Some(player_values) = body["players"].as_array() else {
        return Err(refusal("players must be an array", None, Some("players")));
    };
    let mut players = Vec::with_capacity(player_values.len());
    let mut keys = std::collections::HashSet::new();
    for (index, value) in player_values.iter().enumerate() {
        let line: PlayerLine = decode_entry(value, "player line", Some(index))?;
        check_line_key(&line.arma_id, &line.source_event_id, index)?;
        if line.role_played.trim().is_empty() {
            return Err(refusal(
                "role_played must not be blank",
                Some(index),
                Some("role_played"),
            ));
        }
        if let Some(counters) = &line.counters {
            check_counters(counters, index)?;
        }
        if !keys.insert(line_key(&line.arma_id, &line.source_event_id)) {
            return Err(refusal(
                "two player lines share arma_id and source_event_id",
                Some(index),
                Some("arma_id"),
            ));
        }
        players.push(line);
    }

    let removed_values = match body.get("removed_lines") {
        None => Vec::new(),
        Some(Value::Array(values)) => values.clone(),
        Some(_) => {
            return Err(refusal(
                "removed_lines must be an array",
                None,
                Some("removed_lines"),
            ));
        }
    };
    let mut removed_lines = Vec::with_capacity(removed_values.len());
    for (index, value) in removed_values.iter().enumerate() {
        let line: RemovedLine = decode_entry(value, "removed line", Some(index))?;
        check_line_key(&line.arma_id, &line.source_event_id, index)?;
        if keys.contains(&line_key(&line.arma_id, &line.source_event_id)) {
            return Err(refusal(
                "a removed line is also a player line of this revision",
                Some(index),
                Some("arma_id"),
            ));
        }
        removed_lines.push(line);
    }

    let mut digest_input = body.clone();
    if let Some(object) = digest_input.as_object_mut() {
        object.remove("revision");
    }
    Ok(MatchResultsRevision {
        revision,
        source_match_id,
        outcome,
        event_id,
        mission_id,
        report,
        players,
        removed_lines,
        report_sha256: canonical_sha256(&digest_input),
    })
}

#[cfg(test)]
#[path = "tests/match_results_revision.rs"]
mod tests;
