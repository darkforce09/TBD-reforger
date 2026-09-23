//! Public user-stat recomputation counts distinct matches and one past-registration population.
//!
//! Four player-stat rows across three matches distinguish deployments from raw row counts.
//! Attendance uses the same past registrations for numerator and denominator, excludes future
//! operations regardless of their recorded state, and rounds the percentage to two decimals.
//! Telemetry and identity-link integration suites also exercise this service through HTTP.

use sqlx::PgPool;
use uuid::Uuid;
use website_api::core::database;
// The reachability proof: a `pub(super)` `recompute_user_stats` inside the ingest handler could
// not be imported here at all, so this line is itself the assertion.
use website_api::command_center::services::user_stats::{
    recompute_user_stats, recompute_user_stats_best_effort,
};

mod common;

/// Per-test fixture identity. Tests inside one binary run in parallel, so each owns a **distinct**
/// `discord_id` and a distinct row tag — a shared player would have them deleting each other's
/// matches and reading each other's counts, which is a harness bug wearing the costume of a
/// behaviour change. Ids are in the suite-private range and are never content-golden Vance
/// (`…003`).
struct Fixture {
    player: &'static str,
    tag: &'static str,
}

impl Fixture {
    async fn boot(player: &'static str, tag: &'static str) -> Option<(PgPool, Self)> {
        let url = common::require_test_database_url()?;
        let pool = database::connect(&url).await.expect("connect");
        database::migrate(&pool).await.expect("migrate");
        let f = Self { player, tag };
        common::seed_user(
            &pool,
            player,
            "Stats Fixture",
            &format!("stats-arma-{tag}"),
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
            "WITH removed_participation AS (DELETE FROM event_registration_participation WHERE registration_id IN (SELECT id FROM event_registrations WHERE event_mission_id IN \
             (SELECT em.id FROM event_missions em JOIN events e ON e.id = em.event_id \
               WHERE e.created_by = $1))), removed_history AS (DELETE FROM event_registration_history WHERE registration_id IN (SELECT id FROM event_registrations WHERE event_mission_id IN \
             (SELECT em.id FROM event_missions em JOIN events e ON e.id = em.event_id \
               WHERE e.created_by = $1))) DELETE FROM event_registrations WHERE event_mission_id IN \
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
        .bind(format!("stats-arma-{}-{arma_suffix}", self.tag))
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
        .bind(format!("Stats {name}"))
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
        let mut fixture = pool.begin().await.unwrap();
        let allocation = if matches!(
            state,
            "registered" | "legacy_unknown" | "attended" | "no_show"
        ) {
            Some(common::participant_allocation(&mut fixture, em_id, self.player).await)
        } else {
            None
        };
        sqlx::query(
            "INSERT INTO event_registrations (event_mission_id, discord_id, reservation_state, attendance_state, legacy_attendance_state, registered_at, allocation_id) \
             VALUES ($1, $2, CASE WHEN $3 IN ('attended', 'no_show') THEN 'legacy_unknown'::registration_state ELSE $3::registration_state END, CASE WHEN $3 IN ('attended', 'no_show') THEN $3::registration_state END, CASE WHEN $3 IN ('attended', 'no_show') THEN $3::registration_state END, now(), $4)",
        )
        .bind(em_id)
        .bind(self.player)
        .bind(state)
        .bind(allocation)
        .execute(&mut *fixture)
        .await
        .expect("seed registration");
        fixture.commit().await.unwrap();
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

/// The public service writes deployment and attendance aggregates derived from the same facts.
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
/// A future `attended` registration joins neither count. Moving it into the past joins both,
/// producing three attended out of four past registrations, or exactly 75 percent.
#[tokio::test]
async fn recompute_user_stats_is_reachable_from_services_and_still_correct() {
    let Some((pool, f)) = Fixture::boot("000000000000336001", "stats-correct").await else {
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
    f.seed_registration(&pool, "past-no-show", 12, "no_show")
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
        (rate - 66.67).abs() < 1e-9,
        "2 attended of 3 past registrations rounds to 66.67%, got {rate}"
    );

    // Future attendance cannot inflate the numerator or dilute the denominator.
    f.seed_registration(&pool, "future-attended", -72, "attended")
        .await;
    recompute_user_stats(&pool, f.player)
        .await
        .expect("recompute must succeed");
    let (deployments, rate) = f.stored_stats(&pool).await;
    assert_eq!(deployments, 3);
    assert!(
        (rate - 66.67).abs() < 1e-9,
        "a future attended registration must leave both past counts unchanged at 2/3: got {rate}"
    );

    let moved = sqlx::query(
        "UPDATE event_missions SET start_time = now() - interval '1 hour' \
         WHERE mission_id IN (SELECT id FROM missions \
             WHERE author_id = $1 AND title = 'Stats future-attended')",
    )
    .bind(f.tag)
    .execute(&pool)
    .await
    .expect("move the attended operation into the past")
    .rows_affected();
    assert_eq!(moved, 1);
    recompute_user_stats(&pool, f.player)
        .await
        .expect("recompute the expanded past population");
    assert_eq!(
        f.stored_stats(&pool).await,
        (3, 75.0),
        "the newly past attended operation joins numerator and denominator together: 3/4"
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
    let Some((pool, f)) = Fixture::boot("000000000000336002", "stats-empty").await else {
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

/// The best-effort wrapper writes the same numbers on the happy path.
///
/// It is infallible by design — the point is that it does not swallow the *work*, only the error.
#[tokio::test]
async fn the_best_effort_wrapper_still_writes_the_numbers() {
    let Some((pool, f)) = Fixture::boot("000000000000336003", "stats-wrapper").await else {
        eprintln!("skip: TEST_DATABASE_URL unset — the_best_effort_wrapper…");
        return;
    };
    let m = f.seed_match(&pool, "wrapper").await;
    f.seed_stat(&pool, m, "w").await;

    recompute_user_stats_best_effort(&pool, f.player, "wrapper check").await;

    let (deployments, _) = f.stored_stats(&pool).await;
    assert_eq!(deployments, 1, "the wrapper must actually recompute");
    f.reset(&pool).await;
}

/// The function has exactly one definition, and it is not in a handler.
///
/// Two definitions of "a deployment" drifting apart is the same silent-wrong-number bug the
/// backfill exists to prevent. A relocation that left a copy behind — or a later change that
/// re-derived the SQL in a handler — would satisfy every test above.
#[test]
fn the_sql_lives_only_in_the_service() {
    let service = include_str!("../src/command_center/services/user_stats.rs");
    assert!(
        service.contains("SELECT count(DISTINCT match_id) FROM match_player_stats"),
        "command_center/services/user_stats.rs no longer owns the deployment count"
    );
    for handler in [
        include_str!("../src/match_telemetry/handlers/match_results.rs"),
        include_str!("../src/operations/services/participation_attribution.rs"),
        include_str!("../src/identity_and_access/handlers/arma_link_confirmation.rs"),
        include_str!("../src/identity_and_access/handlers/arma_link_codes.rs"),
        include_str!("../src/operations/handlers/member_service_record.rs"),
    ] {
        assert!(
            !handler.contains("count(DISTINCT match_id) FROM match_player_stats"),
            "a handler re-derives the deployment count — that is the two-definitions drift this \
             function lives in the service layer to prevent"
        );
        assert!(
            !handler.contains("UPDATE users SET total_deployments"),
            "a handler writes users.total_deployments directly — services::recompute_user_stats \
             must stay its only writer"
        );
    }
}

/// Schedule passage changes displayed rates even when no business write refreshes the cache.
#[tokio::test]
async fn attendance_read_crosses_database_clock_boundary_without_a_write() {
    use website_api::identity_and_access::services::user_lookup::load_user;
    let (pool, f) = Fixture::boot("000000000000336004", "stats-clock")
        .await
        .unwrap();
    f.seed_registration(&pool, "decided", -1, "attended").await;
    recompute_user_stats(&pool, f.player).await.unwrap();
    assert_eq!(f.stored_stats(&pool).await.1, 0.0);
    assert_eq!(
        load_user(&pool, f.player)
            .await
            .unwrap()
            .unwrap()
            .attendance_rate,
        0.0
    );
    // PostgreSQL supplies and observes the boundary; this does not assume host clock agreement.
    let boundary: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE event_missions SET start_time = clock_timestamp() + interval '200 milliseconds'
        WHERE mission_id IN (SELECT id FROM missions WHERE author_id = $1) RETURNING start_time",
    )
    .bind(f.tag)
    .fetch_one(&pool)
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let elapsed: bool = sqlx::query_scalar("SELECT clock_timestamp() >= $1")
                .bind(boundary)
                .fetch_one(&pool)
                .await
                .unwrap();
            if elapsed {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("database clock reaches the scheduled boundary");
    assert_eq!(
        f.stored_stats(&pool).await.1,
        0.0,
        "there was no aggregate write"
    );
    assert_eq!(
        load_user(&pool, f.player)
            .await
            .unwrap()
            .unwrap()
            .attendance_rate,
        100.0
    );
    f.reset(&pool).await;
}
