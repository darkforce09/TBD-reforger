//! Authorized discovery: child-policy viewers find the event without unauthorized data, hidden
//! events are indistinguishable from missing ones, and list totals count only visible events.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod common;
mod event_eligibility_support;

use event_eligibility_support::{EventShape, Fixture, named};

const SUITE: &str = "event_visibility_projection";

fn seat_ids(orbat: &Value) -> Vec<String> {
    orbat["data"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|squad| squad["slots"].as_array().unwrap().iter())
        .map(|slot| slot["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn event_visibility_child_policy_discovers_event_without_unauthorized_data() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[
                &["Alpha", "Alpha", "Bravo", "Bravo"],
                &["Charlie", "Charlie"],
            ],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    sqlx::query("UPDATE events SET briefing = 'Members-only plan' WHERE id = $1")
        .bind(f.event)
        .execute(f.pool())
        .await
        .unwrap();
    let member = f.member("member").await;
    let guest = f.guest("guest").await;
    assert_eq!(f.register(&member, 0, Some(2)).await.0, StatusCode::OK);
    assert_eq!(
        f.put_squad_policy(0, "Alpha", named(&guest)).await.0,
        StatusCode::OK
    );

    // The guest discovers the event through the Alpha squad policy alone.
    let (status, hub) = f
        .call(&guest, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{hub}");
    assert_eq!(hub["viewer_access"]["visibility"], "partial");
    assert_eq!(hub["viewer_access"]["quota_class"], "guest");
    assert!(
        hub.get("briefing").is_none(),
        "event briefing belongs to the event audience"
    );
    let missions = hub["missions"].as_array().unwrap();
    assert_eq!(
        missions.len(),
        1,
        "only the mission with admitted seats: {hub}"
    );
    assert_eq!(missions[0]["total"], 2);
    assert_eq!(
        missions[0]["filled"], 0,
        "a restricted seat's occupant is not counted"
    );
    assert_eq!(missions[0]["viewer_eligible"], true);

    let (status, orbat) = f
        .call(
            &guest,
            "GET",
            &format!("/api/v1/event-missions/{}/orbat", f.missions[0]),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{orbat}");
    let visible = seat_ids(&orbat);
    assert_eq!(
        visible,
        vec![f.slots[0][0].to_string(), f.slots[0][1].to_string()]
    );
    assert!(
        !orbat.to_string().contains(&member.id),
        "no occupant outside admitted seats"
    );
    assert!(
        orbat["data"][0]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .all(|slot| slot["viewer_access"] == "eligible")
    );

    // A member sees everything, with the guest-only squad marked restricted.
    let (_, member_orbat) = f
        .call(
            &member,
            "GET",
            &format!("/api/v1/event-missions/{}/orbat", f.missions[0]),
            None,
        )
        .await;
    assert_eq!(seat_ids(&member_orbat).len(), 4);
    let alpha = member_orbat["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|squad| squad["squad"] == "Alpha")
        .unwrap();
    assert_eq!(alpha["slots"][0]["viewer_access"], "restricted");
    assert_eq!(alpha["slots"][0]["policy_source"], "squad");
    let (_, member_hub) = f
        .call(&member, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(member_hub["viewer_access"]["visibility"], "full");
    assert_eq!(member_hub["briefing"], "Members-only plan");
    assert_eq!(member_hub["missions"].as_array().unwrap().len(), 2);
    // A mission without admitted seats reads like a missing one, even inside a visible event.
    let (status, _) = f
        .call(
            &guest,
            "GET",
            &format!("/api/v1/event-missions/{}/orbat", f.missions[1]),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    f.pool().close().await;
}

#[tokio::test]
async fn event_visibility_hidden_event_is_indistinguishable_from_missing() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let guest = f.guest("guest").await;
    let missing = uuid::Uuid::new_v4();
    let hidden = f
        .call(&guest, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    let absent = f
        .call(&guest, "GET", &format!("/api/v1/events/{missing}"), None)
        .await;
    assert_eq!(hidden.0, StatusCode::NOT_FOUND);
    assert_eq!(
        hidden, absent,
        "a hidden event reads exactly like a missing one"
    );
    let hidden_orbat = f
        .call(
            &guest,
            "GET",
            &format!("/api/v1/event-missions/{}/orbat", f.missions[0]),
            None,
        )
        .await;
    let absent_orbat = f
        .call(
            &guest,
            "GET",
            &format!("/api/v1/event-missions/{missing}/orbat"),
            None,
        )
        .await;
    assert_eq!(hidden_orbat.0, StatusCode::NOT_FOUND);
    assert_eq!(hidden_orbat, absent_orbat);
    // Administrators always see the event.
    let (status, hub) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(hub["viewer_access"]["visibility"], "full");
    f.pool().close().await;
}

#[tokio::test]
async fn event_visibility_list_pagination_counts_only_visible_events() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let guest = f.guest("lister").await;
    // Three more events the guest may see; the fixture event stays members-only.
    let mut visible = Vec::new();
    for offset in 1..=3 {
        let event: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO events(name_override, start_time, status, created_by, access_policy)
             VALUES ('Open to guests', clock_timestamp() + make_interval(days => 3 + $1), 'open', $2,
                 jsonb_build_object('grants', jsonb_build_array(jsonb_build_object('conditions',
                     jsonb_build_array(jsonb_build_object('kind', 'authenticated'))))))
             RETURNING id",
        )
        .bind(offset)
        .bind(&f.admin.id)
        .fetch_one(f.pool())
        .await
        .unwrap();
        visible.push(event.to_string());
    }
    let listed = |limit: i64, offset: i64| {
        let guest = &guest;
        let f = &f;
        async move {
            f.call(
                guest,
                "GET",
                &format!("/api/v1/events?scope=all&limit={limit}&offset={offset}"),
                None,
            )
            .await
        }
    };
    let (status, first) = listed(2, 0).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (_, second) = listed(2, 2).await;
    let ids: Vec<String> = first["data"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["data"].as_array().unwrap().iter())
        .map(|event| event["id"].as_str().unwrap().to_owned())
        .filter(|id| visible.contains(id) || *id == f.event.to_string())
        .collect();
    assert!(
        !ids.contains(&f.event.to_string()),
        "hidden events are never listed"
    );
    for id in &visible {
        assert!(ids.contains(id), "{id} missing from pages");
    }
    // Totals count visible events only, so paging reaches every one exactly once.
    let total = first["total"].as_i64().unwrap();
    let (_, everything) = listed(100, 0).await;
    assert_eq!(everything["data"].as_array().unwrap().len() as i64, total);
    assert!(
        everything["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["id"] != json!(f.event.to_string()))
    );
    let (_, admin_list) = f
        .call(&f.admin, "GET", "/api/v1/events?scope=all&limit=100", None)
        .await;
    assert!(
        admin_list["total"].as_i64().unwrap() > total,
        "administrators also see the hidden event"
    );
    f.pool().close().await;
}
