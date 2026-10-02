//! Generated audit appends, commit orders and publication batch sizes hold the production audit
//! publisher and delivery stream to an independent publication model.
//!
//! **Role:** runs the property `audit_publication_preserves_committed_event_delivery`. Every case
//! holds several transactions open, appends required audit rows through the production
//! `append_required_audit` in a generated interleaving, commits or rolls back each transaction
//! in a generated order, publishes through the production `publish_audit_batch` with generated
//! batch sizes before and between the finishes, drains what is left, and finally reads the
//! production `audit_delivery_stream` from the cursor the case started at.
//! **Position:** drives `administration::services::{required_audit, audit_publication,
//! audit_delivery, audit_notifier}` against this binary's private database. [`PublicationModel`]
//! is the oracle: a committed row becomes pending at its commit, and a publish takes the lowest
//! pending audit ids, up to the clamped batch size, numbering them after the tail in audit-id
//! order.
//! **Signals & state:** one runtime, pool and audit listener serve every case; each case owns a
//! fresh actor and unique action names and starts after draining whatever is pending, so the
//! cases share only the publication sequence, whose tail the model reads as its start. This
//! binary holds only this test, so no other publisher runs beside a case.
//! **Invariants:**
//! - every committed row is published exactly once, with sequences contiguous after the start;
//! - a rolled-back row is never stored, never pending and never published;
//! - every publish reports exactly the model's count, and the drain ends on an empty publish;
//! - the delivery stream yields `ready` at the starting cursor, then exactly the model's
//!   (sequence, audit id) pairs in sequence order, each carrying the action it was appended with.

mod common;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use api::administration::services::audit_delivery::{AuditStreamItem, audit_delivery_stream};
use api::administration::services::audit_notifier::AuditNotify;
use api::administration::services::audit_publication::publish_audit_batch;
use api::administration::services::required_audit::append_required_audit;
use api::core::database;
use futures::StreamExt;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const PROPERTY: &str = "audit_publication_preserves_committed_event_delivery";
const CASES: u32 = 256;
/// The bound on every awaited step of a case: a stream item, a publish, a commit.
const STEP_DEADLINE: Duration = Duration::from_secs(10);
/// The delivery stream's timer; its first tick fires at once, so delivery never waits on it.
const STREAM_POLL: Duration = Duration::from_millis(20);
/// The batch size `publish_audit_batch` clamps every request into.
const PUBLISHER_BATCH_RANGE: std::ops::RangeInclusive<i64> = 1..=1000;
/// The batch size the case preamble drains leftovers with.
const DRAIN_EVERYTHING: i64 = 1000;

/// One generated publication case.
#[derive(Debug, Clone)]
struct PublicationScenario {
    /// Per held transaction: `true` commits it, `false` rolls it back.
    commits: Vec<bool>,
    /// The transaction index of every append, in append order, which is audit-id order.
    appends: Vec<usize>,
    /// Transaction indexes in the order they commit or roll back.
    finishes: Vec<usize>,
    /// The publish batch size before the first finish (index 0) and after each finish; `None`
    /// publishes nothing at that point.
    publishes: Vec<Option<i64>>,
    /// The batch size that drains what the scenario leaves pending.
    drain: i64,
}

/// Requested batch sizes: some below the clamp, mostly small enough to split the pending rows,
/// and the publisher's maximum.
fn batch_size() -> impl Strategy<Value = i64> {
    prop_oneof![1 => -2i64..=0, 6 => 1i64..=4, 1 => Just(1000i64)]
}

fn scenario() -> impl Strategy<Value = PublicationScenario> {
    vec(1usize..=2, 1..=5).prop_flat_map(|appends_per_transaction| {
        let transactions = appends_per_transaction.len();
        let append_schedule: Vec<usize> = appends_per_transaction
            .iter()
            .enumerate()
            .flat_map(|(transaction, &count)| std::iter::repeat_n(transaction, count))
            .collect();
        (
            vec(proptest::bool::weighted(0.75), transactions),
            Just(append_schedule).prop_shuffle(),
            Just((0..transactions).collect::<Vec<_>>()).prop_shuffle(),
            vec(proptest::option::of(batch_size()), transactions + 1),
            1i64..=3,
        )
            .prop_map(|(commits, appends, finishes, publishes, drain)| {
                PublicationScenario {
                    commits,
                    appends,
                    finishes,
                    publishes,
                    drain,
                }
            })
    })
}

/// How often the generated cases reached the interleavings the property exists for; each must
/// be reached at least once, so a generator that stops reaching one fails the run.
#[derive(Debug, Default)]
struct PublicationCoverage {
    /// Cases that rolled back at least one appended row.
    rolled_back_cases: Cell<u32>,
    /// Cases that published a lower audit id after a higher one: commit order inverted id order.
    inverted_cases: Cell<u32>,
    /// Publishes that left committed rows pending for a later batch.
    split_publishes: Cell<u32>,
    /// Publishes whose requested size was clamped up to one and still published a row.
    clamped_publishes: Cell<u32>,
}

impl PublicationCoverage {
    fn count(counter: &Cell<u32>) {
        counter.set(counter.get() + 1);
    }

    fn assert_reached(&self) {
        for (name, counter) in [
            ("rolled_back_cases", &self.rolled_back_cases),
            ("inverted_cases", &self.inverted_cases),
            ("split_publishes", &self.split_publishes),
            ("clamped_publishes", &self.clamped_publishes),
        ] {
            assert!(
                counter.get() > 0,
                "no generated case reached {name}: {self:?}"
            );
        }
    }
}

/// The oracle: pending committed rows and the publication list it predicts.
#[derive(Debug)]
struct PublicationModel {
    /// The last predicted publication sequence.
    tail: i64,
    /// Committed audit ids not yet published.
    pending: BTreeSet<i64>,
    /// Predicted (sequence, audit id) pairs after the case's starting tail, in sequence order.
    published: Vec<(i64, i64)>,
}

impl PublicationModel {
    fn starting_at(tail: i64) -> Self {
        Self {
            tail,
            pending: BTreeSet::new(),
            published: Vec::new(),
        }
    }

    fn commit(&mut self, audit_ids: &[i64]) {
        self.pending.extend(audit_ids.iter().copied());
    }

    /// Publish the lowest pending ids, up to the clamped `requested` size; returns the count.
    fn publish(&mut self, requested: i64) -> u64 {
        let limit = requested.clamp(*PUBLISHER_BATCH_RANGE.start(), *PUBLISHER_BATCH_RANGE.end());
        let batch: Vec<i64> = self
            .pending
            .iter()
            .copied()
            .take(usize::try_from(limit).expect("clamped batch size is positive"))
            .collect();
        for audit_id in &batch {
            self.pending.remove(audit_id);
            self.tail += 1;
            self.published.push((self.tail, *audit_id));
        }
        batch.len() as u64
    }
}

async fn publication_tail(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT last_sequence FROM audit_publication_state WHERE singleton = true")
        .fetch_one(pool)
        .await
        .expect("read the publication tail")
}

async fn publish(pool: &PgPool, batch_size: i64) -> u64 {
    tokio::time::timeout(STEP_DEADLINE, publish_audit_batch(pool, batch_size))
        .await
        .expect("a publish never waits on a held audit transaction")
        .expect("publish committed audit rows")
}

/// Publish with `batch_size` and hold the result to the model's prediction.
async fn publish_step(
    pool: &PgPool,
    coverage: &PublicationCoverage,
    model: &mut PublicationModel,
    batch_size: i64,
) -> TestCaseResult {
    let expected = model.publish(batch_size);
    if expected > 0 && !model.pending.is_empty() {
        PublicationCoverage::count(&coverage.split_publishes);
    }
    if expected > 0 && batch_size < 1 {
        PublicationCoverage::count(&coverage.clamped_publishes);
    }
    let published = publish(pool, batch_size).await;
    prop_assert_eq!(
        published,
        expected,
        "publish({}) count differs from the model",
        batch_size
    );
    Ok(())
}

async fn next_item<S>(stream: &mut S) -> AuditStreamItem
where
    S: futures::Stream<Item = AuditStreamItem> + Unpin,
{
    tokio::time::timeout(STEP_DEADLINE, stream.next())
        .await
        .expect("the delivery stream yields within the step deadline")
        .expect("the delivery stream stays open")
}

async fn run_case(
    pool: &PgPool,
    notify: &AuditNotify,
    coverage: &PublicationCoverage,
    scenario: PublicationScenario,
) -> TestCaseResult {
    let case = Uuid::new_v4();
    let actor = format!("audit-publication-property-{case}");
    common::seed_user(
        pool,
        &actor,
        "Audit publication property actor",
        &common::unique_arma("audit-publication-property"),
        "enlisted",
    )
    .await;
    while publish(pool, DRAIN_EVERYTHING).await > 0 {}
    let start = publication_tail(pool).await;
    let mut model = PublicationModel::starting_at(start);

    // Every transaction begins before the first append and stays open until its finish.
    let mut held: Vec<Option<Transaction<'static, Postgres>>> = Vec::new();
    for _ in &scenario.commits {
        held.push(Some(pool.begin().await.expect("begin a held transaction")));
    }
    let mut appended: Vec<Vec<i64>> = vec![Vec::new(); scenario.commits.len()];
    let mut actions: BTreeMap<i64, String> = BTreeMap::new();
    for (ordinal, &transaction) in scenario.appends.iter().enumerate() {
        let action = format!("property.audit_publication.{case}.{ordinal}");
        let connection = &mut **held[transaction]
            .as_mut()
            .expect("appends happen before any finish");
        append_required_audit(
            connection,
            &actor,
            &action,
            &actor,
            "generated audit publication case",
        )
        .await
        .expect("append a required audit row");
        let audit_id: i64 = sqlx::query_scalar("SELECT id FROM audit_logs WHERE action = $1")
            .bind(&action)
            .fetch_one(&mut **held[transaction].as_mut().expect("still held"))
            .await
            .expect("read the appended audit id inside its transaction");
        appended[transaction].push(audit_id);
        actions.insert(audit_id, action);
    }

    if let Some(batch_size) = scenario.publishes[0] {
        publish_step(pool, coverage, &mut model, batch_size).await?;
    }
    let mut rolled_back: Vec<i64> = Vec::new();
    for (step, &transaction) in scenario.finishes.iter().enumerate() {
        let finishing = held[transaction]
            .take()
            .expect("every transaction finishes once");
        if scenario.commits[transaction] {
            tokio::time::timeout(STEP_DEADLINE, finishing.commit())
                .await
                .expect("a commit never waits on a publisher")
                .expect("commit a held transaction");
            model.commit(&appended[transaction]);
        } else {
            finishing
                .rollback()
                .await
                .expect("roll back a held transaction");
            rolled_back.extend(&appended[transaction]);
        }
        if let Some(batch_size) = scenario.publishes[step + 1] {
            publish_step(pool, coverage, &mut model, batch_size).await?;
        }
    }
    loop {
        let expected = model.publish(scenario.drain);
        let published = publish(pool, scenario.drain).await;
        prop_assert_eq!(published, expected, "drain count differs from the model");
        if published == 0 {
            break;
        }
    }
    prop_assert!(model.pending.is_empty(), "the drain empties the model");

    // Persisted publication equals the model: contiguous, exactly once, committed rows only.
    let persisted: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT sequence, audit_id FROM audit_publications WHERE sequence > $1 ORDER BY sequence",
    )
    .bind(start)
    .fetch_all(pool)
    .await
    .expect("read the case's publications");
    prop_assert_eq!(&persisted, &model.published);
    prop_assert_eq!(publication_tail(pool).await, model.tail);
    let sequences: Vec<i64> = persisted.iter().map(|(sequence, _)| *sequence).collect();
    prop_assert_eq!(
        sequences,
        (start + 1..=model.tail).collect::<Vec<_>>(),
        "publication sequences are contiguous after the start"
    );
    let committed: BTreeSet<i64> = scenario
        .commits
        .iter()
        .enumerate()
        .filter(|(_, commits)| **commits)
        .flat_map(|(transaction, _)| appended[transaction].iter().copied())
        .collect();
    let published: Vec<i64> = persisted.iter().map(|(_, audit_id)| *audit_id).collect();
    prop_assert_eq!(published.len(), committed.len(), "each committed row once");
    prop_assert_eq!(
        published.iter().copied().collect::<BTreeSet<_>>(),
        committed
    );
    let rolled_back_traces: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM audit_logs WHERE id = ANY($1))
              + (SELECT count(*) FROM audit_publication_pending WHERE audit_id = ANY($1))
              + (SELECT count(*) FROM audit_publications WHERE audit_id = ANY($1))",
    )
    .bind(&rolled_back)
    .fetch_one(pool)
    .await
    .expect("look for rolled-back audit rows");
    prop_assert_eq!(rolled_back_traces, 0, "a rolled-back row leaves no trace");

    // Delivery replays exactly the model's publications, in sequence order.
    let stream = audit_delivery_stream(pool.clone(), notify.clone(), STREAM_POLL, Some(start))
        .await
        .expect("open the delivery stream");
    let mut stream = Box::pin(stream);
    match next_item(&mut stream).await {
        AuditStreamItem::Ready(ready) => prop_assert_eq!(ready.resume_after, start),
        other => {
            return Err(TestCaseError::fail(format!(
                "the stream opens with ready, not {other:?}"
            )));
        }
    }
    let mut delivered: Vec<(i64, i64)> = Vec::new();
    while delivered.len() < model.published.len() {
        match next_item(&mut stream).await {
            AuditStreamItem::Delivery(delivery) => {
                prop_assert_eq!(
                    actions.get(&delivery.row.id),
                    Some(&delivery.row.action),
                    "a delivery carries the row appended under its id"
                );
                delivered.push((delivery.sequence, delivery.row.id));
            }
            other => {
                return Err(TestCaseError::fail(format!(
                    "no reset is due after the start: {other:?}"
                )));
            }
        }
    }
    prop_assert_eq!(&delivered, &model.published);
    if !rolled_back.is_empty() {
        PublicationCoverage::count(&coverage.rolled_back_cases);
    }
    if model.published.windows(2).any(|pair| pair[1].1 < pair[0].1) {
        PublicationCoverage::count(&coverage.inverted_cases);
    }
    Ok(())
}

#[test]
fn audit_publication_preserves_committed_event_delivery() {
    let runtime = tokio::runtime::Runtime::new().expect("start the property runtime");
    let (pool, notify) = runtime.block_on(async {
        let url = common::require_test_database_url()
            .expect("audit publication properties require an isolated test database");
        let pool = database::connect(&url)
            .await
            .expect("connect the test database");
        let notify = AuditNotify::for_pool(&pool);
        (pool, notify)
    });
    let coverage = PublicationCoverage::default();
    common::property_evidence::run_property(PROPERTY, CASES, &scenario(), |scenario| {
        runtime.block_on(run_case(&pool, &notify, &coverage, scenario))
    });
    println!("audit-publication-coverage: {coverage:?}");
    coverage.assert_reached();
}
