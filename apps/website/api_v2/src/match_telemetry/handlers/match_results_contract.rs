//! The wire contract of `POST /api/v1/ingest/match-results`: the match half, the per-player
//! line with its optional counters block, and the envelope that carries both.

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// The match half of a results POST.
///
/// **`outcome` is deliberately required — do not add `#[serde(default)]` to it, and do not
/// derive `Default` for this struct.** Unlike a heartbeat, a *result* has no honest reason to
/// omit how the match ended; that is the one thing the endpoint exists to report. A default
/// decodes as `""`, `""` maps to `MissionOutcome::Pending`, and the re-ingest path binds it
/// unconditionally — so re-POSTing a known `source_match_id` with a partial body would walk a
/// finished, won match backwards to `pending`. A `Default` derive is the second half of the same
/// hole: with `MatchResultsInput::match_data` defaulted, a body of `{}` would skip the field
/// requirement entirely and mint an anonymous `pending` match row on every call. Neither exists;
/// `{}` is a decode error → 400.
///
/// Requiring `outcome` costs a sender nothing — a match that genuinely has not finished can
/// still say `"pending"` out loud. What it removes is the *silent* pending.
///
/// **Every other field is `Option`, and all of them read the same way on the re-ingest path:
/// absent keeps what is stored, present wins.** That is one rule, not seven, and `upsert_match`
/// implements it with one `COALESCE` per column. Unlike `outcome` they each have a legitimate
/// absence, and the destructive part was only ever the overwrite:
///
/// * `winning_faction` — a `failure`/`aborted`/`pending` match has no winner, so demanding one
///   would be a lie. Absent keeps whatever is stored; an explicit `""` clears it, which is the
///   re-adjudication path. Whitespace-only is the same clear (`coalesce_str`) — `Some("   ")`
///   must not land as a third COALESCE state.
/// * `aar_replay_url` — the replay is uploaded *after* the match, so the POST that carries the
///   result usually cannot know the link yet and a later pass attaches it. Defaulting this to
///   `""` means the next result POST tears the link back off.
/// * `ended_at` — it sits in the same `UPDATE` and is nulled by the same partial body, so it
///   gets the same treatment.
/// * `event_id`, `mission_id`, `terrain`, `started_at` — these must appear in the `UPDATE` too.
///   Left out of it, a correction to any of them is discarded instead of applied, and `event_id`
///   in particular decides whether attendance is marked at all. They are optional for the same
///   reason as the three above (a match need not belong to a scheduled op), but "may be omitted"
///   must not be implemented as "may never be changed". Read the `UPDATE` in `upsert_match` for
///   the measured consequence.
///
/// Note what this rule does **not** claim: none of these is three-stated. A blank `event_id` /
/// `mission_id` / `terrain` reads as absent (keeps), not as a clear. Only `ServerStatusInput`'s
/// `current_match_id` is three-stated, because a live server genuinely has to stop pointing at a
/// finished match; a stored match has no equivalent "un-assign the event" story, and inventing
/// one here would be a contract nobody has asked for.
///
/// **Unparseable non-empty `event_id` / `mission_id` is a 400**, not a silent `None`. Blank still
/// keeps; only genuine garbage rejects. `current_match_id` keeps the soft helper.
#[derive(Debug, Deserialize)]
pub struct MatchInput {
    /// Absent = no source id, create a fresh match; a value = the dedupe key. Blank is neither,
    /// so it is a 400 — read `source_match_key`, which is the only thing allowed to interpret it.
    pub(super) source_match_id: Option<String>,
    pub(super) event_id: Option<String>,
    pub(super) mission_id: Option<String>,
    pub(super) terrain: Option<String>,
    pub(super) started_at: Option<DateTime<Utc>>,
    pub(super) ended_at: Option<DateTime<Utc>>,
    pub(super) outcome: String,
    pub(super) winning_faction: Option<String>,
    pub(super) aar_replay_url: Option<String>,
}

/// One player's final line for one match: a required identity/role **core**, plus an
/// optional, all-or-nothing **counters** block.
///
/// **Absent `counters` is not a write. Present `counters` is authoritative and replaces all
/// of them.** That single sentence is the whole contract: omission is not a write, and a wire
/// shape the one shipping client can actually satisfy stays satisfiable.
///
/// # Why the counters are replaced wholesale
///
/// This row is keyed `(match_id, arma_id, source_event_id)` and holds *final per-match
/// totals*, and the upsert replaces them wholesale, so a re-ingest that defaulted them would
/// write `kills=0 deaths=0 … is_command=false command_win=NULL` over a real scoreline — which
/// `leaderboard_totals` then sums, in the same request, via `refresh_leaderboard`. Three
/// fixes were on the table and only one of them is honest:
///
/// * **`GREATEST(existing, incoming)`** — rejected. It reads as "counters only go up", but
///   half this row is not a counter: `is_command`, `command_win` and `role_played` are
///   corrupted by the same write and `GREATEST` means nothing for them, so the rule would
///   have to be applied field-by-field and would stop being a rule. Worse, it makes the row
///   a permanent high-water mark: a downward correction after an anti-cheat review could
///   never be applied through the API. And it *absorbs* a broken sender instead of
///   reporting it — on a service-token endpoint with nobody watching, that is the one thing
///   we cannot afford.
/// * **Reject the re-ingest as a duplicate** — rejected. Retry safety is the contract here;
///   the endpoint is documented and tested as idempotent, and a game server that retries a
///   dropped response must not get a 409.
/// * **Full replace, with the counters required** — taken. The POST is authoritative for
///   this player-in-this-match, a restatement is exactly what a retry sends, and corrections
///   still work in both directions.
///
/// **Do not put `#[serde(default)]` back on any counter.** It is the exact mechanism of the
/// silent zeroing above: the block is `Option`, and every field *inside* the block is required.
///
/// # Why the block is optional but its contents are not
///
/// "An incomplete body is a bug in the sender" inverts here, because it would change a wire
/// contract the only client cannot meet. `TBD_ResultsReporter.c` `BuildPlayerRow` hand-builds
/// four keys (`arma_id`, `role_played`, `deaths`, `source_event_id`) by string concatenation, so
/// there is no serializer to quietly fill the rest, and it omits them *on purpose* — the mod
/// reports only what it can measure. Serde rejects on the first missing field, so requiring every
/// counter at the top level 400s **every match report from every production server**: match rows,
/// per-player stats, attendance, user-stat recompute and leaderboard refresh all dead on arrival.
///
/// So the fields split by *who is entitled to state them*:
///
/// * **Core (required)** — `arma_id`, `role_played`, `source_event_id`. Identity and role.
///   Any reporter that knows a player was in a match knows all three; two of them are the
///   dedupe key. `role_played` stays required and always-replaced because it is corrupted by
///   the same write, and a reporter that can name the player can name the slot they held.
/// * **Counters (optional block, all-or-nothing)** — a *measurement*, made by one reporter,
///   about one player, in one match. A **partial** block is still a 400: the fields inside
///   it carry no `default`, so a missing key is a decode error.
///
/// This is not a weakening of the replace rule, it is the same rule stated one level up. A
/// present-but-incomplete body would silently zero; a body either states the scoreline in full or
/// does not state it at all, and "does not state it" writes nothing. Silence is not a value.
///
/// # Why `deaths` is inside the block and not in the core
///
/// This is the one genuinely arguable line, since the mod does send `deaths` today and putting it
/// in the block means that value is dropped unless the rest arrives. It goes in the block anyway,
/// for two reasons:
///
/// * **`kd_ratio` couples it to `kills`.** `leaderboard_totals` is
///   `round(sum(kills) / sum(deaths), 2)` (`0001_initial_schema.sql:274-277`). A contract that
///   lets `deaths` be written *without* `kills` is a contract that lets a low-fidelity
///   re-ingest corrupt a derived aggregate — 17 kills over 1 death instead of 3. `deaths` is
///   not separable from the block it is divided into.
/// * **A scoreline is one measurement by one reporter.** Splitting any counter out lets two
///   reporters interleave into a single row — a full report writes `17/3/…`, then the mod's
///   one-life report rewrites `deaths` to `1` and leaves `kills` at `17`. Half the row from
///   each source is precisely the corruption an all-or-nothing block prevents, and it is only
///   all-or-nothing if it is complete.
///
/// **Fold, do not drop.** A body that omits `counters` but still names any of
/// kills/deaths/team_kills/longest_kill_m/vehicles_destroyed/is_command/command_win at the
/// row's top level is a complete scoreline: present values are taken, unsent numeric fields
/// are 0, unsent `is_command` is false, unsent `command_win` is NULL. When `counters` is
/// present it is authoritative and the flat keys are ignored (nested wins; no double count).
/// Identity-only rows — no nested block and no flat keys — still write no counters.
///
/// `command_win` stays `Option<bool>` because it is a genuine tri-state: `NULL` means "not a
/// command slot / not adjudicated", which is a different statement from `false`. It is the
/// one field inside the block that may be omitted, and omitting it means `NULL`, not "keep".
#[derive(Debug, Deserialize)]
pub struct PlayerStatInput {
    pub(super) arma_id: String,
    pub(super) role_played: String,
    pub(super) source_event_id: String,
    /// Absent (or `null`) = this POST makes no claim about the scoreline, so the upsert does
    /// not name the counter columns at all. Present = authoritative for every one of them.
    pub(super) counters: Option<PlayerCountersInput>,

    // Flat top-level keys. Folded into a complete scoreline only when `counters` is absent;
    // nested wins when both shapes are present. Unsent fold fields are 0 / false / NULL.
    #[serde(default)]
    kills: Option<i64>,
    #[serde(default)]
    deaths: Option<i64>,
    #[serde(default)]
    team_kills: Option<i64>,
    #[serde(default)]
    longest_kill_m: Option<i64>,
    #[serde(default)]
    vehicles_destroyed: Option<i64>,
    #[serde(default)]
    is_command: Option<bool>,
    #[serde(default)]
    command_win: Option<bool>,
}

impl PlayerStatInput {
    /// Nested `counters` wins. When that block is absent, any top-level counter key is folded
    /// into a complete scoreline. Identity-only rows (neither shape) still write nothing.
    pub(super) fn effective_counters(&self) -> Option<PlayerCountersInput> {
        self.counters.clone().or_else(|| self.fold_flat_counters())
    }

    fn fold_flat_counters(&self) -> Option<PlayerCountersInput> {
        if self.kills.is_none()
            && self.deaths.is_none()
            && self.team_kills.is_none()
            && self.longest_kill_m.is_none()
            && self.vehicles_destroyed.is_none()
            && self.is_command.is_none()
            && self.command_win.is_none()
        {
            return None;
        }
        Some(PlayerCountersInput {
            // Perturbation target: skip this kills fold — use `0` — and the flat golden goes red.
            kills: self.kills.unwrap_or(0),
            deaths: self.deaths.unwrap_or(0),
            team_kills: self.team_kills.unwrap_or(0),
            longest_kill_m: self.longest_kill_m.unwrap_or(0),
            vehicles_destroyed: self.vehicles_destroyed.unwrap_or(0),
            is_command: self.is_command.unwrap_or(false),
            command_win: self.command_win,
        })
    }
}

/// The measured half of a player's line — all of it, or none of it.
///
/// Every field here is **required on purpose**; this is where the "no `#[serde(default)]`" rule
/// lives. The block as a whole is optional ([`PlayerStatInput::counters`]); the fields inside it
/// are not. A body that sends `{"kills": 17}` and stops is a sender that has half a scoreline and
/// does not know it, and it gets a 400 rather than five zeros.
#[derive(Clone, Debug, Deserialize)]
pub struct PlayerCountersInput {
    pub(super) kills: i64,
    pub(super) deaths: i64,
    pub(super) team_kills: i64,
    pub(super) longest_kill_m: i64,
    pub(super) vehicles_destroyed: i64,
    pub(super) is_command: bool,
    pub(super) command_win: Option<bool>,
}

/// Both keys are required: the handler's own error message claims "match and players are
/// required", and `#[serde(default)]` on either one would make that a lie. `players`
/// may still be `[]` — an explicit empty roster is a statement; a missing key is not.
#[derive(Debug, Deserialize)]
pub struct MatchResultsInput {
    #[serde(rename = "match")]
    pub(super) match_data: MatchInput,
    pub(super) players: Vec<PlayerStatInput>,
}

#[cfg(test)]
#[path = "tests/match_results_contract.rs"]
mod tests;
