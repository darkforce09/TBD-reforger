//! Attendance derives from finalized facts: a reservation active when its exact match
//! was first finalized, and absent from that match's results, is a no-show; late corrections
//! re-derive attendance in both directions; an unlinked result player keeps a result row that
//! reconciles when the identity links; a withdrawal leaves a timestamped tombstone whose seat is
//! reusable. Scheduled time alone never decides attendance.

use axum::http::StatusCode;
use serde_json::{Value, json};
use website_api::operations::services::event_lifecycle_sweep::sweep_once;

mod common;
mod event_eligibility_support;

use event_eligibility_support::{Actor, EventShape, Fixture};

const SUITE: &str = "attendance_no_show_derivation";

fn arma(actor: &Actor) -> String {
    format!("test-arma:{}", actor.id)
}

/// Report `present` Arma identities for `source` against attachment `mission`.
async fn report(
    f: &Fixture,
    source: &str,
    mission: usize,
    outcome: &str,
    present: &[String],
) -> Value {
    let players: Vec<Value> = present
        .iter()
        .map(|arma_id| json!({"arma_id": arma_id, "role_played": "Rifleman", "source_event_id": "life-1"}))
        .collect();
    let (status, body) = f
        .service_call(
            "/api/v1/ingest/match-results",
            json!({
                "match": {
                    "source_match_id": source,
                    "outcome": outcome,
                    "event_id": f.event,
                    "mission_id": f.catalog_mission(mission).await,
                    "terrain": "everon"
                },
                "players": players
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// `(reservation_state, attendance_state)` of the actor's registration for `mission`.
async fn attendance(f: &Fixture, actor: &Actor, mission: usize) -> (String, Option<String>) {
    sqlx::query_as(
        "SELECT reservation_state::text, attendance_state::text FROM event_registrations
         WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(f.missions[mission])
    .bind(&actor.id)
    .fetch_one(f.pool())
    .await
    .unwrap()
}

async fn attendance_rate(f: &Fixture, actor: &Actor) -> f64 {
    sqlx::query_scalar("SELECT attendance_rate::float8 FROM users WHERE discord_id = $1")
        .bind(&actor.id)
        .fetch_one(f.pool())
        .await
        .unwrap()
}

fn state(reservation: &str, attended: Option<&str>) -> (String, Option<String>) {
    (reservation.to_owned(), attended.map(str::to_owned))
}

#[tokio::test]
async fn registration_history_finalized_match_derives_no_show_for_absent_registrant() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    let present = f.member("present").await;
    let absent = f.member("absent").await;
    let departed = f.member("departed").await;
    let promoted = f.member("promoted").await;
    let queued = f.member("queued").await;
    for (actor, seat) in [(&present, 0), (&absent, 1), (&departed, 2)] {
        assert_eq!(f.register(actor, 0, Some(seat)).await.0, StatusCode::OK);
    }
    assert_eq!(
        f.register(&promoted, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    assert_eq!(f.withdraw(&departed, 0).await.0, StatusCode::OK);
    assert_eq!(
        attendance(&f, &promoted, 0).await,
        state("registered", None),
        "promoted into the freed seat"
    );
    assert_eq!(
        f.register(&queued, 0, None).await.1["reservation_state"],
        "waitlisted"
    );

    // The scheduled start and end pass, and the lifecycle converges: nobody is marked.
    f.move_schedule_into_past(6).await;
    sweep_once(f.pool()).await.unwrap();
    let unmarked: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT attendance_state::text FROM event_registrations WHERE event_mission_id = $1",
    )
    .bind(f.missions[0])
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert!(
        unmarked.iter().all(Option::is_none),
        "scheduled time alone decides nothing: {unmarked:?}"
    );

    // An unfinished report obliges nobody.
    let source = format!("{}-played", f.event);
    report(&f, &source, 0, "pending", &[arma(&present)]).await;
    assert_eq!(attendance(&f, &absent, 0).await, state("registered", None));
    assert_eq!(attendance(&f, &present, 0).await, state("registered", None));

    // Finalization obliges every reservation active at that instant.
    report(&f, &source, 0, "success", &[arma(&present)]).await;
    assert_eq!(
        attendance(&f, &present, 0).await,
        state("registered", Some("attended"))
    );
    assert_eq!(
        attendance(&f, &absent, 0).await,
        state("registered", Some("no_show"))
    );
    assert_eq!(
        attendance(&f, &promoted, 0).await,
        state("registered", Some("no_show"))
    );
    assert_eq!(
        attendance(&f, &departed, 0).await,
        state("withdrawn", None),
        "withdrew before finalization"
    );
    assert_eq!(
        attendance(&f, &queued, 0).await,
        state("waitlisted", None),
        "never held a place"
    );
    assert_eq!(attendance_rate(&f, &present).await, 100.0);
    assert_eq!(attendance_rate(&f, &absent).await, 0.0);

    // Withdrawing afterwards changes the reservation, never the obligation it had.
    assert_eq!(f.withdraw(&absent, 0).await.0, StatusCode::OK);
    assert_eq!(
        attendance(&f, &absent, 0).await,
        state("withdrawn", Some("no_show"))
    );
    let history: Vec<String> = sqlx::query_scalar(
        "SELECT h.reservation_state::text FROM event_registration_history h
         JOIN event_registrations r ON r.id = h.registration_id
         WHERE r.event_mission_id = $1 AND r.discord_id = $2 ORDER BY h.id",
    )
    .bind(f.missions[0])
    .bind(&absent.id)
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(history.first().map(String::as_str), Some("registered"));
    assert_eq!(history.last().map(String::as_str), Some("withdrawn"));
    f.pool().close().await;
}

#[tokio::test]
async fn registration_history_late_correction_flips_no_show_both_ways() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"], &["Bravo"]],
        },
    )
    .await;
    let (late, steady, other) = (
        f.member("late").await,
        f.member("steady").await,
        f.member("other").await,
    );
    assert_eq!(f.register(&late, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&steady, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(f.register(&other, 1, Some(0)).await.0, StatusCode::OK);
    f.move_schedule_into_past(4).await;
    let source = format!("{}-corrected", f.event);

    // The first final report omits a participant whose line arrives late.
    report(&f, &source, 0, "success", &[arma(&steady)]).await;
    assert_eq!(
        attendance(&f, &late, 0).await,
        state("registered", Some("no_show"))
    );
    report(&f, &source, 0, "success", &[arma(&steady), arma(&late)]).await;
    assert_eq!(
        attendance(&f, &late, 0).await,
        state("registered", Some("attended"))
    );
    assert_eq!(attendance_rate(&f, &late).await, 100.0);

    // A correction that re-points the match to the other attachment retracts the first
    // attachment's participation and obligations, and obliges the other attachment instead.
    report(&f, &source, 1, "success", &[arma(&steady), arma(&late)]).await;
    assert_eq!(attendance(&f, &late, 0).await, state("registered", None));
    assert_eq!(attendance(&f, &steady, 0).await, state("registered", None));
    assert_eq!(
        attendance(&f, &other, 1).await,
        state("registered", Some("no_show"))
    );

    // Pointing it back restores exactly the facts of the first attachment.
    report(&f, &source, 0, "success", &[arma(&steady), arma(&late)]).await;
    assert_eq!(
        attendance(&f, &late, 0).await,
        state("registered", Some("attended"))
    );
    assert_eq!(attendance(&f, &other, 1).await, state("registered", None));

    // An aborted outcome obliges nobody, and outcome corrections re-derive the obligation.
    let aborted = format!("{}-aborted", f.event);
    report(&f, &aborted, 1, "aborted", &[]).await;
    assert_eq!(attendance(&f, &other, 1).await, state("registered", None));
    report(&f, &aborted, 1, "failure", &[]).await;
    assert_eq!(
        attendance(&f, &other, 1).await,
        state("registered", Some("no_show")),
        "a late outcome correction obliges"
    );
    report(&f, &aborted, 1, "aborted", &[]).await;
    assert_eq!(
        attendance(&f, &other, 1).await,
        state("registered", None),
        "and the reverse correction releases"
    );
    f.pool().close().await;
}

#[tokio::test]
async fn registration_history_unlinked_result_player_reconciles_on_link() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let linker = f.unlinked_member("linker").await;
    assert_eq!(f.register(&linker, 0, Some(0)).await.0, StatusCode::OK);
    f.move_schedule_into_past(4).await;
    let identity = format!("late-link-arma:{}", linker.id);
    let body = report(
        &f,
        &format!("{}-unlinked", f.event),
        0,
        "success",
        std::slice::from_ref(&identity),
    )
    .await;
    assert_eq!(
        (body["linked"].clone(), body["unlinked"].clone()),
        (json!(0), json!(1))
    );
    assert_eq!(body["unlinked_arma_ids"], json!([identity]));
    // The unlinked player's result row is retained without an owner.
    let owner: Option<String> =
        sqlx::query_scalar("SELECT discord_id FROM match_player_stats WHERE arma_id = $1")
            .bind(&identity)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(owner, None);
    assert_eq!(
        attendance(&f, &linker, 0).await,
        state("registered", Some("no_show"))
    );

    // Linking claims the retained row and reconciles attendance in the same transaction.
    let (status, issued) = f.call(&linker, "POST", "/api/v1/me/link", None).await;
    assert_eq!(status, StatusCode::CREATED, "{issued}");
    let (status, confirmed) = f
        .service_call(
            "/api/v1/ingest/link-confirm",
            json!({"code": issued["code"], "arma_id": identity, "arma_character": "[TBD] Late Linker"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");
    let owner: Option<String> =
        sqlx::query_scalar("SELECT discord_id FROM match_player_stats WHERE arma_id = $1")
            .bind(&identity)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(owner.as_deref(), Some(linker.id.as_str()));
    assert_eq!(
        attendance(&f, &linker, 0).await,
        state("registered", Some("attended"))
    );
    let (deployments, rate): (i64, f64) = sqlx::query_as(
        "SELECT total_deployments, attendance_rate::float8 FROM users WHERE discord_id = $1",
    )
    .bind(&linker.id)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!((deployments, rate), (1, 100.0));
    f.pool().close().await;
}

#[tokio::test]
async fn registration_history_withdrawn_tombstone_keeps_timestamp_and_frees_seat() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let (leaver, successor) = (f.member("leaver").await, f.member("successor").await);
    assert_eq!(f.register(&leaver, 0, Some(0)).await.0, StatusCode::OK);
    let signed_up = f.registration(&leaver, 0).await.unwrap().0;
    let (status, body) = f.withdraw(&leaver, 0).await;
    assert_eq!((status, body), (StatusCode::OK, json!({"withdrawn": true})));

    let (id, withdrawn_at, slot): (uuid::Uuid, Option<chrono::DateTime<chrono::Utc>>, Option<uuid::Uuid>) =
        sqlx::query_as(
            "SELECT id, withdrawn_at, slot_id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(f.missions[0])
        .bind(&leaver.id)
        .fetch_one(f.pool())
        .await
        .unwrap();
    assert_eq!(id, signed_up, "the signup row remains as a tombstone");
    let withdrawn_at = withdrawn_at.expect("the tombstone records when the place was released");
    assert_eq!(slot, None);
    assert_eq!(f.occupant(0, 0).await, None);

    // The viewer's own dossier lists the withdrawn signup with its timestamp and reason.
    let (status, hub) = f
        .call(&leaver, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{hub}");
    let mine = &hub["missions"][0];
    assert_eq!(mine["my_state"], "withdrawn");
    assert_eq!(mine["my_reservation_state"], "withdrawn");
    assert_eq!(mine["my_release_reason"], "participant_withdrew");
    let listed: chrono::DateTime<chrono::Utc> =
        mine["my_withdrawn_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(listed.timestamp_micros(), withdrawn_at.timestamp_micros());
    assert!(mine.get("my_slot_id").is_none());

    // The seat is reusable, and a later signup by the same participant reuses the tombstone.
    assert_eq!(f.register(&successor, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(
        f.occupant(0, 0).await.as_deref(),
        Some(successor.id.as_str())
    );
    assert_eq!(f.withdraw(&successor, 0).await.0, StatusCode::OK);
    assert_eq!(f.register(&leaver, 0, Some(0)).await.0, StatusCode::OK);
    let returned = f.registration(&leaver, 0).await.unwrap();
    assert_eq!(
        (returned.0, returned.1.as_str(), returned.3),
        (signed_up, "registered", None)
    );
    let (_, hub) = f
        .call(&leaver, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert!(
        hub["missions"][0].get("my_withdrawn_at").is_none(),
        "an active signup carries no tombstone time"
    );
    f.pool().close().await;
}
