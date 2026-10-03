//! Generated claim, release, assign, withdraw and promote operations hold the production
//! reservation transactions to seat and participant conservation.
//!
//! **Role:** runs the property `reservation_transactions_conserve_slots_and_participants`. Every
//! case builds a fresh open event with a generated participant cap and one or two attached
//! missions of generated seat counts, then drives a generated operation sequence through the
//! production-built router: self-service register and withdraw, administrator seat assignment
//! and clearance, and administrator waitlist promotion. The persisted reservation state is read
//! after every operation and held to the invariants below.
//! **Position:** exercises `api_operations::handlers::{slot_registration, slot_assignment,
//! waitlist_promotion}` and the `event_reservations` services beneath them against this binary's
//! private database. Actors are verified members seeded through `events_support::seed_member`;
//! their sessions come from the production `issue_session`.
//! **Signals & state:** one runtime, application state, router and administrator serve every
//! case; each case owns fresh participants and a fresh event, so cases share no reservation row.
//! Every request arrives from a fresh synthetic peer, so the per-address rate limiter never
//! answers in place of the transactions under test.
//! **Invariants (checked after every operation):**
//! - capacity: active allocations never exceed a nonzero participant cap; an attachment's active
//!   participants never exceed its seats; its occupied seats never exceed the cap;
//! - one seat: a participant occupies at most one seat of an attachment, which is one seat per
//!   event on single-attachment events (a participant may attend each attachment of an event);
//! - conservation: occupied seats and seated active reservations are the same (seat, participant)
//!   pairs; active allocation holders are exactly the participants with an active reservation;
//!   each active reservation names its holder's active allocation; waiting and withdrawn rows
//!   hold neither seat nor allocation; registration rows are never removed;
//! - outcomes: every answer is 200, 404 or 409; a refusal changes nothing; a success leaves its
//!   subject in the state the operation names and changes other participants only by waitlist
//!   promotion (and not at all after a claim or an assignment).

mod common;
mod events_support;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use api::router::router;
use api_configuration::configuration::Config;
use api_identity_and_access::services::session_issuance::issue_session;
use api_state::AppState;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const PROPERTY: &str = "reservation_transactions_conserve_slots_and_participants";
const CASES: u32 = 256;
/// Participants per case; more than the largest generated cap, so capacity binds.
const PLAYERS: usize = 4;
const REGISTERED: &str = "registered";
const WAITLISTED: &str = "waitlisted";
const WITHDRAWN: &str = "withdrawn";

/// One generated reservation operation. Indexes address the case's players, attachments and
/// the chosen attachment's seats.
#[derive(Debug, Clone, Copy)]
enum Operation {
    /// The player registers for the attachment, for the seat or seatless.
    Claim {
        player: usize,
        attachment: usize,
        seat: Option<usize>,
    },
    /// The player withdraws from the attachment.
    Withdraw { player: usize, attachment: usize },
    /// The administrator assigns the player to the seat.
    Assign {
        player: usize,
        attachment: usize,
        seat: usize,
    },
    /// The administrator clears the seat.
    Release { attachment: usize, seat: usize },
    /// The administrator asks for the attachment's waitlist promotion.
    Promote { attachment: usize },
}

impl Operation {
    fn kind(&self) -> &'static str {
        match self {
            Operation::Claim { .. } => "claim",
            Operation::Withdraw { .. } => "withdraw",
            Operation::Assign { .. } => "assign",
            Operation::Release { .. } => "release",
            Operation::Promote { .. } => "promote",
        }
    }
}

/// How often each outcome label (operation, status, refusal code or granted reservation state)
/// occurred across the run.
type OutcomeTally = RefCell<BTreeMap<String, u32>>;

/// Outcomes every run must reach, so a generator or fixture that stops reaching one fails the
/// run instead of passing on fewer paths. A manual promotion is never granted here: every
/// transaction that frees a place promotes waiting participants itself, which the `promoted`
/// labels count.
const REQUIRED_OUTCOMES: &[&str] = &[
    "claim 200 registered",
    "claim 200 waitlisted",
    "claim 409 EVENT_FULL",
    "claim 409 MISSION_FULL",
    "claim 409 SEAT_TAKEN",
    "assign 200",
    "assign 409 EVENT_FULL",
    "assign 409 SEAT_TAKEN",
    "release 200",
    "withdraw 200",
    "withdraw 404",
    "withdraw promoted",
    "promote 404",
    "promote 409 EVENT_FULL",
];

fn outcome_label(operation: Operation, status: StatusCode, body: &Value) -> String {
    let detail = body["details"]["code"]
        .as_str()
        .or_else(|| body["reservation_state"].as_str())
        .unwrap_or_default();
    format!("{} {} {detail}", operation.kind(), status.as_u16())
        .trim_end()
        .to_owned()
}

/// One generated case: the event's participant cap (0 is uncapped), the seat count of each
/// attachment, and the operations in order.
#[derive(Debug, Clone)]
struct ReservationScenario {
    participant_cap: i64,
    seats: Vec<usize>,
    operations: Vec<Operation>,
}

fn operation(seats: Vec<usize>) -> impl Strategy<Value = Operation> {
    (0..seats.len()).prop_flat_map(move |attachment| {
        let seat_count = seats[attachment];
        prop_oneof![
            4 => (0..PLAYERS, proptest::option::of(0..seat_count)).prop_map(
                move |(player, seat)| Operation::Claim { player, attachment, seat }
            ),
            2 => (0..PLAYERS).prop_map(move |player| Operation::Withdraw { player, attachment }),
            2 => (0..PLAYERS, 0..seat_count).prop_map(
                move |(player, seat)| Operation::Assign { player, attachment, seat }
            ),
            1 => (0..seat_count).prop_map(move |seat| Operation::Release { attachment, seat }),
            1 => Just(Operation::Promote { attachment }),
        ]
    })
}

fn scenario() -> impl Strategy<Value = ReservationScenario> {
    (
        prop_oneof![1 => Just(0i64), 4 => 1i64..=3],
        vec(1usize..=3, 1..=2),
    )
        .prop_flat_map(|(participant_cap, seats)| {
            let operations = vec(operation(seats.clone()), 1..=12);
            (Just(participant_cap), Just(seats), operations).prop_map(
                |(participant_cap, seats, operations)| ReservationScenario {
                    participant_cap,
                    seats,
                    operations,
                },
            )
        })
}

/// A seeded account and its bearer.
struct Actor {
    id: String,
    token: String,
}

/// What every case shares: the application state, its router and the administrator who
/// manages seats and promotions.
struct PropertyWorld {
    state: AppState,
    app: Router,
    administrator: Actor,
}

/// One case's event, its attachments with their seats, and its participants.
struct CaseEvent {
    event: Uuid,
    participant_cap: i64,
    attachments: Vec<Uuid>,
    seats: Vec<Vec<Uuid>>,
    players: Vec<Actor>,
}

impl CaseEvent {
    fn attachment_index(&self, attachment: Uuid) -> usize {
        self.attachments
            .iter()
            .position(|candidate| *candidate == attachment)
            .expect("a registration of this case names one of its attachments")
    }
}

/// A verified member with a live production session.
async fn actor(state: &AppState, role: &str) -> Actor {
    let id = format!("reservation-property-{role}-{}", Uuid::new_v4());
    events_support::seed_member(
        &state.pool,
        &id,
        "Reservation property actor",
        &events_support::arma(&id),
        role,
    )
    .await;
    let token = issue_session(state, &api_identifiers::DiscordUserId::new(id.as_str()))
        .await
        .unwrap_or_else(|error| panic!("issue a session for {id}: {error:?}"))
        .0;
    Actor { id, token }
}

async fn create_case(world: &PropertyWorld, scenario: &ReservationScenario) -> CaseEvent {
    let pool = &world.state.pool;
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, max_slots, created_by)
         VALUES ('Reservation property event', clock_timestamp() + interval '3 days', 'open', $1, $2)
         RETURNING id",
    )
    .bind(scenario.participant_cap)
    .bind(&world.administrator.id)
    .fetch_one(pool)
    .await
    .expect("create the case event");
    let mut attachments = Vec::new();
    let mut seats = Vec::new();
    for &seat_count in &scenario.seats {
        let mission: Uuid = sqlx::query_scalar(
            "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
             VALUES ('Reservation property mission', $1, 'everon', 'pve_coop', 32, 'live')
             RETURNING id",
        )
        .bind(&world.administrator.id)
        .fetch_one(pool)
        .await
        .expect("create a case mission");
        let attachment: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions(event_id, mission_id, start_time)
             VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
        )
        .bind(event)
        .bind(mission)
        .fetch_one(pool)
        .await
        .expect("attach the mission to the event");
        let mut attachment_seats = Vec::new();
        for index in 0..seat_count {
            attachment_seats.push(
                sqlx::query_scalar(
                    "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index)
                     VALUES ($1, 'USA', 'Alpha', 'Rifleman', $2) RETURNING id",
                )
                .bind(attachment)
                .bind(i32::try_from(index).expect("a small seat index"))
                .fetch_one(pool)
                .await
                .expect("create a seat"),
            );
        }
        attachments.push(attachment);
        seats.push(attachment_seats);
    }
    let mut players = Vec::new();
    for _ in 0..PLAYERS {
        players.push(actor(&world.state, "enlisted").await);
    }
    CaseEvent {
        event,
        participant_cap: scenario.participant_cap,
        attachments,
        seats,
        players,
    }
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// A distinct synthetic client address for every request.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let mut request = builder
        .body(body.map_or(Body::empty(), |body| Body::from(body.to_string())))
        .expect("build the request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the response body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Send `operation` as its production route; returns the answer and the account it concerns
/// (`None` for a promotion, which chooses its own participants).
async fn perform(
    world: &PropertyWorld,
    case: &CaseEvent,
    before: &ReservationSnapshot,
    operation: Operation,
) -> (StatusCode, Value, Option<String>) {
    let administrator = &world.administrator.token;
    let (method, uri, token, body, subject) = match operation {
        Operation::Claim {
            player,
            attachment,
            seat,
        } => (
            "POST",
            format!(
                "/api/v1/event-missions/{}/register",
                case.attachments[attachment]
            ),
            &case.players[player].token,
            Some(json!({"slot_id": seat
                .map(|seat| case.seats[attachment][seat].to_string())
                .unwrap_or_default()})),
            Some(case.players[player].id.clone()),
        ),
        Operation::Withdraw { player, attachment } => (
            "DELETE",
            format!(
                "/api/v1/event-missions/{}/register",
                case.attachments[attachment]
            ),
            &case.players[player].token,
            None,
            Some(case.players[player].id.clone()),
        ),
        Operation::Assign {
            player,
            attachment,
            seat,
        } => (
            "PUT",
            seat_uri(case, attachment, seat),
            administrator,
            Some(json!({"discord_id": case.players[player].id})),
            Some(case.players[player].id.clone()),
        ),
        Operation::Release { attachment, seat } => (
            "DELETE",
            seat_uri(case, attachment, seat),
            administrator,
            None,
            before.seats[&case.seats[attachment][seat]].clone(),
        ),
        Operation::Promote { attachment } => (
            "POST",
            format!(
                "/api/v1/event-missions/{}/waitlist/promote",
                case.attachments[attachment]
            ),
            administrator,
            None,
            None,
        ),
    };
    let (status, answer) = send(&world.app, method, &uri, token, body).await;
    (status, answer, subject)
}

fn seat_uri(case: &CaseEvent, attachment: usize, seat: usize) -> String {
    format!(
        "/api/v1/event-missions/{}/slots/{}/assign",
        case.attachments[attachment], case.seats[attachment][seat]
    )
}

/// One registration row of the case.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reservation {
    state: String,
    seat: Option<Uuid>,
    allocation: Option<Uuid>,
}

impl Reservation {
    fn is_active(&self) -> bool {
        matches!(self.state.as_str(), REGISTERED | "legacy_unknown")
    }
}

/// One stored registration: attachment, participant, reservation state, seat, allocation.
type RegistrationRow = (Uuid, String, String, Option<Uuid>, Option<Uuid>);

/// Everything a snapshot records about one participant: its registrations by attachment, the
/// seats it occupies, and its active allocation.
type ParticipantProjection = (Vec<(Uuid, Reservation)>, BTreeSet<Uuid>, Option<Uuid>);

/// The persisted reservation state of one case event.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReservationSnapshot {
    /// The occupant of every seat of the event, by seat id.
    seats: BTreeMap<Uuid, Option<String>>,
    /// Every registration row, by (attachment, participant).
    reservations: BTreeMap<(Uuid, String), Reservation>,
    /// The number of registration rows, which duplicate keys would make exceed the map's size.
    registration_rows: usize,
    /// Every active allocation as (participant, allocation id), sorted.
    allocations: Vec<(String, Uuid)>,
}

impl ReservationSnapshot {
    async fn read(pool: &PgPool, case: &CaseEvent) -> Self {
        let seats: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, assigned_to FROM orbat_slots WHERE event_mission_id = ANY($1)",
        )
        .bind(&case.attachments)
        .fetch_all(pool)
        .await
        .expect("read the case seats");
        let rows: Vec<RegistrationRow> = sqlx::query_as(
            "SELECT event_mission_id, discord_id, reservation_state::text, slot_id, allocation_id
             FROM event_registrations WHERE event_mission_id = ANY($1)",
        )
        .bind(&case.attachments)
        .fetch_all(pool)
        .await
        .expect("read the case registrations");
        let allocations: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT discord_id, id FROM event_participant_allocations
             WHERE event_id = $1 AND released_at IS NULL ORDER BY discord_id, id",
        )
        .bind(case.event)
        .fetch_all(pool)
        .await
        .expect("read the case allocations");
        let registration_rows = rows.len();
        let reservations = rows
            .into_iter()
            .map(|(attachment, account, state, seat, allocation)| {
                (
                    (attachment, account),
                    Reservation {
                        state,
                        seat,
                        allocation,
                    },
                )
            })
            .collect();
        Self {
            seats: seats.into_iter().collect(),
            reservations,
            registration_rows,
            allocations,
        }
    }

    fn reservation(&self, attachment: Uuid, account: &str) -> Option<&Reservation> {
        self.reservations.get(&(attachment, account.to_owned()))
    }

    fn allocation_of(&self, account: &str) -> Option<Uuid> {
        self.allocations
            .iter()
            .find(|(holder, _)| holder == account)
            .map(|(_, allocation)| *allocation)
    }

    fn seats_of(&self, account: &str) -> BTreeSet<Uuid> {
        self.seats
            .iter()
            .filter(|(_, occupant)| occupant.as_deref() == Some(account))
            .map(|(seat, _)| *seat)
            .collect()
    }

    fn accounts(&self) -> BTreeSet<String> {
        let reserved = self.reservations.keys().map(|(_, account)| account.clone());
        let seated = self.seats.values().flatten().cloned();
        let allocated = self.allocations.iter().map(|(account, _)| account.clone());
        reserved.chain(seated).chain(allocated).collect()
    }

    /// Everything the snapshot records about one account.
    fn projection(&self, account: &str) -> ParticipantProjection {
        let reservations = self
            .reservations
            .iter()
            .filter(|((_, holder), _)| holder == account)
            .map(|((attachment, _), reservation)| (*attachment, reservation.clone()))
            .collect();
        (
            reservations,
            self.seats_of(account),
            self.allocation_of(account),
        )
    }

    /// `account` changed from `before` to `self` only by being promoted off a waitlist: every
    /// changed row went from waiting to registered, no seat was lost, no allocation replaced.
    fn changed_only_by_promotion(&self, before: &Self, account: &str) -> bool {
        let rows_promoted = before
            .reservations
            .keys()
            .chain(self.reservations.keys())
            .filter(|(_, holder)| holder == account)
            .all(|(attachment, _)| {
                match (
                    before.reservation(*attachment, account),
                    self.reservation(*attachment, account),
                ) {
                    (previous, current) if previous == current => true,
                    (Some(previous), Some(current)) => {
                        previous.state == WAITLISTED && current.state == REGISTERED
                    }
                    _ => false,
                }
            });
        let seats_kept = before.seats_of(account).is_subset(&self.seats_of(account));
        let allocation_kept = before
            .allocation_of(account)
            .is_none_or(|allocation| self.allocation_of(account) == Some(allocation));
        rows_promoted && seats_kept && allocation_kept
    }
}

/// Capacity, seat uniqueness and conservation over one persisted state.
fn check_invariants(case: &CaseEvent, state: &ReservationSnapshot) -> TestCaseResult {
    let holders: BTreeSet<&str> = state
        .allocations
        .iter()
        .map(|(account, _)| account.as_str())
        .collect();
    prop_assert_eq!(
        holders.len(),
        state.allocations.len(),
        "one active allocation per participant"
    );
    prop_assert_eq!(
        state.registration_rows,
        state.reservations.len(),
        "one registration row per participant and attachment"
    );
    let cap = usize::try_from(case.participant_cap).expect("a generated cap is small");
    if cap > 0 {
        prop_assert!(
            holders.len() <= cap,
            "{} allocations exceed the cap {}",
            holders.len(),
            cap
        );
    }
    for (index, attachment) in case.attachments.iter().enumerate() {
        let seats = &case.seats[index];
        let occupants: Vec<&String> = seats
            .iter()
            .filter_map(|seat| state.seats[seat].as_ref())
            .collect();
        let distinct: BTreeSet<&&String> = occupants.iter().collect();
        prop_assert_eq!(
            distinct.len(),
            occupants.len(),
            "one seat per participant per attachment"
        );
        let capacity = if cap > 0 {
            cap.min(seats.len())
        } else {
            seats.len()
        };
        prop_assert!(
            occupants.len() <= capacity,
            "occupied seats exceed capacity {}",
            capacity
        );
        let participants = state
            .reservations
            .iter()
            .filter(|((holder, _), reservation)| holder == attachment && reservation.is_active())
            .count();
        prop_assert!(
            participants <= seats.len(),
            "{} participants on {} seats",
            participants,
            seats.len()
        );
    }
    let occupied: BTreeSet<(Uuid, String)> = state
        .seats
        .iter()
        .filter_map(|(seat, occupant)| occupant.clone().map(|occupant| (*seat, occupant)))
        .collect();
    let seated: BTreeSet<(Uuid, String)> = state
        .reservations
        .iter()
        .filter(|(_, reservation)| reservation.is_active())
        .filter_map(|((_, account), reservation)| {
            reservation.seat.map(|seat| (seat, account.clone()))
        })
        .collect();
    prop_assert_eq!(
        occupied,
        seated,
        "occupied seats are exactly the seated reservations"
    );
    let mut participants: BTreeSet<&str> = BTreeSet::new();
    for ((attachment, account), reservation) in &state.reservations {
        if let Some(seat) = reservation.seat {
            let seats = &case.seats[case.attachment_index(*attachment)];
            prop_assert!(
                seats.contains(&seat),
                "a reservation names a seat of its attachment"
            );
        }
        if reservation.is_active() {
            participants.insert(account);
            prop_assert_eq!(
                reservation.allocation,
                state.allocation_of(account),
                "an active reservation names its holder's active allocation"
            );
        } else {
            prop_assert_eq!((reservation.seat, reservation.allocation), (None, None));
        }
    }
    prop_assert_eq!(
        holders,
        participants,
        "allocation holders are the active participants"
    );
    Ok(())
}

/// The answer and the state change of one operation, against what the operation promises;
/// returns the participants the operation promoted off a waitlist.
fn check_outcome(
    case: &CaseEvent,
    operation: Operation,
    answer: (StatusCode, &Value, Option<&str>),
    before: &ReservationSnapshot,
    after: &ReservationSnapshot,
) -> Result<BTreeSet<String>, TestCaseError> {
    let (status, body, subject) = answer;
    prop_assert!(
        matches!(status.as_u16(), 200 | 404 | 409),
        "{:?} answered {} {}",
        operation,
        status,
        body
    );
    for key in before.reservations.keys() {
        prop_assert!(
            after.reservations.contains_key(key),
            "registration rows are never removed"
        );
    }
    if status != StatusCode::OK {
        prop_assert!(
            body["error"].is_string(),
            "a refusal carries the error envelope: {}",
            body
        );
        prop_assert_eq!(before, after, "a refused {:?} changes nothing", operation);
        return Ok(BTreeSet::new());
    }
    let seat_id = |attachment: usize, seat: usize| case.seats[attachment][seat];
    let promoted: BTreeSet<String> = match operation {
        Operation::Claim {
            attachment, seat, ..
        } => {
            let account = subject.expect("a claim has a claimant");
            let current = after.reservation(case.attachments[attachment], account);
            prop_assert!(current.is_some(), "a granted claim records a registration");
            let current = current.expect("checked above");
            match seat {
                Some(seat) => {
                    prop_assert_eq!(current.state.as_str(), REGISTERED);
                    prop_assert_eq!(current.seat, Some(seat_id(attachment, seat)));
                }
                None => prop_assert!(matches!(current.state.as_str(), REGISTERED | WAITLISTED)),
            }
            prop_assert_eq!(
                body["reservation_state"].as_str(),
                Some(current.state.as_str())
            );
            prop_assert_eq!(
                body["slot_id"].as_str().map(str::to_owned),
                current.seat.map(|seat| seat.to_string())
            );
            BTreeSet::new()
        }
        Operation::Assign {
            attachment, seat, ..
        } => {
            let account = subject.expect("an assignment has an assignee");
            let current = after.reservation(case.attachments[attachment], account);
            prop_assert_eq!(
                after.seats[&seat_id(attachment, seat)].as_deref(),
                Some(account)
            );
            prop_assert_eq!(
                current.map(|current| current.state.as_str()),
                Some(REGISTERED)
            );
            prop_assert_eq!(
                current.and_then(|current| current.seat),
                Some(seat_id(attachment, seat))
            );
            prop_assert_eq!(body["assigned_to"].as_str(), Some(account));
            BTreeSet::new()
        }
        Operation::Withdraw { attachment, .. } => {
            let account = subject.expect("a withdrawal has a participant");
            let current = after.reservation(case.attachments[attachment], account);
            prop_assert!(current.is_none_or(|current| current.state == WITHDRAWN));
            prop_assert!(
                case.seats[attachment]
                    .iter()
                    .all(|seat| after.seats[seat].as_deref() != Some(account))
            );
            prop_assert_eq!(body, &json!({"withdrawn": true}));
            promoted_bystanders(before, after, subject)?
        }
        Operation::Release { attachment, seat } => {
            let seat = seat_id(attachment, seat);
            if let Some(account) = subject {
                prop_assert_ne!(after.seats[&seat].as_deref(), Some(account));
                let previous = before.reservation(case.attachments[attachment], account);
                if previous.is_some_and(Reservation::is_active) {
                    let current = after.reservation(case.attachments[attachment], account);
                    prop_assert_eq!(
                        current.map(|current| (current.state.as_str(), current.seat)),
                        Some((REGISTERED, None))
                    );
                }
            }
            prop_assert_eq!(body, &json!({"cleared": true}));
            promoted_bystanders(before, after, subject)?
        }
        Operation::Promote { attachment } => {
            let entries = body["promoted"].as_array().cloned().unwrap_or_default();
            prop_assert!(
                !entries.is_empty(),
                "a granted promotion names who moved: {}",
                body
            );
            let mut promoted = BTreeSet::new();
            for entry in entries {
                let account = entry["discord_id"].as_str().unwrap_or_default().to_owned();
                let previous = before.reservation(case.attachments[attachment], &account);
                let current = after.reservation(case.attachments[attachment], &account);
                prop_assert_eq!(
                    previous.map(|previous| previous.state.as_str()),
                    Some(WAITLISTED)
                );
                prop_assert_eq!(
                    current.map(|current| current.state.as_str()),
                    Some(REGISTERED)
                );
                prop_assert_eq!(
                    current
                        .and_then(|current| current.seat)
                        .map(|seat| seat.to_string()),
                    entry["slot_id"].as_str().map(str::to_owned)
                );
                promoted.insert(account);
            }
            let moved = promoted_bystanders(before, after, None)?;
            prop_assert_eq!(
                &moved,
                &promoted,
                "a promotion changes exactly the promoted"
            );
            promoted
        }
    };
    // Claims and assignments change no one but their subject.
    if matches!(
        operation,
        Operation::Claim { .. } | Operation::Assign { .. }
    ) {
        prop_assert!(promoted.is_empty());
        for account in before.accounts().union(&after.accounts()) {
            if Some(account.as_str()) != subject {
                prop_assert_eq!(before.projection(account), after.projection(account));
            }
        }
    }
    Ok(promoted)
}

/// Every account but `subject` whose rows changed, each checked to have changed only by
/// promotion.
fn promoted_bystanders(
    before: &ReservationSnapshot,
    after: &ReservationSnapshot,
    subject: Option<&str>,
) -> Result<BTreeSet<String>, TestCaseError> {
    let mut changed = BTreeSet::new();
    for account in before.accounts().union(&after.accounts()) {
        if Some(account.as_str()) == subject
            || before.projection(account) == after.projection(account)
        {
            continue;
        }
        prop_assert!(
            after.changed_only_by_promotion(before, account),
            "{} changed other than by promotion",
            account
        );
        changed.insert(account.clone());
    }
    Ok(changed)
}

async fn run_case(
    world: &PropertyWorld,
    tally: &OutcomeTally,
    scenario: ReservationScenario,
) -> TestCaseResult {
    let case = create_case(world, &scenario).await;
    let mut before = ReservationSnapshot::read(&world.state.pool, &case).await;
    check_invariants(&case, &before)?;
    for operation in scenario.operations {
        let (status, body, subject) = perform(world, &case, &before, operation).await;
        let label = outcome_label(operation, status, &body);
        let after = ReservationSnapshot::read(&world.state.pool, &case).await;
        let promoted = check_outcome(
            &case,
            operation,
            (status, &body, subject.as_deref()),
            &before,
            &after,
        )?;
        check_invariants(&case, &after)?;
        let mut tally = tally.borrow_mut();
        *tally.entry(label).or_default() += 1;
        if !promoted.is_empty() {
            *tally
                .entry(format!("{} promoted", operation.kind()))
                .or_default() += 1;
        }
        before = after;
    }
    Ok(())
}

#[test]
fn reservation_transactions_conserve_slots_and_participants() {
    let runtime = tokio::runtime::Runtime::new().expect("start the property runtime");
    let world = runtime.block_on(async {
        let url = common::require_test_database_url()
            .expect("reservation properties require an isolated test database");
        let pool = api_database::connect(&url)
            .await
            .expect("connect the test database");
        let state = api::composition::application_state(
            pool,
            Config::for_tests(url, "reservation-properties"),
        );
        let administrator = actor(&state, "admin").await;
        PropertyWorld {
            app: router(state.clone()),
            state,
            administrator,
        }
    });
    let tally = OutcomeTally::default();
    api_property_evidence::run_property(PROPERTY, CASES, &scenario(), |scenario| {
        runtime.block_on(run_case(&world, &tally, scenario))
    });
    let tally = tally.into_inner();
    println!("reservation-outcomes: {tally:?}");
    for outcome in REQUIRED_OUTCOMES {
        assert!(
            tally.contains_key(*outcome),
            "no generated case reached `{outcome}`: {tally:?}"
        );
    }
}
