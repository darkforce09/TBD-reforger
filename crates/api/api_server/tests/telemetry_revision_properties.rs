//! Generated results-revision streams hold the production ingest transaction to an independent
//! oracle of the match's revision history.
//!
//! **Role:** the property `telemetry_revisions_contribute_exactly_once`: every case registers a
//! fresh match and delivers a generated stream of revisions (fresh revisions in any order, exact
//! replays, digest conflicts, pending reports after finalisation) through the production
//! `register_match` and `ingest_results_revision`; each answer or refusal equals the oracle's
//! verdict, and after every delivery the match row, its player lines, the leaderboard totals and
//! account deployments of its linked players, and its unlinked-player audit rows equal the oracle
//! of the highest applied revision.
//! **Position:** a `db test-it` binary over its own database; it drives the services directly
//! with the machine caller of one `telemetry_support::match_reports::ReportingServer`, bypassing
//! only the HTTP decoding layer that `telemetry_revisions.rs` covers.
//! **Signals & state:** one database, one application state and one reporting server serve every
//! case; each case owns a unique source match id and freshly seeded accounts, so cases never share
//! rows. The coverage tally is test-local.
//! **Invariants:** only a strictly higher revision changes stored facts; the stored revision again
//! is inert with equal content and a 409 `REVISION_CONFLICT` with other content; a lower revision
//! is a 409 `STALE_REVISION`; a finalized match refuses a pending report with a 409
//! `MATCH_FINALIZED` and keeps its first `finalized_at`; every applied revision with an unlinked
//! player contributes exactly one audit row, and no other delivery contributes one.

mod common;
mod telemetry_support;

use std::cell::Cell;

use api_caller_identity::machine_caller::{MachineCaller, authenticate_machine};
use api_identifiers::MatchId;
use api_match_telemetry::models::match_registration::decode_registration;
use api_match_telemetry::models::match_results_revision::{
    MatchResultsAnswer, decode_results_revision,
};
use api_match_telemetry::services::match_registration::register_match;
use api_match_telemetry::services::match_results_ingest::ingest_results_revision;
use api_state::AppState;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::{REGISTERED_STARTED_AT, ReportingServer};
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player,
};
use uuid::Uuid;

/// Cases the property executes.
const CASES: u32 = 256;

/// The source event id every generated player line carries.
const LIFE: &str = "life-1";

/// Players per case: two linked accounts, then one identity no account owns.
const PLAYERS: usize = 3;

/// A reported mission outcome; everything but `Pending` finalizes the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Pending,
    Success,
    Failure,
    Aborted,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Aborted => "aborted",
        }
    }
}

/// One player's line in a report: present with complete counters, or named as removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scoreline {
    present: bool,
    kills: i64,
    deaths: i64,
}

/// The content of a report apart from its revision number. Every report names every player,
/// present or removed, so the stored lines are exactly the applied report's present lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReportContent {
    outcome: Outcome,
    lines: [Scoreline; PLAYERS],
}

/// One delivery of the stream: a report content under a revision number, or an exact replay of
/// an earlier delivery (a retry after a lost answer).
#[derive(Debug, Clone, Copy)]
enum Delivery {
    Fresh { revision: i64, content: usize },
    Replay { back: usize },
}

/// A generated stream: the distinct report contents and the deliveries that post them.
#[derive(Debug, Clone)]
struct RevisionStream {
    contents: Vec<ReportContent>,
    deliveries: Vec<Delivery>,
}

impl RevisionStream {
    /// Each delivery as `(revision, content)`; a replay with nothing that far back posts the
    /// first content as revision 1.
    fn posts(&self) -> Vec<(i64, ReportContent)> {
        let mut posts: Vec<(i64, ReportContent)> = Vec::with_capacity(self.deliveries.len());
        for delivery in &self.deliveries {
            let post = match *delivery {
                Delivery::Fresh { revision, content } => {
                    (revision, self.contents[content % self.contents.len()])
                }
                Delivery::Replay { back } => match posts.len().checked_sub(back) {
                    Some(index) => posts[index],
                    None => (1, self.contents[0]),
                },
            };
            posts.push(post);
        }
        posts
    }
}

fn outcome_strategy() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        2 => Just(Outcome::Pending),
        1 => Just(Outcome::Success),
        1 => Just(Outcome::Failure),
        1 => Just(Outcome::Aborted),
    ]
}

fn scoreline_strategy() -> impl Strategy<Value = Scoreline> {
    (prop::bool::weighted(0.8), 0_i64..4, 0_i64..4).prop_map(|(present, kills, deaths)| Scoreline {
        present,
        kills,
        deaths,
    })
}

fn content_strategy() -> impl Strategy<Value = ReportContent> {
    (
        outcome_strategy(),
        prop::array::uniform3(scoreline_strategy()),
    )
        .prop_map(|(outcome, lines)| ReportContent { outcome, lines })
}

fn delivery_strategy() -> impl Strategy<Value = Delivery> {
    prop_oneof![
        3 => (1_i64..=4, 0_usize..3)
            .prop_map(|(revision, content)| Delivery::Fresh { revision, content }),
        1 => (1_usize..4).prop_map(|back| Delivery::Replay { back }),
    ]
}

fn stream_strategy() -> impl Strategy<Value = RevisionStream> {
    (
        prop::collection::vec(content_strategy(), 1..=3),
        prop::collection::vec(delivery_strategy(), 1..=8),
    )
        .prop_map(|(contents, deliveries)| RevisionStream {
            contents,
            deliveries,
        })
}

/// The players of one case: their Arma identities and owning accounts, in line order.
struct Roster {
    arma: [String; PLAYERS],
    owners: [Option<String>; PLAYERS],
}

impl Roster {
    async fn seed(pool: &PgPool) -> Self {
        let (first, _, first_arma) = seed_player(pool, "revision-property").await;
        let (second, _, second_arma) = seed_player(pool, "revision-property").await;
        Self {
            arma: [
                first_arma,
                second_arma,
                common::unique_arma("revision-property-unlinked"),
            ],
            owners: [Some(first), Some(second), None],
        }
    }

    /// The `match-results` body of `content` as revision `revision` of `source`.
    fn body(&self, source: &str, revision: i64, content: &ReportContent) -> Value {
        let mut players = Vec::new();
        let mut removed = Vec::new();
        for (scoreline, arma) in content.lines.iter().zip(&self.arma) {
            if scoreline.present {
                let measured = counters(scoreline.kills, scoreline.deaths);
                players.push(line(arma, LIFE, Some(measured)));
            } else {
                removed.push(json!({ "arma_id": arma, "source_event_id": LIFE }));
            }
        }
        let mut body = report(source, content.outcome.as_str(), players);
        body["removed_lines"] = Value::Array(removed);
        body["revision"] = json!(revision);
        body
    }
}

/// What the oracle expects of one delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Apply,
    Duplicate,
    Refuse(&'static str),
}

/// The revision history of one match as the ingest rules define it.
#[derive(Default)]
struct MatchOracle {
    revision: i64,
    applied: Option<(ReportContent, String)>,
    finalized: bool,
    unlinked_audits: i64,
}

impl MatchOracle {
    fn decide(&self, revision: i64, content: &ReportContent) -> Verdict {
        if revision < self.revision {
            Verdict::Refuse("STALE_REVISION")
        } else if revision == self.revision {
            match &self.applied {
                Some((stored, _)) if stored == content => Verdict::Duplicate,
                _ => Verdict::Refuse("REVISION_CONFLICT"),
            }
        } else if self.finalized && content.outcome == Outcome::Pending {
            Verdict::Refuse("MATCH_FINALIZED")
        } else {
            Verdict::Apply
        }
    }

    fn apply(&mut self, revision: i64, content: ReportContent, sha256: String, roster: &Roster) {
        self.revision = revision;
        self.finalized |= content.outcome != Outcome::Pending;
        let unlinked_present = content
            .lines
            .iter()
            .zip(&roster.owners)
            .any(|(scoreline, owner)| scoreline.present && owner.is_none());
        self.unlinked_audits += i64::from(unlinked_present);
        self.applied = Some((content, sha256));
    }

    fn outcome(&self) -> &'static str {
        self.applied
            .as_ref()
            .map_or("pending", |(content, _)| content.outcome.as_str())
    }
}

/// Deliveries observed per verdict over the whole run; the property fails if any stays zero.
#[derive(Default)]
struct Coverage {
    applied: Cell<u32>,
    duplicates: Cell<u32>,
    stale: Cell<u32>,
    conflicts: Cell<u32>,
    finalized: Cell<u32>,
}

impl Coverage {
    fn count(&self, verdict: Verdict) {
        let cell = match verdict {
            Verdict::Apply => &self.applied,
            Verdict::Duplicate => &self.duplicates,
            Verdict::Refuse("STALE_REVISION") => &self.stale,
            Verdict::Refuse("REVISION_CONFLICT") => &self.conflicts,
            Verdict::Refuse(_) => &self.finalized,
        };
        cell.set(cell.get() + 1);
    }
}

/// The shared state every case reports through.
struct RevisionWorld {
    state: AppState,
    server: ReportingServer,
    caller: MachineCaller,
}

impl RevisionWorld {
    async fn open() -> Self {
        let (app, pool, state) = boot_with_state().await;
        let server = ReportingServer::open(&app, &pool, "Revision property host").await;
        let caller = authenticate_machine(&pool, &server.session.secret)
            .await
            .expect("the reporting server's runtime credential authenticates");
        Self {
            state,
            server,
            caller,
        }
    }

    /// Register a fresh source match through the production registration.
    async fn register(&self, source: &str) -> Uuid {
        let registration = decode_registration(&json!({
            "source_match_id": source,
            "runtime_session_id": self.server.session.id,
            "started_at": REGISTERED_STARTED_AT,
        }))
        .expect("the generated registration is valid");
        let answer = register_match(&self.state, &self.caller, &registration)
            .await
            .expect("register the case's match");
        assert!(answer.registered, "a fresh source registers a new match");
        answer.match_id.into_inner()
    }
}

fn check_answer(
    answer: &MatchResultsAnswer,
    (match_id, revision, applied): (Uuid, i64, bool),
    content: &ReportContent,
    roster: &Roster,
) -> TestCaseResult {
    let present: Vec<usize> = (0..PLAYERS)
        .filter(|index| content.lines[*index].present)
        .collect();
    let unlinked: Vec<String> = present
        .iter()
        .filter(|index| roster.owners[**index].is_none())
        .map(|index| roster.arma[*index].clone())
        .collect();
    prop_assert_eq!(answer.match_id, MatchId::new(match_id));
    prop_assert_eq!(answer.revision, revision);
    prop_assert_eq!(answer.applied, applied);
    prop_assert_eq!(answer.players, present.len());
    prop_assert_eq!(answer.unlinked, unlinked.len());
    prop_assert_eq!(answer.linked, present.len() - unlinked.len());
    prop_assert_eq!(&answer.unlinked_arma_ids, &unlinked);
    Ok(())
}

type StoredLine = (String, String, Option<i64>, Option<i64>, Option<String>);

/// The persisted match, lines, aggregates and audit rows equal the oracle.
async fn check_persisted(
    pool: &PgPool,
    match_id: Uuid,
    oracle: &MatchOracle,
    roster: &Roster,
    first_finalized: &mut Option<chrono::DateTime<chrono::Utc>>,
) -> TestCaseResult {
    let (revision, sha256, outcome, finalized_at, _) = match_state(pool, match_id).await;
    prop_assert_eq!(revision, oracle.revision);
    prop_assert_eq!(sha256, oracle.applied.as_ref().map(|(_, sha)| sha.clone()));
    prop_assert_eq!(outcome.as_str(), oracle.outcome());
    prop_assert_eq!(finalized_at.is_some(), oracle.finalized);
    match first_finalized {
        Some(first) => prop_assert_eq!(finalized_at, Some(*first), "finalized_at moved"),
        None => *first_finalized = finalized_at,
    }

    let applied = oracle.applied.as_ref().map(|(content, _)| content);
    let mut expected: Vec<StoredLine> = Vec::new();
    for index in 0..PLAYERS {
        if let Some(scoreline) = applied.map(|content| content.lines[index])
            && scoreline.present
        {
            expected.push((
                roster.arma[index].clone(),
                LIFE.to_owned(),
                Some(scoreline.kills),
                Some(scoreline.deaths),
                roster.owners[index].clone(),
            ));
        }
    }
    expected.sort();
    let mut stored = player_rows(pool, match_id).await;
    stored.sort();
    prop_assert_eq!(stored, expected);

    for index in 0..PLAYERS {
        let Some(owner) = &roster.owners[index] else {
            continue;
        };
        let line = applied
            .map(|content| content.lines[index])
            .filter(|scoreline| scoreline.present);
        prop_assert_eq!(
            leaderboard_kills(pool, owner).await,
            (
                line.map(|scoreline| scoreline.kills),
                i64::from(line.is_some())
            ),
            "leaderboard kills and deployments of player {}",
            index
        );
    }

    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs
         WHERE action = 'match.unlinked_players' AND target_type = 'match' AND target_id = $1",
    )
    .bind(match_id.to_string())
    .fetch_one(pool)
    .await
    .expect("count unlinked-player audit rows");
    prop_assert_eq!(audits, oracle.unlinked_audits, "unlinked-player audit rows");
    Ok(())
}

/// Deliver every post of `stream` to a freshly registered match and hold each step to the oracle.
async fn run_stream(
    world: &RevisionWorld,
    stream: &RevisionStream,
    coverage: &Coverage,
) -> TestCaseResult {
    let pool = &world.state.pool;
    let source = format!("revision-property-{}", Uuid::new_v4());
    let match_id = world.register(&source).await;
    let roster = Roster::seed(pool).await;
    let mut oracle = MatchOracle::default();
    let mut first_finalized = None;
    check_persisted(pool, match_id, &oracle, &roster, &mut first_finalized).await?;

    for (revision, content) in stream.posts() {
        let decoded = decode_results_revision(&roster.body(&source, revision, &content))
            .expect("generated revisions are valid");
        let verdict = oracle.decide(revision, &content);
        coverage.count(verdict);
        let result = ingest_results_revision(&world.state, &world.caller, &decoded).await;
        match (verdict, result) {
            (Verdict::Apply, Ok(answer)) => {
                check_answer(&answer, (match_id, revision, true), &content, &roster)?;
                oracle.apply(revision, content, decoded.report_sha256.clone(), &roster);
            }
            (Verdict::Duplicate, Ok(answer)) => {
                check_answer(&answer, (match_id, revision, false), &content, &roster)?;
            }
            (Verdict::Refuse(code), Err(error)) => {
                prop_assert_eq!(error.status, axum::http::StatusCode::CONFLICT);
                let details = error.details.unwrap_or(Value::Null);
                prop_assert_eq!(details["code"].as_str(), Some(code));
            }
            (verdict, Ok(answer)) => {
                prop_assert!(false, "expected {verdict:?}, ingest answered {answer:?}");
            }
            (verdict, Err(error)) => {
                prop_assert!(false, "expected {verdict:?}, ingest refused {error:?}");
            }
        }
        check_persisted(pool, match_id, &oracle, &roster, &mut first_finalized).await?;
    }
    Ok(())
}

#[test]
fn telemetry_revisions_contribute_exactly_once() {
    let runtime = tokio::runtime::Runtime::new().expect("build the test runtime");
    let world = runtime.block_on(RevisionWorld::open());
    let coverage = Coverage::default();
    api_property_evidence::run_property(
        "telemetry_revisions_contribute_exactly_once",
        CASES,
        &stream_strategy(),
        |stream| runtime.block_on(run_stream(&world, &stream, &coverage)),
    );
    let tally = [
        ("applied", coverage.applied.get()),
        ("duplicates", coverage.duplicates.get()),
        ("stale", coverage.stale.get()),
        ("conflicts", coverage.conflicts.get()),
        ("finalized", coverage.finalized.get()),
    ];
    println!("revision-coverage: {tally:?}");
    for (verdict, count) in tally {
        assert!(count > 0, "no generated delivery exercised {verdict}");
    }
}
