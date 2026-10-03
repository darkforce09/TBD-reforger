//! The load procedure's committed database reads and the rows they return.
//!
//! **Role:** the `SELECT` constants the load procedure runs through
//! `remote_observers/database_reader.rs` (the staging mission, the synthetic population, the
//! fixture events with their slots, the `strict` rate-limit buckets, the fleet's heartbeat
//! census) and the parsers of their `psql -A -t` rows.
//!
//! **Position:** used by the plan's probes, `seed-load`, the fixture identities, the census
//! during the member load, and the local rehearsal.
//!
//! **Signals & state:** none; constants and pure parsers.
//!
//! **Invariants:** every statement is a single `SELECT` without `;` whose only value is a
//! `:'name'` literal; the synthetic range pattern is the reserved range's; a row that does not
//! parse is dropped, so a judge sees fewer rows, never invented ones.

use serde::Serialize;

use crate::remote_observers::database_reader::{CommittedQuery, rows};

/// The live, undeleted missions titled `:'title'`.
pub(crate) const STAGING_MISSION: CommittedQuery = CommittedQuery {
    name: "load_staging_mission",
    sql: "SELECT id FROM missions WHERE title = :'title' AND status = 'live' \
          AND deleted_at IS NULL ORDER BY id",
    parameters: &["title"],
};

/// The synthetic accounts: count, count of unbanned `enlisted` ones, lowest and highest id.
pub(crate) const SYNTHETIC_POPULATION: CommittedQuery = CommittedQuery {
    name: "load_synthetic_population",
    sql: "SELECT count(*), count(*) FILTER (WHERE role = 'enlisted' AND NOT is_banned), \
          coalesce(min(discord_id), ''), coalesce(max(discord_id), '') FROM users \
          WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
    parameters: &[],
};

/// Each `[Load fixture]` event of a reserved author with its attachment, its mission and its
/// slots in ORBAT order (faction, squad, index).
pub(crate) const LOAD_FIXTURE_EVENTS: CommittedQuery = CommittedQuery {
    name: "load_fixture_events",
    sql: "SELECT e.name_override, e.id, em.id, em.mission_id, m.title, count(s.id), \
          coalesce(string_agg(s.id::text, ',' ORDER BY s.faction, s.squad, s.slot_index, s.id), '') \
          FROM events e JOIN event_missions em ON em.event_id = e.id AND em.deleted_at IS NULL \
          JOIN missions m ON m.id = em.mission_id \
          LEFT JOIN orbat_slots s ON s.event_mission_id = em.id \
          WHERE e.name_override LIKE '[Load fixture]%' AND e.deleted_at IS NULL \
          AND e.created_by ~ '^91000000000000[0-9]{5}$' \
          GROUP BY e.name_override, e.id, em.id, em.mission_id, m.title \
          ORDER BY e.name_override, e.id, em.id",
    parameters: &[],
};

/// Every `strict|<address>` rate-limit bucket with its last update in Unix milliseconds.
pub(crate) const STRICT_RATE_LIMIT_BUCKETS: CommittedQuery = CommittedQuery {
    name: "load_strict_rate_limit_buckets",
    sql: "SELECT bucket_key, (extract(epoch FROM updated_at) * 1000)::bigint \
          FROM rate_limit_buckets WHERE bucket_key LIKE 'strict|%' ORDER BY bucket_key",
    parameters: &[],
};

/// The open runtime session of each active fleet server, its last heartbeat and the database's
/// clock, in Unix milliseconds.
pub(crate) const FLEET_HEARTBEAT_CENSUS: CommittedQuery = CommittedQuery {
    name: "load_fleet_heartbeat_census",
    sql: "SELECT s.name, s.id, r.generation, \
          coalesce((extract(epoch FROM r.last_heartbeat_at) * 1000)::bigint, 0), \
          (extract(epoch FROM now()) * 1000)::bigint \
          FROM servers s JOIN server_runtime_sessions r ON r.server_id = s.id \
          AND r.ended_at IS NULL WHERE s.is_active AND s.name LIKE 'TBD Staging %' \
          ORDER BY s.name",
    parameters: &[],
};

/// The synthetic population as the database holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PopulationCensus {
    pub(crate) accounts: u64,
    pub(crate) enlisted_accounts: u64,
    pub(crate) lowest_id: String,
    pub(crate) highest_id: String,
}

/// One seeded fixture event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SeededFixtureEvent {
    pub(crate) title: String,
    pub(crate) event_id: String,
    pub(crate) event_mission_id: String,
    pub(crate) mission_id: String,
    pub(crate) mission_title: String,
    pub(crate) slot_ids: Vec<String>,
}

/// One fleet server's open session in a heartbeat census.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct HeartbeatRow {
    pub(crate) server_name: String,
    pub(crate) server_id: String,
    pub(crate) generation: i64,
    pub(crate) last_heartbeat_unix_ms: u64,
    pub(crate) database_now_unix_ms: u64,
}

impl HeartbeatRow {
    /// Milliseconds since the last heartbeat by the database's clock; `u64::MAX` for none.
    pub(crate) fn heartbeat_age_ms(&self) -> u64 {
        if self.last_heartbeat_unix_ms == 0 {
            return u64::MAX;
        }
        self.database_now_unix_ms
            .saturating_sub(self.last_heartbeat_unix_ms)
    }
}

/// The single row of [`SYNTHETIC_POPULATION`].
pub(crate) fn population_census(output: &str) -> Option<PopulationCensus> {
    let row = rows(output).into_iter().next()?;
    match row.as_slice() {
        [accounts, enlisted, lowest, highest] => Some(PopulationCensus {
            accounts: accounts.trim().parse().ok()?,
            enlisted_accounts: enlisted.trim().parse().ok()?,
            lowest_id: lowest.trim().to_string(),
            highest_id: highest.trim().to_string(),
        }),
        _ => None,
    }
}

/// The rows of [`LOAD_FIXTURE_EVENTS`], in title order.
pub(crate) fn fixture_events(output: &str) -> Vec<SeededFixtureEvent> {
    rows(output)
        .into_iter()
        .filter_map(|row| match row.as_slice() {
            [
                title,
                event,
                attachment,
                mission,
                mission_title,
                count,
                slots,
            ] => {
                let slot_ids: Vec<String> = slots
                    .split(',')
                    .map(str::trim)
                    .filter(|slot| !slot.is_empty())
                    .map(str::to_string)
                    .collect();
                (count.trim().parse::<usize>().ok()? == slot_ids.len()).then(|| {
                    SeededFixtureEvent {
                        title: title.clone(),
                        event_id: event.trim().to_string(),
                        event_mission_id: attachment.trim().to_string(),
                        mission_id: mission.trim().to_string(),
                        mission_title: mission_title.clone(),
                        slot_ids,
                    }
                })
            }
            _ => None,
        })
        .collect()
}

/// The rows of [`STRICT_RATE_LIMIT_BUCKETS`]: `(bucket key, updated at)`. The key itself holds a
/// `|` (`strict|<address>`), so each row splits at its last `|` only.
pub(crate) fn strict_buckets(output: &str) -> Vec<(String, u64)> {
    output
        .lines()
        .filter_map(|line| {
            let (key, updated) = line.trim().rsplit_once('|')?;
            Some((key.to_string(), updated.trim().parse().ok()?))
        })
        .collect()
}

/// The rows of [`FLEET_HEARTBEAT_CENSUS`].
pub(crate) fn heartbeat_census(output: &str) -> Vec<HeartbeatRow> {
    rows(output)
        .into_iter()
        .filter_map(|row| match row.as_slice() {
            [name, id, generation, heartbeat, now] => Some(HeartbeatRow {
                server_name: name.clone(),
                server_id: id.trim().to_string(),
                generation: generation.trim().parse().ok()?,
                last_heartbeat_unix_ms: heartbeat.trim().parse().ok()?,
                database_now_unix_ms: now.trim().parse().ok()?,
            }),
            _ => None,
        })
        .collect()
}

/// The ids of [`STAGING_MISSION`]'s rows.
pub(crate) fn mission_ids(output: &str) -> Vec<String> {
    rows(output)
        .into_iter()
        .filter_map(|row| row.first().map(|id| id.trim().to_string()))
        .filter(|id| !id.is_empty())
        .collect()
}
