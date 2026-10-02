//! Controlled races of a review decision and of results-revision ingest, each played in both
//! interleavings.
//!
//! **Role:** proves that an approval and a rejection of one review decide it exactly once, that
//! two identical results revisions apply once, and that corrected revisions `n` and `n + 1` end at
//! revision `n + 1` whichever of them arrives first.
//! **Position:** drives the real routes (`POST /api/v1/approvals/:id/approve`,
//! `POST /api/v1/approvals/:id/reject`, `POST /api/v1/ingest/match-results`) through
//! `tests/mission_artifact_support` and `tests/telemetry_support`; orders the contenders with a
//! failpoint pause (`ReviewDecisionBeforeCommit`, `ResultsRevisionBeforeCommit`) and the wait until
//! the follower queues behind the leader's row locks (`tests/failpoint_and_race_support`).
//! **Signals & state:** one per-binary database; every case holds the failpoint suite lock for its
//! whole body, so no other case arms a point or opens a transaction meanwhile.
//! **Invariants:** the leader holds its transaction open at the paused point until the follower
//! is blocked behind it; every observed answer and the final persisted state are checked in each
//! interleaving.

mod common;
mod failpoint_and_race_support;
mod mission_artifact_support;
mod telemetry_support;

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use failpoint_and_race_support::{
    Failpoint, FailpointArming, FailpointSuiteLock, Interleaving, audit_evidence,
    check_review_decided_once, lock_suite, paused_transaction_backend, run_in_both_orders,
    wait_for_blocked,
};
use mission_artifact_support::MissionFixture;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player,
};

const SUITE: &str = "controlled_races_missions_and_telemetry";

/// The reason every rejection in this suite gives.
const REJECTION_REASON: &str = "The objective markers overlap the insertion point.";

/// The corrected revision that loses the inverted race.
const OLDER: i64 = 2;

/// The corrected revision every order ends at.
const NEWER: i64 = 3;

/// Both answers of one race and how many requests reached the paused point.
struct RaceAnswers {
    leader: (StatusCode, Value),
    follower: (StatusCode, Value),
    arrivals: usize,
}

/// Plays one race: `leader` runs until it waits at `point` inside its transaction, `follower`
/// starts and queues behind the leader's row locks, then the leader commits and both finish.
async fn race_through_pause<Leader, Follower>(
    suite: &FailpointSuiteLock,
    pool: &PgPool,
    point: Failpoint,
    leader: Leader,
    follower: Follower,
) -> RaceAnswers
where
    Leader: Future<Output = (StatusCode, Value)> + Send + 'static,
    Follower: Future<Output = (StatusCode, Value)> + Send + 'static,
{
    let paused = suite.pause(point);
    let leading = tokio::spawn(leader);
    paused.reached().await;
    let holder = paused_transaction_backend(pool).await;
    let following = tokio::spawn(follower);
    wait_for_blocked(pool, holder, 1).await;
    paused.release();
    let leader = leading.await.expect("the leading request completes");
    let follower = following.await.expect("the following request completes");
    RaceAnswers {
        leader,
        follower,
        arrivals: paused.arrivals(),
    }
}

/// One of the two decisions of a review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReviewContender {
    Approve,
    Reject,
}

impl ReviewContender {
    /// Sends the decision of `artifact` through its approvals route as the fixture's reviewer.
    async fn send(
        self,
        fixture: &MissionFixture,
        mission: Uuid,
        artifact: Uuid,
    ) -> (StatusCode, Value) {
        match self {
            Self::Approve => fixture.approve(mission, artifact, None).await,
            Self::Reject => fixture.reject(mission, artifact, REJECTION_REASON).await,
        }
    }

    /// The mission status this decision leaves.
    fn mission_status(self) -> &'static str {
        match self {
            Self::Approve => "live",
            Self::Reject => "rejected",
        }
    }

    /// The review state this decision records.
    fn review_state(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
        }
    }

    /// The audit action this decision appends.
    fn audit_action(self) -> &'static str {
        match self {
            Self::Approve => "mission.approve",
            Self::Reject => "mission.reject",
        }
    }
}

#[tokio::test]
async fn controlled_races_approval_and_rejection_of_one_review_decide_it_once() {
    let suite = lock_suite().await;
    let suite = &suite;
    let fixture = Arc::new(MissionFixture::new(SUITE).await);
    let winners = run_in_both_orders(|order| {
        let fixture = Arc::clone(&fixture);
        async move {
            let (leader, follower) =
                order.arrange(ReviewContender::Approve, ReviewContender::Reject);
            let (mission, _) = fixture
                .compilable_mission(&format!("Review race, {}", order.name()))
                .await;
            let artifact = fixture.submit(mission).await;
            let answers = race_through_pause(
                suite,
                fixture.pool(),
                Failpoint::ReviewDecisionBeforeCommit,
                {
                    let fixture = Arc::clone(&fixture);
                    async move { leader.send(&fixture, mission, artifact).await }
                },
                {
                    let fixture = Arc::clone(&fixture);
                    async move { follower.send(&fixture, mission, artifact).await }
                },
            )
            .await;
            check_decided_by(&fixture, mission, artifact, leader, follower, &answers).await;
            leader
        }
    })
    .await;
    assert_eq!(
        winners,
        [ReviewContender::Approve, ReviewContender::Reject],
        "the decision that reaches the review first decides it, in either order"
    );
}

/// The leader decided the review, the follower was refused before it reached the decision, and
/// the persisted review, mission, comment and audit rows carry the leader's decision alone.
async fn check_decided_by(
    fixture: &MissionFixture,
    mission: Uuid,
    artifact: Uuid,
    leader: ReviewContender,
    follower: ReviewContender,
    answers: &RaceAnswers,
) {
    let (status, body) = &answers.leader;
    assert_eq!(*status, StatusCode::OK, "{leader:?} leads: {body}");
    assert_eq!(body["status"], leader.mission_status(), "{body}");
    let (status, body) = &answers.follower;
    assert_eq!(
        *status,
        StatusCode::CONFLICT,
        "{follower:?} follows: {body}"
    );
    assert_eq!(body["error"], "mission is not pending approval", "{body}");
    assert_eq!(
        answers.arrivals, 1,
        "the follower is refused before it decides anything"
    );

    check_review_decided_once(fixture.pool(), mission)
        .await
        .unwrap();
    let states: Vec<String> =
        sqlx::query_scalar("SELECT state FROM mission_reviews WHERE mission_id = $1")
            .bind(mission)
            .fetch_all(fixture.pool())
            .await
            .expect("read the mission's reviews");
    assert_eq!(states, [leader.review_state()]);
    let (status, approved, reason): (String, Option<Uuid>, String) = sqlx::query_as(
        "SELECT status::text, approved_artifact_id, rejection_reason FROM missions WHERE id = $1",
    )
    .bind(mission)
    .fetch_one(fixture.pool())
    .await
    .expect("read the mission");
    let expected = match leader {
        ReviewContender::Approve => (leader.mission_status(), Some(artifact), ""),
        ReviewContender::Reject => (leader.mission_status(), None, REJECTION_REASON),
    };
    assert_eq!((status.as_str(), approved, reason.as_str()), expected);
    let rejection_comments = fixture
        .count(
            "SELECT count(*) FROM mission_review_comments WHERE mission_id = $1 AND kind = 'rejection'",
            mission,
        )
        .await;
    assert_eq!(
        rejection_comments,
        i64::from(leader == ReviewContender::Reject)
    );
    assert_eq!(fixture.audits(leader.audit_action(), mission).await, 1);
    assert_eq!(fixture.audits(follower.audit_action(), mission).await, 0);
    assert_eq!(
        audit_evidence(fixture.pool(), leader.audit_action(), &mission.to_string())
            .await
            .rows,
        1
    );
}

/// The router, pool and reporting server a results race posts through.
struct ReportingRace {
    app: Router,
    pool: PgPool,
    server: ReportingServer,
}

/// A registered match with one seeded player reported in it.
struct RacedMatch {
    id: Uuid,
    source: String,
    discord_id: String,
    arma_id: String,
}

impl ReportingRace {
    /// Boots the router and opens one reporting server named after `label`.
    async fn open(label: &str) -> Arc<Self> {
        let (app, pool, _state) = boot_with_state().await;
        let server = ReportingServer::open(&app, &pool, &common::unique_arma(label)).await;
        Arc::new(Self { app, pool, server })
    }

    /// Registers a fresh match of this server and seeds the one player its reports name.
    async fn match_with_player(&self, label: &str) -> RacedMatch {
        let (discord_id, _, arma_id) = seed_player(&self.pool, label).await;
        let source = common::unique_arma(label);
        let id = self.server.register_match(&self.app, &source).await;
        RacedMatch {
            id,
            source,
            discord_id,
            arma_id,
        }
    }

    /// The report of `raced` crediting its player with `kills`.
    fn report_crediting(raced: &RacedMatch, kills: i64) -> Value {
        report(
            &raced.source,
            "success",
            vec![line(&raced.arma_id, "life-1", Some(counters(kills, 1)))],
        )
    }

    /// The post of `body` as `revision`, ready to spawn.
    fn post(
        self: &Arc<Self>,
        revision: i64,
        body: Value,
    ) -> impl Future<Output = (StatusCode, Value)> + Send + 'static {
        let race = Arc::clone(self);
        async move { race.server.post_results(&race.app, revision, &body).await }
    }

    /// The stored revision and the one player line of `raced` hold `revision` and `kills`, and
    /// the derived statistics count those kills and one deployment exactly once.
    async fn check_stored(&self, raced: &RacedMatch, revision: i64, kills: i64) {
        let (stored, digest, ..) = match_state(&self.pool, raced.id).await;
        assert_eq!(stored, revision, "the stored revision");
        assert!(digest.is_some(), "an applied revision stores its digest");
        let rows = player_rows(&self.pool, raced.id).await;
        assert_eq!(rows.len(), 1, "one line per reported life: {rows:?}");
        assert_eq!(rows[0].2, Some(kills), "{rows:?}");
        assert_eq!(rows[0].4.as_deref(), Some(raced.discord_id.as_str()));
        assert_eq!(
            leaderboard_kills(&self.pool, &raced.discord_id).await,
            (Some(kills), 1),
            "the derived statistics count the stored revision once"
        );
    }
}

#[tokio::test]
async fn controlled_races_duplicate_identical_revisions_apply_once() {
    let suite = lock_suite().await;
    let suite = &suite;
    let race = ReportingRace::open("races-duplicate").await;
    run_in_both_orders(|order| {
        let race = Arc::clone(&race);
        async move {
            let raced = race.match_with_player("duplicate").await;
            let body = ReportingRace::report_crediting(&raced, 4);
            let (leader, follower) =
                order.arrange(race.post(1, body.clone()), race.post(1, body.clone()));
            let answers = race_through_pause(
                suite,
                &race.pool,
                Failpoint::ResultsRevisionBeforeCommit,
                leader,
                follower,
            )
            .await;
            let (status, applied) = &answers.leader;
            assert_eq!(*status, StatusCode::OK, "{applied}");
            assert_eq!(applied["applied"], true, "the leader applies: {applied}");
            let (status, duplicate) = &answers.follower;
            assert_eq!(*status, StatusCode::OK, "{duplicate}");
            assert_eq!(
                duplicate["applied"], false,
                "the follower is an inert duplicate: {duplicate}"
            );
            for key in [
                "match_id",
                "revision",
                "players",
                "linked",
                "unlinked",
                "unlinked_arma_ids",
            ] {
                assert_eq!(
                    duplicate[key], applied[key],
                    "a duplicate answers the same {key}"
                );
            }
            assert_eq!(
                answers.arrivals, 1,
                "the duplicate writes nothing, so it never reaches the commit point"
            );
            race.check_stored(&raced, 1, 4).await;
        }
    })
    .await;
}

#[tokio::test]
async fn controlled_races_corrected_revisions_in_either_order_end_at_the_newer_revision() {
    let suite = lock_suite().await;
    let suite = &suite;
    let race = ReportingRace::open("races-corrected").await;
    run_in_both_orders(|order| {
        let race = Arc::clone(&race);
        async move {
            let raced = race.match_with_player("corrected").await;
            let (status, first) = race
                .server
                .post_results(&race.app, 1, &ReportingRace::report_crediting(&raced, 1))
                .await;
            assert_eq!(
                (status, &first["applied"]),
                (StatusCode::OK, &json!(true)),
                "{first}"
            );
            let (leader, follower) = order.arrange(
                race.post(OLDER, ReportingRace::report_crediting(&raced, 5)),
                race.post(NEWER, ReportingRace::report_crediting(&raced, 7)),
            );
            let answers = race_through_pause(
                suite,
                &race.pool,
                Failpoint::ResultsRevisionBeforeCommit,
                leader,
                follower,
            )
            .await;
            let (older, newer) = order.arrange(answers.leader, answers.follower);
            let (status, body) = &newer;
            assert_eq!(*status, StatusCode::OK, "{body}");
            assert_eq!(
                (&body["applied"], &body["revision"]),
                (&json!(true), &json!(NEWER)),
                "{body}"
            );
            let (status, body) = &older;
            match order {
                Interleaving::FirstLeads => {
                    assert_eq!(*status, StatusCode::OK, "{body}");
                    assert_eq!(
                        (&body["applied"], &body["revision"]),
                        (&json!(true), &json!(OLDER)),
                        "revision {OLDER} applies before revision {NEWER} arrives: {body}"
                    );
                    assert_eq!(answers.arrivals, 2, "both revisions apply");
                }
                Interleaving::SecondLeads => {
                    assert_eq!(*status, StatusCode::CONFLICT, "{body}");
                    assert_eq!(body["details"]["code"], "STALE_REVISION", "{body}");
                    assert_eq!(
                        body["details"]["revision"], NEWER,
                        "the refusal names the stored revision: {body}"
                    );
                    assert_eq!(answers.arrivals, 1, "the stale revision writes nothing");
                }
            }
            race.check_stored(&raced, NEWER, 7).await;
        }
    })
    .await;
}
