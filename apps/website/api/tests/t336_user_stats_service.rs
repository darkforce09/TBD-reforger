//! T-336 — `recompute_user_stats` in `services/`, and proof the move changed nothing.
//!
//! # What this file has to prove, and why it is shaped like this
//!
//! T-336 is a **pure relocation**: the function moved from `pub(super) fn` in
//! `handlers/telemetry.rs` to `pub fn` in `services/user_stats.rs`, with the three statements
//! byte-identical. The ticket asks for two things, and they are different things.
//!
//! 1. **Reachability from where it should be.** This file `use`s
//!    `website_api::services::recompute_user_stats` from *outside the crate*. That import does not
//!    compile against the pre-T-336 tree at all — `pub(super)` in `handlers::telemetry` is not
//!    reachable from an integration test — so the existence of this binary is the proof.
//! 2. **Behaviour unchanged.** The numbers below are arithmetic written out in the comments, not
//!    whatever the query returned: four matches (one of them a second stat line for the *same*
//!    match, so `count(DISTINCT match_id)` and `count(*)` disagree), and three past registrations
//!    of which two are `attended`. Those two fixtures exist specifically so a plausible rewrite of
//!    the moved SQL goes red rather than passing on a degenerate case.
//!
//! The existing `tests/telemetry.rs` and `tests/identity_link.rs` suites are the other half: they
//! exercise the same function through `POST /ingest/match-results` and the identity-link confirm,
//! and they were green before this move and are green after it without an edit.

use sqlx::PgPool;
use uuid::Uuid;
use website_api::db;
// The T-336 reachability proof: `handlers::telemetry::recompute_user_stats` was `pub(super)`, so
// this line is the thing that could not be written before the move.
use website_api::services::{recompute_user_stats, recompute_user_stats_best_effort};

mod common;

/// Per-test fixture identity. Tests inside one binary run in parallel, so each owns a **distinct**
/// `discord_id` and a distinct row tag — a shared player would have them deleting each other's
/// matches and reading each other's counts, which is a harness bug wearing the costume of a
/// behaviour change. Ids are in the T-400 private range and are never content-golden Vance
/// (`…003`).
struct Fixture {
    player: &'static str,
    tag: &'static str,
}

impl Fixture {
    async fn boot(player: &'static str, tag: &'static str) -> Option<(PgPool, Self)> {
        let url = common::require_test_database_url()?;
        let pool = db::connect(&url).await.expect("connect");
        db::migrate(&pool).await.expect("migrate");
        let f = Self { player, tag };
        common::seed_user(
            &pool,
            player,
            "T336 Fixture",
            &format!("t336-arma-{tag}"),
            "enlisted",
        )
        .await;
        f.reset(&pool).await;
        Some((pool, f))
    }

    /// Drop everything this fixture seeds. `matches` has no cascade to `match_player_stats`, so
    /// both are cleaned explicitly — an orphan stat row would silently change
    /// `count(DISTINCT match_id)`, which is the exact number under test.
    async fn reset(&self, pool: &PgPool) {
        sqlx::query("DELETE FROM match_player_stats WHERE source_event_id = $1")
            .bind(self.tag)
            .execute(pool)
            .await
            .expect("clean stats");
        sqlx::query("DELETE FROM matches WHERE source_match_id LIKE $1")
            .bind(format!("m-{}-%", self.tag))
            .execute(pool)
            .await
            .expect("clean matches");
        sqlx::query(
            "DELETE FROM event_registrations WHERE event_mission_id IN \
             (SELECT em.id FROM event_missions em JOIN events e ON e.id = em.event_id \
               WHERE e.created_by = $1)",
        )
        .bind(self.tag)
        .execute(pool)
        .await
        .expect("clean registrations");
        sqlx::query(
            "DELETE FROM event_missions WHERE event_id IN \
             (SELECT id FROM events WHERE created_by = $1)",
        )
        .bind(self.tag)
        .execute(pool)
        .await
        .expect("clean event_missions");
        sqlx::query("DELETE FROM events WHERE created_by = $1")
            .bind(self.tag)
            .execute(pool)
            .await
            .expect("clean events");
        sqlx::query("DELETE FROM missions WHERE author_id = $1")
            .bind(self.tag)
            .execute(pool)
            .await
            .expect("clean missions");
    }

    /// Insert a `matches` row and return its id.
    async fn seed_match(&self, pool: &PgPool, name: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO matches (source_match_id, started_at, outcome, created_at) \
             VALUES ($1, now(), 'success', now()) RETURNING id",
        )
        .bind(format!("m-{}-{name}", self.tag))
        .fetch_one(pool)
        .await
        .expect("seed match")
    }

    /// One stat line for this fixture's player in `match_id`. `arma_id` varies so two lines can
    /// share a match without colliding on the natural key.
    async fn seed_stat(&self, pool: &PgPool, match_id: Uuid, arma_suffix: &str) {
        sqlx::query(
            "INSERT INTO match_player_stats \
             (match_id, discord_id, arma_id, role_played, kills, deaths, team_kills, \
              longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id, \
              created_at) \
             VALUES ($1, $2, $3, 'SL', 1, 1, 0, 0, 0, false, NULL, $4, now())",
        )
        .bind(match_id)
        .bind(self.player)
        .bind(format!("t336-arma-{}-{arma_suffix}", self.tag))
        .bind(self.tag)
        .execute(pool)
        .await
        .expect("seed stat line");
    }

    /// One `event_missions` row whose `start_time` is `hours_ago` in the past (negative = future),
    /// plus a registration for this fixture's player in the given state.
    async fn seed_registration(&self, pool: &PgPool, name: &str, hours_ago: i32, state: &str) {
        let mission_id: Uuid = sqlx::query_scalar(
            "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, \
             created_at, updated_at) \
             VALUES ($1, $2, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
        )
        .bind(format!("T336 {name}"))
        .bind(self.tag)
        .fetch_one(pool)
        .await
        .expect("seed mission");
        let event_id: Uuid = sqlx::query_scalar(
            "INSERT INTO events (start_time, created_by, created_at, updated_at) \
             VALUES (now() - make_interval(hours => $1), $2, now(), now()) RETURNING id",
        )
        .bind(hours_ago)
        .bind(self.tag)
        .fetch_one(pool)
        .await
        .expect("seed event");
        let em_id: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
             VALUES ($1, $2, now() - make_interval(hours => $3), now(), now()) RETURNING id",
        )
        .bind(event_id)
        .bind(mission_id)
        .bind(hours_ago)
        .fetch_one(pool)
        .await
        .expect("seed event_mission");
        sqlx::query(
            "INSERT INTO event_registrations (event_mission_id, discord_id, state, registered_at) \
             VALUES ($1, $2, $3::registration_state, now())",
        )
        .bind(em_id)
        .bind(self.player)
        .bind(state)
        .execute(pool)
        .await
        .expect("seed registration");
    }

    async fn stored_stats(&self, pool: &PgPool) -> (i64, f64) {
        sqlx::query_as::<_, (i64, f64)>(
            "SELECT total_deployments, attendance_rate::float8 FROM users WHERE discord_id = $1",
        )
        .bind(self.player)
        .fetch_one(pool)
        .await
        .expect("read user stats")
    }
}

/// The relocated function, called from outside the crate, writing the numbers the arithmetic says.
///
/// Fixture (all figures are written out, none are copied from a run):
/// * **three distinct matches**, one of which carries **two** stat lines for this player — so
///   `count(DISTINCT match_id)` = 3 while `count(*)` = 4. A rewrite that dropped `DISTINCT` reads
///   4 and fails here.
/// * **three past registrations**, of which **two** are `attended` → `2 / 3 * 100 = 66.66…`.
/// * **one future registration**, `registered`. `past_registered` filters on
///   `start_time <= now()`, so it must not join the denominator; a rewrite that dropped that
///   filter reads `2 / 4 * 100 = 50` and fails here.
///
/// # The asymmetry this deliberately pins rather than fixes
///
/// The numerator (`state = 'attended'`) is **not** time-filtered while the denominator is. Phase 2
/// below marks a *future* op attended and requires the rate to move to 100 — which is what the
/// shipped SQL does, and which means `attendance_rate` can in principle exceed 100.
///
/// That is a real latent defect and it is **not** this ticket's to fix: T-336 is a pure
/// relocation, and quietly correcting the SQL inside a move is exactly how a "no behaviour change"
/// claim stops being true. Pinning it here means the fix, when someone takes it, arrives as a
/// deliberate red test rather than as a silent difference nobody notices.
#[tokio::test]
async fn recompute_user_stats_is_reachable_from_services_and_still_correct() {
    let Some((pool, f)) = Fixture::boot("000000000000336001", "t336-correct").await else {
        eprintln!("skip: TEST_DATABASE_URL unset — recompute_user_stats_is_reachable…");
        return;
    };

    let m1 = f.seed_match(&pool, "a").await;
    let m2 = f.seed_match(&pool, "b").await;
    let m3 = f.seed_match(&pool, "c").await;
    f.seed_stat(&pool, m1, "1").await;
    f.seed_stat(&pool, m2, "2").await;
    f.seed_stat(&pool, m3, "3").await;
    // Second line in an already-counted match: DISTINCT is what keeps this from becoming a 4th
    // "deployment".
    f.seed_stat(&pool, m1, "4").await;

    // ── phase 1 ──
    f.seed_registration(&pool, "past-attended-1", 48, "attended")
        .await;
    f.seed_registration(&pool, "past-attended-2", 24, "attended")
        .await;
    f.seed_registration(&pool, "past-registered", 12, "registered")
        .await;
    // Future op the player has signed up for but not yet played. Outside `start_time <= now()`,
    // so it is not a denominator.
    f.seed_registration(&pool, "future-registered", -48, "registered")
        .await;

    recompute_user_stats(&pool, f.player)
        .await
        .expect("recompute must succeed");

    let (deployments, rate) = f.stored_stats(&pool).await;
    assert_eq!(
        deployments, 3,
        "three DISTINCT matches, four stat lines — got {deployments}"
    );
    assert!(
        (rate - 200.0 / 3.0).abs() < 0.01,
        "2 attended of 3 past registrations is 66.66…%, got {rate}"
    );

    // ── phase 2: the untimed numerator, pinned ──
    f.seed_registration(&pool, "future-attended", -72, "attended")
        .await;
    recompute_user_stats(&pool, f.player)
        .await
        .expect("recompute must succeed");
    let (_, rate) = f.stored_stats(&pool).await;
    assert!(
        (rate - 100.0).abs() < 0.01,
        "the shipped SQL counts attended rows without a time filter, so a future op marked \
         attended raises the numerator against an unchanged denominator: expected 3/3 = 100, \
         got {rate}. If this is now 75, the SQL was changed — which is a behaviour change, not \
         a move."
    );

    f.reset(&pool).await;
}

/// A player with rows in neither table reads zero, not a divide-by-zero.
///
/// `past_registered = 0` is the one branch of the moved function that is not a query result, and
/// the arithmetic that produces it (`0.0` rather than `attended / 0`) is the sort of thing a move
/// can silently drop.
#[tokio::test]
async fn a_player_with_no_history_reads_zero_rather_than_dividing_by_zero() {
    let Some((pool, f)) = Fixture::boot("000000000000336002", "t336-empty").await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_player_with_no_history…");
        return;
    };
    // Give the row a non-zero starting point so "still zero" cannot be the initial value.
    sqlx::query(
        "UPDATE users SET total_deployments = 99, attendance_rate = 42 WHERE discord_id = $1",
    )
    .bind(f.player)
    .execute(&pool)
    .await
    .expect("prime");

    recompute_user_stats(&pool, f.player)
        .await
        .expect("recompute must succeed");

    assert_eq!(
        f.stored_stats(&pool).await,
        (0, 0.0),
        "no matches and no registrations must recompute to (0, 0)"
    );
}

/// The best-effort wrapper T-336 folded in writes the same numbers on the happy path.
///
/// It is infallible by design — the point is that it does not swallow the *work*, only the error.
#[tokio::test]
async fn the_best_effort_wrapper_still_writes_the_numbers() {
    let Some((pool, f)) = Fixture::boot("000000000000336003", "t336-wrapper").await else {
        eprintln!("skip: TEST_DATABASE_URL unset — the_best_effort_wrapper…");
        return;
    };
    let m = f.seed_match(&pool, "wrapper").await;
    f.seed_stat(&pool, m, "w").await;

    recompute_user_stats_best_effort(&pool, f.player, "T-336 wrapper check").await;

    let (deployments, _) = f.stored_stats(&pool).await;
    assert_eq!(deployments, 1, "the wrapper must actually recompute");
    f.reset(&pool).await;
}

/// Class-R: the function has exactly one definition, and it is not in `handlers/`.
///
/// T-326's whole argument was that two definitions of "a deployment" drifting apart is the same
/// silent-wrong-number bug the backfill was filed to fix. A move that left a copy behind — or a
/// later slice that re-derived the SQL in a handler — would satisfy every test above.
#[test]
fn the_sql_lives_only_in_the_service() {
    let service = include_str!("../src/services/user_stats.rs");
    assert!(
        service.contains("SELECT count(DISTINCT match_id) FROM match_player_stats"),
        "services/user_stats.rs no longer owns the deployment count"
    );
    for handler in [
        include_str!("../src/handlers/telemetry.rs"),
        include_str!("../src/handlers/me.rs"),
        include_str!("../src/handlers/deployments.rs"),
    ] {
        assert!(
            !handler.contains("count(DISTINCT match_id) FROM match_player_stats"),
            "a handler re-derives the deployment count — that is the two-definitions drift T-326 \
             refused and T-336 moved this function to prevent"
        );
        assert!(
            !handler.contains("UPDATE users SET total_deployments"),
            "a handler writes users.total_deployments directly — services::recompute_user_stats \
             must stay its only writer"
        );
    }
}
