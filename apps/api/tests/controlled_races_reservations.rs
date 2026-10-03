//! Controlled races over event reservations: the last seat, and a seat assignment against the
//! participant's withdrawal.
//!
//! **Role:** plays each reservation race once in each interleaving through the HTTP router and
//! holds every answer and the persisted event to the reservation invariants: a contested seat has
//! exactly one holder, the leader of the race; the loser gets the coded `409` and leaves no row;
//! an assignment and a withdrawal of one participant serialise, both succeed, keep the signup
//! identity and its history, and end in the state of whichever committed last; seats and places
//! stay within capacity; every committed change carries exactly one audit row queued for
//! publication.
//! **Position:** its own test binary; the event, its attachments, seats and actors come from
//! `tests/reservation_guard_support`, the ordering from `tests/failpoint_and_race_support` (a
//! pause at `ReservationClaimBeforeCommit` in `api_operations/src/handlers/slot_registration.rs`, and a
//! held event row lock the contenders queue behind, observed through `pg_blocking_pids`).
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case takes first; each order builds a fresh fixture and closes its pool at the end.
//! **Invariants:** the leader of an order holds the event lock before the follower starts, so the
//! leader always commits first; every wait is bounded and panics naming what it waited for.

mod common;
mod failpoint_and_race_support;
// The binary uses the fixture and its request helpers, not every seeding helper the reservation
// guard suite needs; rustc judges each binary on its own and the gate runs clippy with
// `-D warnings`.
#[allow(dead_code)]
mod reservation_guard_support;
mod telemetry_support;

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use failpoint_and_race_support::{
    BLOCKED_WAIT_BOUND, Failpoint, FailpointArming, FailpointSuiteLock, Interleaving,
    RowLockHolder, audit_evidence, check_event_seats_and_places, lock_suite, run_in_both_orders,
    wait_for_blocked,
};
use reservation_guard_support::Fixture;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// One HTTP answer: status and JSON body.
type Answer = (StatusCode, Value);

/// How long both contenders may take to finish once the leader is released.
const RACE_BOUND: Duration = Duration::from_secs(30);

/// The event row lock every reservation writer takes first.
const EVENT_LOCK: &str = "SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE";

/// The backend of the transaction a paused leader holds open: the one client backend of this
/// database, other than the asking one, that holds a transaction id. A leader holds one once it
/// has locked or written a row; its `state` column is no witness, since the backend reports
/// `idle in transaction` only after the client may already have read its last rows.
async fn paused_leader_backend(pool: &PgPool) -> i32 {
    tokio::time::timeout(BLOCKED_WAIT_BOUND, async {
        loop {
            let backends: Vec<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE datname = current_database()
                 AND backend_type = 'client backend' AND backend_xid IS NOT NULL
                 AND pid <> pg_backend_pid()",
            )
            .fetch_all(pool)
            .await
            .expect("read the open writing transactions");
            if let [leader] = backends.as_slice() {
                return *leader;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!("the paused leader must be the one open writing transaction within {BLOCKED_WAIT_BOUND:?}")
    })
}

/// Both contenders' answers, leader first, within [`RACE_BOUND`].
async fn finish(leading: JoinHandle<Answer>, following: JoinHandle<Answer>) -> (Answer, Answer) {
    tokio::time::timeout(RACE_BOUND, async {
        (
            leading.await.expect("the leading request completes"),
            following.await.expect("the following request completes"),
        )
    })
    .await
    .expect("both contenders finish once the leader is released")
}

/// Unreleased participant allocations `account` holds in `event`.
async fn active_allocations(pool: &PgPool, event: Uuid, account: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM event_participant_allocations
         WHERE event_id = $1 AND discord_id = $2 AND released_at IS NULL",
    )
    .bind(event)
    .bind(account)
    .fetch_one(pool)
    .await
    .expect("read the account's allocations")
}

/// The account seated in `seat`, if any.
async fn seat_occupant(pool: &PgPool, seat: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1")
        .bind(seat)
        .fetch_one(pool)
        .await
        .expect("read the seat")
}

/// Rows of the signup history of `registration`.
async fn history_rows(pool: &PgPool, registration: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM event_registration_history WHERE registration_id = $1")
        .bind(registration)
        .fetch_one(pool)
        .await
        .expect("read the signup history")
}

/// `action` on `target` committed exactly one audit row, and that row is queued for publication
/// or published.
async fn assert_audited_once(pool: &PgPool, action: &str, target: Uuid) {
    let evidence = audit_evidence(pool, action, &target.to_string()).await;
    assert_eq!(evidence.rows, 1, "{action} on {target}: {evidence:?}");
    assert_eq!(
        evidence.pending + evidence.published,
        evidence.rows,
        "every committed {action} row is queued or published: {evidence:?}"
    );
}

/// A claim of seat 0 of attachment 0 by player `claimant`.
fn spawn_seat_claim(fixture: &Arc<Fixture>, claimant: usize) -> JoinHandle<Answer> {
    let fixture = Arc::clone(fixture);
    tokio::spawn(async move {
        fixture
            .register(&fixture.players[claimant], 0, Some(0))
            .await
    })
}

/// Two players claim the one seat of an event with one place; the leader's claim pauses before
/// its commit while the follower queues behind its event lock. Answers the winning player.
async fn last_seat_race(suite: &FailpointSuiteLock, order: Interleaving) -> usize {
    let fixture = Arc::new(Fixture::new(1, 1).await);
    let pool = fixture.state.pool.clone();
    let seat = fixture.slots[0][0];
    let (leader, follower) = order.arrange(0, 1);

    let paused = suite.pause(Failpoint::ReservationClaimBeforeCommit);
    let leading = spawn_seat_claim(&fixture, leader);
    paused.reached().await;
    let holder = paused_leader_backend(&pool).await;
    let following = spawn_seat_claim(&fixture, follower);
    wait_for_blocked(&pool, holder, 1).await;
    paused.release();
    let (leading, following) = finish(leading, following).await;
    assert_eq!(
        paused.arrivals(),
        1,
        "the refused claim never reaches the commit boundary"
    );
    drop(paused);

    // Every observed answer: the leader holds the seat, the follower is refused with its code.
    let (status, body) = &leading;
    assert_eq!(*status, StatusCode::OK, "{}: leader {body}", order.name());
    assert_eq!(body["reservation_state"], "registered", "leader {body}");
    assert_eq!(body["slot_id"], seat.to_string(), "leader {body}");
    let (status, body) = &following;
    assert_eq!(
        *status,
        StatusCode::CONFLICT,
        "{}: follower {body}",
        order.name()
    );
    assert_eq!(body["error"], "slot already taken", "follower {body}");
    assert_eq!(body["details"]["code"], "SEAT_TAKEN", "follower {body}");

    // The persisted event: one holder, capacity kept, the loser left nothing behind.
    check_event_seats_and_places(&pool, fixture.event)
        .await
        .unwrap();
    let winner = &fixture.players[leader];
    let loser = &fixture.players[follower];
    assert_eq!(
        seat_occupant(&pool, seat).await.as_deref(),
        Some(winner.id.as_str())
    );
    let stored = fixture
        .registration(winner, 0)
        .await
        .expect("the winner is registered");
    assert_eq!((stored.1.as_str(), stored.2), ("registered", Some(seat)));
    assert_eq!(
        active_allocations(&pool, fixture.event, &winner.id).await,
        1
    );
    assert!(fixture.registration(loser, 0).await.is_none());
    assert!(fixture.occupants(loser, 0).await.is_empty());
    assert_eq!(active_allocations(&pool, fixture.event, &loser.id).await, 0);
    assert_audited_once(&pool, "event.registration_changed", fixture.missions[0]).await;
    pool.close().await;
    leader
}

#[tokio::test]
async fn controlled_races_last_seat_has_one_winner_in_both_orders() {
    let suite = lock_suite().await;
    let winners = run_in_both_orders(|order| last_seat_race(&suite, order)).await;
    assert_eq!(winners, [0, 1], "the leading claimant wins in each order");
}

/// The two writers of one participant's reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservationWriter {
    /// An admin moves the participant from seat 0 to seat 1.
    Assignment,
    /// The participant withdraws from the attachment.
    Withdrawal,
}

fn spawn_reservation_writer(
    fixture: &Arc<Fixture>,
    writer: ReservationWriter,
) -> JoinHandle<Answer> {
    let fixture = Arc::clone(fixture);
    tokio::spawn(async move {
        let participant = &fixture.players[0];
        match writer {
            ReservationWriter::Assignment => {
                fixture.assign(&fixture.admin, 0, 1, participant).await
            }
            ReservationWriter::Withdrawal => fixture.withdraw(participant, 0).await,
        }
    })
}

/// Asserts the answer of `writer`: both writers succeed whichever order they commit in.
fn assert_writer_answer(writer: ReservationWriter, answer: &Answer, participant: &str) {
    let (status, body) = answer;
    assert_eq!(*status, StatusCode::OK, "{writer:?}: {body}");
    let expected = match writer {
        ReservationWriter::Assignment => json!({ "assigned_to": participant }),
        ReservationWriter::Withdrawal => json!({ "withdrawn": true }),
    };
    assert_eq!(*body, expected, "{writer:?}");
}

/// A seated participant withdraws while an admin moves them to another seat; both queue behind
/// a held event lock in the order's sequence. Answers the final reservation state.
async fn assignment_withdrawal_race(order: Interleaving) -> String {
    let fixture = Arc::new(Fixture::new(0, 2).await);
    let pool = fixture.state.pool.clone();
    let participant = &fixture.players[0];
    fixture
        .seed_registration(participant, 0, "registered", Some(0))
        .await;
    let signup = fixture
        .registration(participant, 0)
        .await
        .expect("the seeded signup")
        .0;
    let (leader, follower) =
        order.arrange(ReservationWriter::Assignment, ReservationWriter::Withdrawal);

    let holder = RowLockHolder::acquire(&pool, EVENT_LOCK, fixture.event).await;
    let leading = spawn_reservation_writer(&fixture, leader);
    holder.wait_for_blocked(&pool, 1).await;
    let following = spawn_reservation_writer(&fixture, follower);
    holder.wait_for_blocked(&pool, 2).await;
    holder.release().await;
    let (leading, following) = finish(leading, following).await;

    // Every observed answer.
    assert_writer_answer(leader, &leading, &participant.id);
    assert_writer_answer(follower, &following, &participant.id);

    // The persisted event: capacity kept, one signup with every transition in its history, and
    // the state of the writer that committed last.
    check_event_seats_and_places(&pool, fixture.event)
        .await
        .unwrap();
    let (stored, state, seat) = fixture
        .registration(participant, 0)
        .await
        .expect("a withdrawal never deletes the signup");
    assert_eq!(stored, signup, "the signup keeps its identity");
    assert_eq!(
        history_rows(&pool, signup).await,
        3,
        "the seeded signup and both serialised transitions stay in the history"
    );
    assert_eq!(seat_occupant(&pool, fixture.slots[0][0]).await, None);
    let occupied = fixture.occupants(participant, 0).await;
    let allocations = active_allocations(&pool, fixture.event, &participant.id).await;
    match follower {
        ReservationWriter::Withdrawal => {
            assert_eq!((state.as_str(), seat), ("withdrawn", None));
            assert!(occupied.is_empty(), "a withdrawn participant holds no seat");
            assert_eq!(allocations, 0, "a withdrawn participant holds no place");
        }
        ReservationWriter::Assignment => {
            assert_eq!(
                (state.as_str(), seat),
                ("registered", Some(fixture.slots[0][1]))
            );
            assert_eq!(occupied, vec![fixture.slots[0][1]]);
            assert_eq!(allocations, 1, "the reassigned participant holds one place");
        }
    }
    assert_audited_once(&pool, "event.slot_assigned", fixture.slots[0][1]).await;
    assert_audited_once(&pool, "event.registration_withdrawn", fixture.missions[0]).await;
    pool.close().await;
    state
}

#[tokio::test]
async fn controlled_races_assignment_and_withdrawal_serialise_in_both_orders() {
    let _suite = lock_suite().await;
    let finals = run_in_both_orders(assignment_withdrawal_race).await;
    assert_eq!(
        finals,
        ["withdrawn", "registered"],
        "the writer that commits last decides the reservation"
    );
}
