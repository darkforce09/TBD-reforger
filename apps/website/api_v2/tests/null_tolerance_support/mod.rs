//! Shared harness for the null-tolerance suites.
//!
//! **The invariant.** Every nullable column read into a non-`Option` model field must be
//! `COALESCE`d *in the query*. A non-`Option` field such as `String`/`DateTime` cannot hold
//! NULL, so the model keeps the zero value on the field and pushes the NULL→zero conversion
//! into SQL. `Option` is deliberately NOT the fix — `skip_serializing_if = "String::is_empty"`
//! already omits the key for `""` byte-identically to an omitted `None`, so `Option` would add
//! a second encoding of one state and break the committed goldens. The full rejection is
//! recorded on `match_telemetry::models::match_record::Match`. So: the safety lives in the
//! query, and these suites' job is to prove that no read site is missing it.
//!
//! **Why the harness is shaped this way.** A hand-written list of a few `INSERT`s and a few
//! URIs misses the bug class it exists to catch. Three structural reasons, each answered here:
//!
//!   1. **A listed endpoint set can only ever catch bugs someone remembered.**
//!      → [`database_fixtures::route_sweep`] sweeps every GET route, and
//!      `every_get_route_is_swept_or_skipped_with_a_reason` parses the route tables and fails
//!      when a route is added without being covered.
//!   2. **Rows have to be reachable.** Authenticating as the shared dev-login user while
//!      seeding rows against a different synthetic id leaves every `WHERE assigned_to = $me` /
//!      `WHERE discord_id = $me` branch — precisely where these defects live — dead from the
//!      suite's point of view, and the assertions pass **vacuously**. → these suites mint their
//!      own session for [`NULL_UID`] and seed every row owned by / assigned to that same id.
//!   3. **Tables and columns have to be enumerated, not listed.** Omitting a column from an
//!      `INSERT` only yields NULL for a column with no `DEFAULT`, so that coverage is
//!      contingent on a schema property nothing checks and becomes a silent no-op the day a
//!      `DEFAULT` is added. → [`database_fixtures::blast_nulls`] enumerates nullable columns
//!      from `information_schema` and `UPDATE`s them to NULL explicitly, then asserts the
//!      NULLs actually landed.
//!
//! **The enumerating half.** A behavioural sweep can still only reach code whose predicates the
//! seed happens to satisfy, so `tests/null_tolerance_select_scan.rs` closes the class
//! statically: it cross-references every `SELECT` literal in `src/` against
//! `information_schema` nullability and fails on a bare `*` or an un-`COALESCE`d nullable
//! column, with [`OPTION_FIELDS`] as the only escape hatch.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per top-level `tests/*.rs` file; files under a
//! `tests/<dir>/` subdirectory are compiled *into* whichever suite writes
//! `mod null_tolerance_support;` and add no target of their own.

// Each suite compiles its own copy of this module and uses a different subset of it, so an
// item unused by one binary is not dead code — but rustc judges each binary on its own.
#![allow(dead_code)]

pub mod database_fixtures;
pub mod source_scan;

/// This suite's own Discord id. Every seeded row is owned by / assigned to it and the session
/// is minted for it, so caller-scoped predicates (`WHERE assigned_to = $me`) resolve to rows
/// this file controls — and nothing here can collide with another test file's fixtures on the
/// shared integration database.
pub const NULL_UID: &str = "000000000000000099";

/// The service token `Config::for_tests` installs, for the `X-Service-Token` routes.
pub const SERVICE_TOKEN: &str = "test-service-token";

/// The only columns [`database_fixtures::blast_nulls`] must leave alone, because the endpoints under test *find*
/// the seeded rows through them — NULL them and the sweep silently stops reaching the code it
/// is meant to exercise, which is failure mode 2 above. Deliberately as small as possible;
/// every other nullable column in the schema gets NULLed.
pub const REACHABILITY_KEEP: &[&str] = &[
    // `dashboard.rs` / `operations/handlers/member_service_record.rs`: `WHERE orbat_slots.assigned_to = $me`.
    "orbat_slots.assigned_to",
    // `operations/handlers/member_service_record.rs` service history: `WHERE match_player_stats.discord_id = $me`.
    "match_player_stats.discord_id",
];

/// `(table, column)` pairs that are nullable in the schema *and* `Option<..>` on the model, so
/// a read site is allowed to select them without `COALESCE`. This is the allowlist for
/// `no_query_as_reads_a_nullable_column_without_coalesce`; every entry names the model field
/// that makes it sound. Adding an entry is a claim that the field is `Option` — check it.
///
/// A nullable column that is NOT here and NOT `COALESCE`d is the bug these suites exist for.
pub const OPTION_FIELDS: &[(&str, &str)] = &[
    // identity_and_access::models::user_account::User
    ("users", "arma_id"),
    ("users", "banned_by"),
    ("users", "banned_at"),
    ("users", "last_login_at"),
    // identity_and_access::models::user_account::RefreshToken
    ("refresh_tokens", "revoked_at"),
    // community_content::models::announcement::Announcement
    ("announcements", "published_at"),
    // missions::models::mission::Mission
    ("missions", "current_version_id"),
    ("missions", "reviewed_by"),
    ("missions", "reviewed_at"),
    // missions::models::mission::MissionArmory — `null` = unlimited, a real third state.
    ("mission_armories", "quantity"),
    // operations::models::event::Event — the link to a match is matches.event_id, so the
    // event row carries no match id of its own.
    ("events", "server_id"),  // Option<Uuid> — migration 0011
    ("events", "modpack_id"), // Option<Uuid> — migration 0011
    // operations::models::event::OrbatSlot
    ("orbat_slots", "assigned_to"),
    ("orbat_slots", "assigned_at"),
    // operations::models::event::EventRegistration
    ("event_registrations", "slot_id"),
    // operations::models::leave_request::LeaveRequest
    ("leave_requests", "reviewed_by"),
    // match_telemetry::models::match_record::Match
    ("matches", "source_match_id"),
    ("matches", "event_id"),
    ("matches", "mission_id"),
    ("matches", "terrain"),
    ("matches", "ended_at"),
    // match_telemetry::models::match_record::MatchPlayerStat
    ("match_player_stats", "discord_id"),
    ("match_player_stats", "command_win"),
    // Counters are Option: NULL = not measured, which is distinct from a scored 0.
    ("match_player_stats", "kills"),
    ("match_player_stats", "deaths"),
    ("match_player_stats", "team_kills"),
    ("match_player_stats", "longest_kill_m"),
    ("match_player_stats", "vehicles_destroyed"),
    ("match_player_stats", "is_command"),
    // server_infrastructure::models::server::{Server, ServerStatus}
    ("server_statuses", "current_match_id"),
    ("servers", "required_modpack_id"),
    // administration::models::audit_log::AuditLog
    ("audit_logs", "actor_id"),
    ("audit_logs", "metadata"),
    // operations::models::fire_mission::FireMission
    ("fire_missions", "event_id"),
    // community_content::models::wiki::WikiPage
    ("wiki_pages", "updated_by"),
    // missions::models::registry::RegistryItem — every one of these is `Option`, and NULL means
    // "engine class default", which is a distinct state from any zero value.
    ("registry_items", "abstract"),
    ("registry_items", "arsenal_type"),
    ("registry_items", "weight_kg"),
    ("registry_items", "volume_cm3"),
    ("registry_items", "max_weight_kg"),
    ("registry_items", "max_volume_cm3"),
    ("registry_items", "addon"),
    ("registry_items", "variant_of"),
    ("registry_items", "cargo_grid_w"),
    ("registry_items", "cargo_grid_h"),
];

/// Instances of this exact defect that are being fixed elsewhere, keyed
/// `(source file, produced column or `*`, owner)`.
///
/// **Tolerance, not assertion.** An entry suppresses a finding when it matches and is simply
/// inert when it does not. It deliberately does *not* assert the defect is still present: a
/// worktree branched from an older `main` answers "is this sibling bug fixed yet?" differently
/// from merged `main`, and a presence assertion would turn one of the two trees
/// red no matter which way it was written. Instead:
///   * anything **not** listed is a hard failure (that is the enumeration doing its job), and
///   * [`BASELINE_CAP`] stops the list growing silently, which is the only way a tolerance list
///     can quietly become the false confidence this suite exists to prevent.
///
/// Stale entries are printed to stderr on every run — prune them when you see them.
///
/// Keyed by file, not `file:line`, so an unrelated edit above the defect does not break it.
/// Do not add entries for files you own. Fix those.
pub const KNOWN_OPEN: &[(&str, &str, &str)] = &[
    // EMPTY, and that is the point. Every entry this list has ever held was fixed rather than
    // tolerated. BASELINE_CAP is 0, so the next entry cannot be added without raising it in a
    // diff a reviewer sees.
];

/// Ceiling on [`KNOWN_OPEN`] + [`KNOWN_OPEN_ROUTES`]. The teeth behind a tolerance list: entries
/// can go inert harmlessly, but nobody can add one without raising this number deliberately, in a
/// diff a reviewer sees.
///
/// **Zero**: no instance of this defect is open anywhere in the crate.
///
/// Leaving the cap above 0 would re-open silent slack in the one suite whose whole purpose is to
/// stop this bug class hiding — the failure mode where a suite passes vacuously and nobody
/// notices.
pub const BASELINE_CAP: usize = 0;

/// Routes that 5xx under the NULL blast because of a defect being fixed elsewhere — the
/// behavioural mirror of [`KNOWN_OPEN`], with the same shrinking-baseline semantics: each entry
/// must still fail, so a fix elsewhere shows up here as "delete this line" rather than as silent
/// slack. The precise cause of each is pinned by [`KNOWN_OPEN`]; this list only records that the
/// route is user-visibly broken while that defect stands.
pub const KNOWN_OPEN_ROUTES: &[(&str, &str, &str)] = &[
    // EMPTY. Every route the NULL blast reaches survives it.
];

/// GET routes deliberately outside [`route_sweep`], each with the reason it cannot be swept.
/// Everything else the api_v2 route tables register must appear in the sweep — see
/// [`every_get_route_is_swept_or_skipped_with_a_reason`].
pub const ROUTE_SWEEP_SKIP: &[(&str, &str)] = &[
    (
        "/auth/dev-login",
        "mints a session and 302s; reads no model",
    ),
    ("/auth/discord/login", "302 to Discord; reads no model"),
    (
        "/auth/discord/callback",
        "needs a live Discord code exchange",
    ),
    (
        "/admin/audit-logs/stream",
        "SSE — never completes under oneshot",
    ),
    (
        "/servers/{id}/status/stream",
        "SSE — never completes under oneshot",
    ),
];
