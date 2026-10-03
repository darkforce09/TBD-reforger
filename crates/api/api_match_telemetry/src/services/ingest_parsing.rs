//! Wire-value parsing shared by the heartbeat and the match-telemetry ingest services: the
//! foreign-key constants and the 23503 → 4xx mapping, the terrain and optional-UUID parsers, and
//! the two-state COALESCE string. The ingest bodies' own validation lives in `models`.

use uuid::Uuid;

use api_database::postgres_errors::{is_foreign_key_violation, violated_constraint};
use api_foundation::error_handling::api_error::ApiError;
use api_mission_vocabulary::TerrainType;

/// Migration `0018` constraint 10 — the one foreign key on an ingest pointer that exists today.
/// A heartbeat naming an unregistered server trips it.
pub(crate) const FK_STATUS_SERVER: &str = "server_statuses_server_id_fkey";

/// The three foreign keys written by **this domain** that migration `0018` abstained from
/// (`0018:151-169`, abstention (iv)). They do not exist yet — adding them is a migration.
///
/// **The arms below are armed for them anyway, deliberately.** The stated reason for abstaining
/// was that a violation would surface as a 500; removing that reason is the whole point of the
/// mapping. If it only covered the constraint that exists today, the migration that lifts the
/// abstention would re-open the exact defect the mapping closes — for `current_match_id` on *the
/// same INSERT statement* as `server_id`, where a 23503 carrying a different constraint name falls
/// straight through to the 500 arm. Naming them here makes that migration pure SQL.
///
/// **Not speculative — rehearsed.** All three were created by hand on a scratch database and
/// driven over HTTP; each returns its 400. The names follow `0018`'s `<table>_<column>_fkey`
/// convention, which all 25 of its constraints use; a migration that names them anything else
/// silently reverts these arms to 500, so `fk_constant_names_follow_migration_convention` in the
/// sibling tests pins the convention as the contract rather than a hope.
pub(crate) const FK_STATUS_MATCH: &str = "server_statuses_current_match_id_fkey";
pub(crate) const FK_MATCH_EVENT: &str = "matches_event_id_fkey";
pub(crate) const FK_MATCH_MISSION: &str = "matches_mission_id_fkey";

/// Map a foreign-key violation onto the 4xx that names the parent the request asked for and the
/// database could not find, or hand the error back untouched.
///
/// **Returns `Option` on purpose: the fallthrough must stay a 500.** A helper that answered
/// "4xx" for every `sqlx::Error` — or even for every 23503 — would be worse than no mapping at
/// all, because it would tell a game-server bridge that a connection reset, a NOT NULL breach or
/// a numeric overflow were its own fault and it should stop retrying. Only the constraints named
/// above are claimed; anything else returns `None` and the caller's `Err(e) => e.into()` arm
/// logs it and answers 500.
///
/// **400, not 409.** 409 is already this crate's answer for 23505 (mission version conflict,
/// duplicate event attach) and it means "the state you would create collides with state that
/// exists". This is the opposite: the state the body points at is *absent*. The caller cannot
/// resolve it by retrying unchanged, which is precisely what 400 tells it and 409 does not, and
/// `ingest_server_status` already answers 400 for `invalid server_id` / `server_id required` —
/// the same class of unusable body, reached one layer deeper. 404 was considered and rejected:
/// the route exists, and a bridge that reads 404 as "endpoint gone" is a plausible way to lose
/// telemetry for a whole deployment.
pub(crate) fn foreign_key_error(e: &sqlx::Error) -> Option<ApiError> {
    if !is_foreign_key_violation(e) {
        return None;
    }
    let msg = match violated_constraint(e)? {
        FK_STATUS_SERVER => "unknown server_id — no server is registered with that id",
        FK_STATUS_MATCH => "unknown current_match_id — no match exists with that id",
        FK_MATCH_EVENT => "unknown event_id — no event exists with that id",
        FK_MATCH_MISSION => "unknown mission_id — no mission exists with that id",
        _ => return None,
    };
    Some(ApiError::bad_request(msg))
}

/// How many unresolved `arma_id`s the unlinked-players audit row names before it summarises the
/// rest.
///
/// The **complete** list always reaches the caller in the response; this bounds only the audit
/// row's prose, which is read by a human. Today every player in a production match is
/// unresolved (see `ingest_match_results`), so an uncapped list would be a 64-id paragraph with
/// the count — the actionable number — buried in the middle of it.
pub(crate) const AUDIT_UNLINKED_ID_SAMPLE: usize = 20;

/// The terrain `s` spells exactly, or `None`.
pub(crate) fn valid_terrain(s: &str) -> Option<TerrainType> {
    s.parse().ok()
}

/// Map a wire terrain string onto the Postgres enum, or `None` when absent/blank/unknown.
///
/// **Unknown names soft-fail to `None`; they do not 400 the report.** The mission schema
/// constrains terrain to `^[a-z][a-z0-9_]*$` (`contracts/definitions/mission.schema.json`),
/// so community missions legitimately carry names outside `everon|arland|custom`. Rejecting
/// the whole match-results POST for that is the "production ingest 400s" failure mode for
/// those senders. Soft-fails like [`parse_uuid_opt`] (heartbeat three-state), **not** like
/// [`parse_uuid_opt_strict`] (match event/mission ids). The sibling tests lock
/// both halves.
pub(crate) fn parse_terrain_opt(s: &Option<String>) -> Option<TerrainType> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(valid_terrain)
}

/// Soft UUID parse for **three-state** optional fields (`current_match_id`).
///
/// `""` / `"   "` → `None` (clear when the field is present). Valid uuid → `Some`.
/// **Unparseable non-empty also → `None`** — that is intentional for `current_match_id`
/// only: the heartbeat contract is absent=keep / present-empty=clear / uuid=set, and a
/// garbage present value clears rather than 400ing the whole heartbeat. Do **not** use this
/// helper for `event_id` / `mission_id`; those must reject via [`parse_uuid_opt_strict`].
///
/// `server_id` has always been trimmed before `Uuid::parse_str` (`ingest_server_status`), and
/// these optional ids must be too — otherwise a padded uuid fails the parse and falls out as
/// `None`.
pub(crate) fn parse_uuid_opt(s: &Option<String>) -> Option<Uuid> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| Uuid::parse_str(v).ok())
}

/// Two-state COALESCE string for the keep/clear fields (`ingame_time`, `ingame_weather`,
/// `winning_faction`).
///
/// SQL `COALESCE($n, <stored>)` keys on Rust `None` → SQL NULL → keep. An explicit `""` is
/// intentional clear (`COALESCE('', col)` yields `''` because `''` is not NULL). Without
/// collapsing whitespace, `Some("   ")` is non-NULL, so COALESCE admits a third state that is
/// neither keep nor clear. Trim first; blank → `Some("")` (clear); non-blank → trimmed set.
/// `None` stays `None` (keep). Do **not** turn blank into `None` — that would break the
/// deliberate clear path.
pub(crate) fn coalesce_str(s: &Option<String>) -> Option<&str> {
    match s.as_deref().map(str::trim) {
        None => None,
        Some("") => Some(""),
        Some(v) => Some(v),
    }
}

#[cfg(test)]
#[path = "tests/ingest_parsing.rs"]
pub(crate) mod tests;
