//! Manager-controlled event, squad and slot policies and groups, exercised through the real
//! HTTP routes and PostgreSQL: precedence, grant algebra, mandatory gates, group provenance,
//! bot-verified partner membership and optimistic access revisions.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

mod common;
mod event_eligibility_support;

use event_eligibility_support::{
    EventShape, Fixture, PARTNER_ROLE, named, single_grant, tbd_members,
};

const SUITE: &str = "event_eligibility_policies";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn slot_policy_precedence_overrides_squad_and_event_over_http() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Bravo", "Bravo"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let member = f.member("member").await;
    let second_member = f.member("second-member").await;
    let guest = f.guest("guest").await;

    // The default event policy admits verified TBD members only.
    let refused = f.register(&guest, 0, Some(0)).await;
    assert_eq!(refused.0, StatusCode::FORBIDDEN, "{refused:?}");
    assert_eq!(code(&refused.1), "ACCESS_POLICY");
    assert_eq!(refused.1["details"]["policy_source"], "event");
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);

    // An explicit squad policy replaces the event policy for every seat of the squad.
    let squad = f.put_squad_policy(0, "Bravo", named(&guest)).await;
    assert_eq!(squad.0, StatusCode::OK, "{squad:?}");
    assert_eq!(f.register(&guest, 0, Some(2)).await.0, StatusCode::OK);
    let excluded = f.register(&second_member, 0, Some(3)).await;
    assert_eq!(excluded.0, StatusCode::FORBIDDEN, "{excluded:?}");
    assert_eq!(excluded.1["details"]["policy_source"], "squad");

    // An explicit slot policy replaces the squad policy; an empty grant list closes the seat.
    assert_eq!(
        f.put_slot_policy(0, 3, tbd_members()).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.put_slot_policy(0, 1, json!({"grants": []})).await.0,
        StatusCode::OK
    );
    let closed = f.register(&second_member, 0, Some(1)).await;
    assert_eq!(closed.0, StatusCode::FORBIDDEN, "{closed:?}");
    assert_eq!(closed.1["details"]["policy_source"], "slot");
    assert_eq!(
        f.register(&second_member, 0, Some(3)).await.0,
        StatusCode::OK
    );

    // Removing the explicit slot policy makes the seat inherit again.
    let revision = f.access_revision().await;
    let inherit = f
        .call(
            &f.admin,
            "DELETE",
            &format!(
                "/api/v1/event-missions/{}/slots/{}/access-policy?expected_access_revision={revision}",
                f.missions[0], f.slots[0][1]
            ),
            None,
        )
        .await;
    assert_eq!(inherit.0, StatusCode::OK, "{inherit:?}");
    let view = &inherit.1["access"];
    assert_eq!(view["slot_policies"].as_array().unwrap().len(), 1);
    assert_eq!(view["squad_policies"][0]["squad"], "Bravo");
    let third = f.member("third").await;
    assert_eq!(f.register(&third, 0, Some(1)).await.0, StatusCode::OK);
    f.pool().close().await;
}

#[tokio::test]
async fn policy_grants_or_alternatives_and_conjoined_conditions_over_http() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Alpha", "Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let officer = f.member("officer").await;
    f.observe(
        &officer,
        &f.main_guild.clone(),
        "member",
        &["officer-role"],
        0,
    )
    .await;
    let plain_member = f.member("plain").await;
    let rostered_guest = f.guest("rostered").await;
    let rostered_member = f.member("rostered-member").await;
    let named_guest = f.guest("named").await;
    let outsider = f.guest("outsider").await;
    let group = f
        .create_group("Allies", json!({"kind": "managed_roster"}))
        .await;
    assert_eq!(f.add_roster(group, &rostered_guest).await.0, StatusCode::OK);
    assert_eq!(
        f.add_roster(group, &rostered_member).await.0,
        StatusCode::OK
    );
    let policy = json!({"grants": [
        {"conditions": [{"kind": "tbd_member"}, {"kind": "discord_role", "guild_id": f.main_guild, "role_id": "officer-role"}]},
        {"conditions": [{"kind": "event_group", "group_id": group}, {"kind": "tbd_member"}]},
        {"conditions": [{"kind": "named_account", "discord_id": named_guest.id}]}
    ]});
    let changed = f.put_event_policy(policy).await;
    assert_eq!(changed.0, StatusCode::OK, "{changed:?}");

    for (index, (actor, admitted)) in [
        (&officer, true),
        (&plain_member, false),
        (&rostered_guest, false),
        (&rostered_member, true),
        (&named_guest, true),
        (&outsider, false),
    ]
    .into_iter()
    .enumerate()
    {
        let response = f.register(actor, 0, Some(index)).await;
        let expected = if admitted {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        };
        assert_eq!(response.0, expected, "{} {response:?}", actor.id);
    }
    // The manager explanation names the alternative that admitted each participant.
    let (status, participants) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access/participants", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{participants}");
    let grants_of = |account: &str| -> Value {
        participants
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["discord_id"] == account)
            .map(|entry| entry["registrations"][0]["admitting_grants"].clone())
            .unwrap()
    };
    assert_eq!(grants_of(&officer.id), json!([0]));
    assert_eq!(grants_of(&rostered_member.id), json!([1]));
    assert_eq!(grants_of(&named_guest.id), json!([2]));
    f.pool().close().await;
}

#[tokio::test]
async fn policy_grants_cannot_bypass_ban_session_capacity_opening_or_lock() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    let holder = f.member("holder").await;
    let named_member = f.member("named-member").await;
    let named_guest = f.guest("named-guest").await;
    let banned = f.member("banned").await;
    let policy = json!({"grants": [{"conditions": [{"kind": "authenticated"}]}]});
    assert_eq!(f.put_event_policy(policy).await.0, StatusCode::OK);

    // Capacity: an admitting grant cannot exceed the event-wide limit.
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    let full = f.register(&named_member, 0, Some(1)).await;
    assert_eq!(full.0, StatusCode::CONFLICT, "{full:?}");
    assert_eq!(code(&full.1), "EVENT_FULL");

    // Opening times: the guest pool has places but opens later.
    let opens_at = "2099-01-01T00:00:00Z";
    let quotas = json!({
        "member": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"},
        "guest": {"seats": 5, "opens_at": opens_at},
        "open": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"}
    });
    let revision = f.access_revision().await;
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"max_slots": 0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(f.access_revision().await, revision);
    assert_eq!(f.put_quotas(quotas).await.0, StatusCode::OK);
    let early = f.register(&named_guest, 0, Some(1)).await;
    assert_eq!(early.0, StatusCode::CONFLICT, "{early:?}");
    assert_eq!(code(&early.1), "QUOTA_NOT_OPEN");
    assert_eq!(early.1["details"]["quota_kind"], "guest");
    assert!(early.1["error"].as_str().unwrap().contains("2099-01-01"));

    // A ban revokes the session and releases nothing a grant could restore.
    let (status, _) = f
        .call(
            &f.admin,
            "POST",
            &format!("/api/v1/admin/users/{}/ban", banned.id),
            Some(json!({"reason": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        f.register(&banned, 0, Some(1)).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Registration locked by an administrator admits nobody through self-service.
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"registration_locked": true})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let locked = f.register(&named_member, 0, Some(1)).await;
    assert_eq!(locked.0, StatusCode::FORBIDDEN, "{locked:?}");
    f.pool().close().await;
}

#[tokio::test]
async fn event_groups_managed_roster_and_partner_guild_provenance_are_visible() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let guest = f.guest("roster-guest").await;
    let partner = f.partner("partner", &[PARTNER_ROLE]).await;
    let roster = f
        .create_group("Roster", json!({"kind": "managed_roster"}))
        .await;
    assert_eq!(f.add_roster(roster, &guest).await.0, StatusCode::OK);
    let allies = f
        .create_group(
            "Partner unit",
            json!({"kind": "partner_guild", "guild_id": f.partner_guild, "required_role_ids": [PARTNER_ROLE]}),
        )
        .await;
    let policy = json!({"grants": [
        {"conditions": [{"kind": "event_group", "group_id": roster}]},
        {"conditions": [{"kind": "event_group", "group_id": allies}]}
    ]});
    assert_eq!(f.put_event_policy(policy).await.0, StatusCode::OK);
    assert_eq!(f.register(&guest, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&partner, 0, Some(1)).await.0, StatusCode::OK);

    let (status, view) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let group = |id: Uuid| {
        view["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|group| group["id"] == id.to_string())
            .cloned()
            .unwrap()
    };
    let roster_view = group(roster);
    assert_eq!(roster_view["provenance"]["created_by"], f.admin.id);
    assert!(roster_view["provenance"]["system_origin"].is_null());
    assert_eq!(roster_view["roster"][0]["discord_id"], guest.id);
    assert_eq!(roster_view["roster"][0]["added_by"], f.admin.id);
    let partner_view = group(allies);
    assert_eq!(partner_view["source"]["guild_id"], f.partner_guild);
    assert_eq!(partner_view["roster"], json!([]));

    let (_, participants) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access/participants", f.event),
            None,
        )
        .await;
    let entry = |account: &str| {
        participants
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["discord_id"] == account)
            .cloned()
            .unwrap()
    };
    let partner_entry = entry(&partner.id);
    let guild = partner_entry["guilds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|guild| guild["guild_id"] == f.partner_guild)
        .cloned()
        .unwrap();
    assert_eq!(guild["membership_status"], "member");
    assert_eq!(guild["current"], true);
    assert!(guild["verified_at"].is_string());
    assert_eq!(entry(&guest.id)["roster_groups"][0]["added_by"], f.admin.id);
    assert_eq!(entry(&guest.id)["allocation"]["quota_kind"], "guest");
    f.pool().close().await;
}

#[tokio::test]
async fn event_groups_partner_membership_requires_bot_verified_snapshot() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let allies = f
        .create_group(
            "Partner unit",
            json!({"kind": "partner_guild", "guild_id": f.partner_guild, "required_role_ids": [PARTNER_ROLE]}),
        )
        .await;
    assert_eq!(
        f.put_event_policy(single_grant(
            json!([{"kind": "event_group", "group_id": allies}])
        ))
        .await
        .0,
        StatusCode::OK
    );
    // No observation of the partner guild exists yet: verification is requested, never assumed.
    let pending = f.guest("pending").await;
    let waiting = f.register(&pending, 0, Some(0)).await;
    assert_eq!(waiting.0, StatusCode::FORBIDDEN, "{waiting:?}");
    assert_eq!(code(&waiting.1), "MEMBERSHIP_VERIFICATION_REQUIRED");
    let enrolled: Option<(String, bool)> = sqlx::query_as(
        "SELECT membership_status, next_refresh_at <= clock_timestamp() FROM discord_membership_snapshots
         WHERE discord_id = $1 AND guild_id = $2",
    )
    .bind(&pending.id)
    .bind(&f.partner_guild)
    .fetch_optional(f.pool())
    .await
    .unwrap();
    assert_eq!(enrolled, Some(("unknown".into(), true)));
    assert_eq!(
        f.registration(&pending, 0).await,
        None,
        "the refusal wrote nothing"
    );

    // The bot observation is the only source of partner membership.
    f.observe(
        &pending,
        &f.partner_guild.clone(),
        "member",
        &[PARTNER_ROLE],
        0,
    )
    .await;
    assert_eq!(f.register(&pending, 0, Some(0)).await.0, StatusCode::OK);
    let without_role = f.partner("without-role", &["other-role"]).await;
    let refused = f.register(&without_role, 0, Some(1)).await;
    assert_eq!(code(&refused.1), "ACCESS_POLICY", "{refused:?}");
    let departed = f.guest("departed").await;
    f.observe(&departed, &f.partner_guild.clone(), "nonmember", &[], 0)
        .await;
    assert_eq!(
        code(&f.register(&departed, 0, Some(2)).await.1),
        "ACCESS_POLICY"
    );
    // A stale verified member needs fresh verification for new places.
    let stale = f.guest("stale").await;
    f.observe(
        &stale,
        &f.partner_guild.clone(),
        "member",
        &[PARTNER_ROLE],
        60,
    )
    .await;
    let stale_claim = f.register(&stale, 0, Some(3)).await;
    assert_eq!(
        code(&stale_claim.1),
        "MEMBERSHIP_VERIFICATION_REQUIRED",
        "{stale_claim:?}"
    );
    // Participant evidence names every guild the policies rely on: a participant never
    // observed in the partner guild is listed there as unknown rather than omitted.
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id = $1 AND guild_id = $2")
        .bind(&pending.id)
        .bind(&f.partner_guild)
        .execute(f.pool())
        .await
        .unwrap();
    let (status, participants) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access/participants", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{participants}");
    let evidence = participants
        .as_array()
        .unwrap()
        .iter()
        .find(|participant| participant["discord_id"] == pending.id.as_str())
        .expect("the registered participant is explained");
    let partner = evidence["guilds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|guild| guild["guild_id"] == f.partner_guild.as_str())
        .expect("the partner guild the policy relies on is listed");
    assert_eq!(partner["membership_status"], "unknown");
    assert!(partner["verified_at"].is_null() && partner["current"] == false);
    f.pool().close().await;
}

#[tokio::test]
async fn event_groups_referenced_group_deletion_conflicts() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let group = f
        .create_group("Referenced", json!({"kind": "managed_roster"}))
        .await;
    let policy = single_grant(json!([{"kind": "event_group", "group_id": group}]));
    assert_eq!(f.put_event_policy(policy).await.0, StatusCode::OK);
    let delete = |revision: i64| {
        format!(
            "/api/v1/events/{}/groups/{group}?expected_access_revision={revision}",
            f.event
        )
    };
    let referenced = f
        .call(&f.admin, "DELETE", &delete(f.access_revision().await), None)
        .await;
    assert_eq!(referenced.0, StatusCode::CONFLICT, "{referenced:?}");
    // A form prepared against an older revision cannot overwrite a newer one.
    let stale_revision = f.access_revision().await;
    assert_eq!(f.put_event_policy(tbd_members()).await.0, StatusCode::OK);
    let stale = f
        .call(
            &f.admin,
            "PUT",
            &format!("/api/v1/events/{}/access-policy", f.event),
            Some(json!({"expected_access_revision": stale_revision, "policy": {"grants": []}})),
        )
        .await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    assert_eq!(code(&stale.1), "ACCESS_REVISION_CONFLICT");
    let removed = f
        .call(&f.admin, "DELETE", &delete(f.access_revision().await), None)
        .await;
    assert_eq!(removed.0, StatusCode::OK, "{removed:?}");
    assert_eq!(removed.1["access"]["groups"], json!([]));
    // Policies may name only this event's groups.
    let foreign = f
        .put_event_policy(single_grant(
            json!([{"kind": "event_group", "group_id": Uuid::new_v4()}]),
        ))
        .await;
    assert_eq!(foreign.0, StatusCode::BAD_REQUEST, "{foreign:?}");
    f.pool().close().await;
}
