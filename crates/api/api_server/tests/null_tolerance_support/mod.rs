//! Shared harness for the null-tolerance suites.
//!
//! **The invariant.** Every nullable column read into a non-`Option` model field must be
//! `COALESCE`d *in the query*. A non-`Option` field such as `String`/`DateTime` cannot hold
//! NULL, so the model keeps the zero value on the field and pushes the NULL→zero conversion
//! into SQL. `Option` is deliberately NOT the fix — `skip_serializing_if = "String::is_empty"`
//! already omits the key for `""` byte-identically to an omitted `None`, so `Option` would add
//! a second encoding of one state and break the committed goldens. The full rejection is
//! recorded on `api_match_telemetry::models::match_record::Match`. So: the safety lives in the
//! query, and these suites' job is to prove that no read site is missing it.
//!
//! **Why the harness is shaped this way.** A hand-written list of a few `INSERT`s and a few
//! URIs misses the bug class it exists to catch. Three structural reasons, each answered here:
//!
//!   1. **A listed endpoint set can only ever catch bugs someone remembered.**
//!      → [`route_sweep::route_sweep`] sweeps every GET route.
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
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per top-level `tests/*.rs` file; files under a
//! `tests/<dir>/` subdirectory are compiled *into* whichever suite writes
//! `mod null_tolerance_support;` and add no target of their own.

// Each suite compiles its own copy of this module and uses a different subset of it, so an
// item unused by one binary is not dead code — but rustc judges each binary on its own.
#![allow(dead_code)]

pub(crate) mod database_fixtures;
pub(crate) mod fire_mission_fixtures;
pub(crate) mod route_sweep;

/// This suite's own Discord id. Every seeded row is owned by / assigned to it and the session
/// is minted for it, so caller-scoped predicates (`WHERE assigned_to = $me`) resolve to rows
/// this file controls — and nothing here can collide with another test file's fixtures on the
/// shared integration database.
pub(crate) const NULL_UID: &str = "000000000000000099";

/// The observability token `Config::for_tests` installs, the bearer `/metrics` requires.
pub(crate) const OBSERVABILITY_TOKEN: &str = "test-observability-token";

/// How a swept route authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SweepCaller {
    /// The suite's administrator bearer session.
    Member,
    /// The operator's observability bearer (`/metrics`).
    Observability,
    /// The seeded server's `mod_runtime` machine credential.
    Machine,
}

/// The id [`database_fixtures::seed`] gives its vehicle row, so the single-row vehicle route can
/// name that row without a field on the seed.
pub(crate) const NULL_VEHICLE_ID: &str = "00000000-0000-4000-8000-000000000099";

/// The [`route_sweep::route_sweep`] entry of `GET /vehicle-database/{id}`: the seeded
/// vehicle, read by the member session.
pub(crate) fn vehicle_row_sweep() -> (&'static str, String, SweepCaller) {
    (
        "/vehicle-database/{id}",
        format!("/api/v1/vehicle-database/{NULL_VEHICLE_ID}"),
        SweepCaller::Member,
    )
}

/// The [`route_sweep::route_sweep`] entry of `GET /wiki/{slug}`: the seeded page, read by
/// the member session.
pub(crate) fn wiki_page_sweep(slug: &str) -> (&'static str, String, SweepCaller) {
    (
        "/wiki/{slug}",
        format!("/api/v1/wiki/{slug}"),
        SweepCaller::Member,
    )
}

/// The [`route_sweep::route_sweep`] entry of `GET /wiki/{slug}/revisions`: the seeded
/// page's revision history, read by the member session.
pub(crate) fn wiki_revisions_sweep(slug: &str) -> (&'static str, String, SweepCaller) {
    (
        "/wiki/{slug}/revisions",
        format!("/api/v1/wiki/{slug}/revisions"),
        SweepCaller::Member,
    )
}

/// The [`route_sweep::route_sweep`] entry of `GET /wiki/{slug}/revisions/{revision}`: the
/// seeded page's revision 1, read by the member session.
pub(crate) fn wiki_revision_sweep(slug: &str) -> (&'static str, String, SweepCaller) {
    (
        "/wiki/{slug}/revisions/{revision}",
        format!("/api/v1/wiki/{slug}/revisions/1"),
        SweepCaller::Member,
    )
}

/// Records the seeded wiki page under `slug` as its revision 1, and answers the
/// [`database_fixtures::blast_nulls`] row that names the revision by its non-null `slug`.
pub(crate) async fn seed_wiki_revision(pool: &sqlx::PgPool, slug: &str) -> (&'static str, String) {
    sqlx::query(
        "INSERT INTO wiki_page_revisions \
         (page_id, revision, slug, category, title, icon, nav_order, body_md, author_id, \
         created_at) \
         SELECT id, 1, slug, category, title, icon, nav_order, body_md, updated_by, updated_at \
         FROM wiki_pages WHERE slug = $1",
    )
    .bind(slug)
    .execute(pool)
    .await
    .expect("seed: wiki_page_revisions");
    ("wiki_page_revisions", format!("slug = '{slug}'"))
}

/// The only columns [`database_fixtures::blast_nulls`] must leave alone, because the endpoints under test *find*
/// the seeded rows through them — NULL them and the sweep silently stops reaching the code it
/// is meant to exercise, which is failure mode 2 above. Deliberately as small as possible;
/// every other nullable column in the schema gets NULLed.
pub(crate) const REACHABILITY_KEEP: &[&str] = &[
    // `dashboard.rs` / `api_operations/src/handlers/member_service_record.rs`: `WHERE orbat_slots.assigned_to = $me`.
    "orbat_slots.assigned_to",
    // `api_operations/src/handlers/member_service_record.rs` service history: `WHERE match_player_stats.discord_id = $me`.
    "match_player_stats.discord_id",
    // `api_operations/src/handlers/game_runtime_roster.rs`: a machine credential reads only the roster
    // of an event bound to its own server.
    "events.server_id",
    // `api_operations/src/services/fire_mission_store.rs`: the per-event list reads
    // `WHERE fire_missions.event_id = $1`.
    "fire_missions.event_id",
];

/// Columns NULL only in other row states: a CHECK constraint requires them for the state the
/// seed stores (an active reservation always names its event allocation), so the blast keeps
/// them rather than asserting a row the schema itself rejects.
pub(crate) const STATE_BOUND_KEEP: &[&str] = &["event_registrations.allocation_id"];

/// Routes that 5xx under the NULL blast because of a defect being fixed elsewhere — the
/// behavioural mirror of [`KNOWN_OPEN`], with the same shrinking-baseline semantics: each entry
/// must still fail, so a fix elsewhere shows up here as "delete this line" rather than as silent
/// slack. The precise cause of each is pinned by [`KNOWN_OPEN`]; this list only records that the
/// route is user-visibly broken while that defect stands.
pub(crate) const KNOWN_OPEN_ROUTES: &[(&str, &str, &str)] = &[
    // EMPTY. Every route the NULL blast reaches survives it.
];
