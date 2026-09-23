//! PostgreSQL eligibility projections preserve membership provenance and event policy boundaries.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use website_api::core::{database, error_handling::api_error::ApiError};
use website_api::operations::{
    models::{
        event_access_policy::{EventAccessCondition, EventAccessGrant, EventAccessPolicy},
        event_group::EventGroupSource,
    },
    services::event_access::{
        context::EventAccessContext,
        evaluation::{
            AccessDenial, EventAccessSubject, MandatoryAccessConstraints, PolicySource,
            evaluate_access,
        },
    },
};

mod common;

struct Fixture {
    pool: PgPool,
    actor: String,
    main_guild: String,
    partner_guild: String,
    event: Uuid,
}

impl Fixture {
    async fn new() -> Self {
        let url = common::require_test_database_url().expect("event access requires PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let actor = format!("event-access-{}", Uuid::new_v4());
        sqlx::query("INSERT INTO users(discord_id, username, created_at, updated_at) VALUES ($1, $1, now(), now())")
            .bind(&actor).execute(&pool).await.unwrap();
        let event = Self::insert_event(&pool, &actor).await;
        Self {
            pool,
            actor,
            main_guild: format!("main-{}", Uuid::new_v4()),
            partner_guild: format!("partner-{}", Uuid::new_v4()),
            event,
        }
    }

    async fn insert_event(pool: &PgPool, author: &str) -> Uuid {
        sqlx::query_scalar("INSERT INTO events(name_override, start_time, created_by, created_at) VALUES ('Eligibility fixture', now(), $1, now()) RETURNING id")
            .bind(author).fetch_one(pool).await.unwrap()
    }

    async fn load(&self) -> Result<EventAccessContext, ApiError> {
        EventAccessContext::load(&mut self.pool.acquire().await.unwrap(), self.event).await
    }

    async fn subject(&self, context: &EventAccessContext) -> Result<EventAccessSubject, ApiError> {
        context
            .subject(
                &mut self.pool.acquire().await.unwrap(),
                &self.actor,
                &self.main_guild,
            )
            .await
    }

    async fn group(&self, event: Uuid, source: EventGroupSource) -> Uuid {
        sqlx::query_scalar("INSERT INTO event_groups(event_id, name, source, created_by) VALUES ($1, 'Eligibility group', $2, $3) RETURNING id")
            .bind(event).bind(serde_json::to_value(source).unwrap()).bind(&self.actor)
            .fetch_one(&self.pool).await.unwrap()
    }

    async fn partner_group(&self, roles: &[&str]) -> Uuid {
        self.group(
            self.event,
            EventGroupSource::PartnerGuild {
                guild_id: self.partner_guild.clone(),
                required_role_ids: roles.iter().map(|role| (*role).to_owned()).collect(),
            },
        )
        .await
    }

    async fn roster(&self, group: Uuid) {
        sqlx::query(
            "INSERT INTO event_group_roster(group_id, discord_id, added_by) VALUES ($1, $2, $2)",
        )
        .bind(group)
        .bind(&self.actor)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn snapshot(&self, guild: &str, status: &str, age_hours: Option<i32>, roles: &[&str]) {
        let mut transaction = self.pool.begin().await.unwrap();
        sqlx::query("INSERT INTO discord_membership_snapshots(discord_id, guild_id, membership_status, verified_at, last_error)
            VALUES ($1, $2, $3, CASE WHEN $4::integer IS NULL THEN NULL ELSE clock_timestamp() - make_interval(hours => $4) END, 'fixture transport unavailable')
            ON CONFLICT (discord_id, guild_id) DO UPDATE SET membership_status = EXCLUDED.membership_status,
                verified_at = EXCLUDED.verified_at, last_error = EXCLUDED.last_error")
            .bind(&self.actor).bind(guild).bind(status).bind(age_hours).execute(&mut *transaction).await.unwrap();
        sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1 AND guild_id = $2")
            .bind(&self.actor)
            .bind(guild)
            .execute(&mut *transaction)
            .await
            .unwrap();
        for role in roles {
            sqlx::query("INSERT INTO user_discord_roles(discord_id, guild_id, discord_role_id) VALUES ($1, $2, $3)")
                .bind(&self.actor).bind(guild).bind(role).execute(&mut *transaction).await.unwrap();
        }
        transaction.commit().await.unwrap();
    }

    async fn grace_override(&self, guild: &str) {
        sqlx::query("INSERT INTO discord_membership_grace_overrides(discord_id, guild_id, authorized_by, reason, created_at, expires_at)
            VALUES ($1, $2, $1, 'Explicit fixture outage extension', clock_timestamp(), clock_timestamp() + interval '24 hours')
            ON CONFLICT (discord_id, guild_id) DO UPDATE SET created_at = EXCLUDED.created_at, expires_at = EXCLUDED.expires_at")
            .bind(&self.actor).bind(guild).execute(&self.pool).await.unwrap();
    }

    async fn mission(&self, event: Uuid) -> (Uuid, Uuid) {
        let mission: Uuid = sqlx::query_scalar("INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at)
            VALUES ('Eligibility mission', $1, 'everon', 'pve_coop', 32, 'live', now()) RETURNING id")
            .bind(&self.actor).fetch_one(&self.pool).await.unwrap();
        let event_mission = sqlx::query_scalar("INSERT INTO event_missions(event_id, mission_id, start_time, created_at) VALUES ($1, $2, now(), now()) RETURNING id")
            .bind(event).bind(mission).fetch_one(&self.pool).await.unwrap();
        (mission, event_mission)
    }

    async fn squad(&self, mission: Uuid, faction: &str, squad: &str, policy: &EventAccessPolicy) {
        sqlx::query("INSERT INTO event_squad_access_policies(event_mission_id, faction, squad, access_policy) VALUES ($1, $2, $3, $4)")
            .bind(mission).bind(faction).bind(squad).bind(serde_json::to_value(policy).unwrap())
            .execute(&self.pool).await.unwrap();
    }

    async fn slot(
        &self,
        mission: Uuid,
        faction: &str,
        index: i64,
        policy: Option<&EventAccessPolicy>,
    ) -> Uuid {
        sqlx::query_scalar("INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, access_policy)
            VALUES ($1, $2, 'Alpha', 'Rifleman', $3, $4) RETURNING id")
            .bind(mission).bind(faction).bind(index).bind(policy.map(|policy| serde_json::to_value(policy).unwrap()))
            .fetch_one(&self.pool).await.unwrap()
    }
}

fn policy(alternatives: Vec<Vec<EventAccessCondition>>) -> EventAccessPolicy {
    EventAccessPolicy {
        grants: alternatives
            .into_iter()
            .map(|conditions| EventAccessGrant { conditions })
            .collect(),
    }
}

fn group_policy(group: Uuid) -> EventAccessPolicy {
    policy(vec![vec![EventAccessCondition::EventGroup {
        group_id: group,
    }]])
}

fn open_policy() -> EventAccessPolicy {
    policy(vec![vec![EventAccessCondition::Authenticated {}]])
}

fn constraints() -> MandatoryAccessConstraints {
    MandatoryAccessConstraints::SATISFIED
}

fn permitted(policy: &EventAccessPolicy, subject: &EventAccessSubject) -> bool {
    evaluate_access(policy, None, None, subject, constraints())
        .unwrap()
        .denial
        .is_none()
}

#[tokio::test]
async fn event_access_default_policy_distinguishes_empty_member_roles_guests_and_anonymous() {
    let fixture = Fixture::new().await;
    let context = fixture.load().await.unwrap();
    assert_eq!(context.event_id, fixture.event);
    assert_eq!(context.revision, 0);
    assert_eq!(context.policy, EventAccessPolicy::default());
    let guest = fixture.subject(&context).await.unwrap();
    assert_eq!(guest.discord_id, fixture.actor);
    assert!(!guest.tbd_member);
    assert!(!permitted(&context.policy, &guest));
    assert!(permitted(&open_policy(), &guest));
    assert_eq!(
        evaluate_access(
            &open_policy(),
            None,
            None,
            &EventAccessSubject::default(),
            constraints()
        )
        .unwrap()
        .denial,
        Some(AccessDenial::InvalidSession)
    );
    fixture
        .snapshot(&fixture.main_guild, "member", Some(0), &[])
        .await;
    let member = fixture.subject(&context).await.unwrap();
    assert!(member.tbd_member);
    assert!(member.guild_roles[&fixture.main_guild].is_empty());
    assert!(permitted(&context.policy, &member));
    fixture
        .snapshot(&fixture.main_guild, "nonmember", Some(0), &[])
        .await;
    let departed = fixture.subject(&context).await.unwrap();
    assert!(!departed.tbd_member);
    assert!(!permitted(&context.policy, &departed));
    assert!(permitted(&open_policy(), &departed));
}

#[tokio::test]
async fn event_access_group_provenance_ignores_rosters_for_partners_and_requires_every_role() {
    let fixture = Fixture::new().await;
    let managed = fixture
        .group(fixture.event, EventGroupSource::ManagedRoster {})
        .await;
    let partner = fixture.partner_group(&["medic", "leader"]).await;
    let any_member = fixture.partner_group(&[]).await;
    fixture.roster(managed).await;
    fixture.roster(partner).await;
    let context = fixture.load().await.unwrap();
    let sources: Vec<_> = context
        .groups
        .iter()
        .map(|group| serde_json::to_value(&group.source).unwrap())
        .collect();
    assert!(sources.contains(&json!({"kind": "managed_roster"})));
    assert!(sources.contains(&json!({"kind": "partner_guild", "guild_id": fixture.partner_guild, "required_role_ids": ["medic", "leader"]})));
    let initial = fixture.subject(&context).await.unwrap();
    assert!(initial.event_groups.contains(&managed));
    assert!(!initial.event_groups.contains(&partner));
    assert!(!initial.event_groups.contains(&any_member));
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(0), &[])
        .await;
    let no_roles = fixture.subject(&context).await.unwrap();
    assert!(no_roles.event_groups.contains(&any_member));
    assert!(!no_roles.event_groups.contains(&partner));
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(0), &["medic"])
        .await;
    fixture
        .snapshot(&fixture.main_guild, "member", Some(0), &["leader"])
        .await;
    assert!(
        !fixture
            .subject(&context)
            .await
            .unwrap()
            .event_groups
            .contains(&partner)
    );
    fixture
        .snapshot(
            &fixture.partner_guild,
            "member",
            Some(0),
            &["leader", "medic"],
        )
        .await;
    assert!(
        fixture
            .subject(&context)
            .await
            .unwrap()
            .event_groups
            .contains(&partner)
    );
    sqlx::query("UPDATE event_group_roster SET removed_at = clock_timestamp() WHERE group_id = $1 AND discord_id = $2")
        .bind(managed).bind(&fixture.actor).execute(&fixture.pool).await.unwrap();
    let removed = fixture.subject(&context).await.unwrap();
    assert!(!removed.event_groups.contains(&managed));
    assert!(removed.event_groups.contains(&partner));
}

#[tokio::test]
async fn event_access_stale_unknown_future_and_departed_partners_revoke_only_dependent_grants() {
    let fixture = Fixture::new().await;
    let partner = fixture.partner_group(&["role"]).await;
    let managed = fixture
        .group(fixture.event, EventGroupSource::ManagedRoster {})
        .await;
    fixture.roster(managed).await;
    fixture
        .snapshot(&fixture.main_guild, "member", Some(0), &["main-role"])
        .await;
    let context = fixture.load().await.unwrap();
    let either = policy(vec![
        vec![EventAccessCondition::EventGroup { group_id: partner }],
        vec![EventAccessCondition::TbdMember {}],
    ]);
    let partner_role = policy(vec![vec![EventAccessCondition::DiscordRole {
        guild_id: fixture.partner_guild.clone(),
        role_id: "role".into(),
    }]]);
    for (status, age) in [
        ("member", Some(49)),
        ("unknown", None),
        ("unknown", Some(0)),
        ("member", Some(-1)),
        ("nonmember", Some(0)),
    ] {
        fixture
            .snapshot(&fixture.partner_guild, status, age, &["role"])
            .await;
        let subject = fixture.subject(&context).await.unwrap();
        assert!(
            !subject.guild_roles.contains_key(&fixture.partner_guild),
            "{status}, {age:?}"
        );
        assert!(!permitted(&group_policy(partner), &subject));
        assert!(!permitted(&partner_role, &subject));
        assert!(subject.tbd_member);
        assert!(permitted(&group_policy(managed), &subject));
        assert!(permitted(&context.policy, &subject));
        assert!(permitted(&either, &subject));
    }
    fixture
        .snapshot(&fixture.main_guild, "member", Some(49), &["main-role"])
        .await;
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(0), &["role"])
        .await;
    let partner_guest = fixture.subject(&context).await.unwrap();
    assert!(!partner_guest.tbd_member);
    assert!(permitted(&group_policy(partner), &partner_guest));
    assert!(permitted(&either, &partner_guest));
}

#[tokio::test]
async fn event_access_outage_cache_and_override_preserve_only_verified_nonfuture_members() {
    let fixture = Fixture::new().await;
    let partner = fixture.partner_group(&["role"]).await;
    fixture
        .snapshot(&fixture.main_guild, "member", Some(24), &[])
        .await;
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(24), &["role"])
        .await;
    let context = fixture.load().await.unwrap();
    let cached = fixture.subject(&context).await.unwrap();
    assert!(cached.tbd_member);
    assert!(permitted(&group_policy(partner), &cached));
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(49), &["role"])
        .await;
    assert!(!permitted(
        &group_policy(partner),
        &fixture.subject(&context).await.unwrap()
    ));
    fixture.grace_override(&fixture.partner_guild).await;
    assert!(permitted(
        &group_policy(partner),
        &fixture.subject(&context).await.unwrap()
    ));
    for (status, age) in [
        ("nonmember", Some(0)),
        ("unknown", None),
        ("member", Some(-1)),
    ] {
        fixture
            .snapshot(&fixture.partner_guild, status, age, &["role"])
            .await;
        let subject = fixture.subject(&context).await.unwrap();
        assert!(
            !permitted(&group_policy(partner), &subject),
            "{status}, {age:?}"
        );
        assert!(subject.tbd_member);
    }
    fixture
        .snapshot(&fixture.partner_guild, "member", Some(49), &["role"])
        .await;
    sqlx::query("UPDATE discord_membership_grace_overrides SET created_at = clock_timestamp() - interval '24 hours', expires_at = clock_timestamp() - interval '1 hour' WHERE discord_id = $1 AND guild_id = $2")
        .bind(&fixture.actor).bind(&fixture.partner_guild).execute(&fixture.pool).await.unwrap();
    assert!(!permitted(
        &group_policy(partner),
        &fixture.subject(&context).await.unwrap()
    ));
}

#[tokio::test]
async fn event_access_unavailable_accounts_cannot_obtain_a_subject_even_with_all_grants() {
    let fixture = Fixture::new().await;
    let managed = fixture
        .group(fixture.event, EventGroupSource::ManagedRoster {})
        .await;
    fixture.roster(managed).await;
    fixture
        .snapshot(&fixture.main_guild, "member", Some(0), &["role"])
        .await;
    let context = fixture.load().await.unwrap();
    for (banned, deleted) in [(true, false), (false, true), (true, true)] {
        sqlx::query("UPDATE users SET is_banned = $2, deleted_at = CASE WHEN $3 THEN clock_timestamp() ELSE NULL END WHERE discord_id = $1")
            .bind(&fixture.actor).bind(banned).bind(deleted).execute(&fixture.pool).await.unwrap();
        assert_eq!(
            fixture.subject(&context).await.unwrap_err().status,
            StatusCode::FORBIDDEN
        );
    }
    let missing = format!("missing-{}", Uuid::new_v4());
    assert_eq!(
        context
            .subject(
                &mut fixture.pool.acquire().await.unwrap(),
                &missing,
                &fixture.main_guild
            )
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn event_access_policy_references_require_live_groups_and_guilds_from_the_same_event() {
    let fixture = Fixture::new().await;
    let own_group = fixture
        .group(fixture.event, EventGroupSource::ManagedRoster {})
        .await;
    let own_partner = fixture.partner_group(&[]).await;
    let other_event = Fixture::insert_event(&fixture.pool, &fixture.actor).await;
    let foreign_guild = format!("foreign-{}", Uuid::new_v4());
    let foreign_group = fixture
        .group(
            other_event,
            EventGroupSource::PartnerGuild {
                guild_id: foreign_guild.clone(),
                required_role_ids: vec![],
            },
        )
        .await;
    let context = fixture.load().await.unwrap();
    for candidate in [
        group_policy(own_group),
        group_policy(own_partner),
        EventAccessPolicy::default(),
        open_policy(),
        policy(vec![vec![EventAccessCondition::NamedAccount {
            discord_id: fixture.actor.clone(),
        }]]),
    ] {
        context
            .validate_policy_references(&candidate, &fixture.main_guild)
            .unwrap();
    }
    for guild in [&fixture.main_guild, &fixture.partner_guild] {
        context
            .validate_policy_references(
                &policy(vec![vec![EventAccessCondition::DiscordRole {
                    guild_id: guild.clone(),
                    role_id: "role".into(),
                }]]),
                &fixture.main_guild,
            )
            .unwrap();
    }
    for candidate in [
        group_policy(foreign_group),
        group_policy(Uuid::new_v4()),
        policy(vec![vec![EventAccessCondition::DiscordRole {
            guild_id: foreign_guild,
            role_id: "role".into(),
        }]]),
    ] {
        assert_eq!(
            context
                .validate_policy_references(&candidate, &fixture.main_guild)
                .unwrap_err()
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    sqlx::query("UPDATE event_groups SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(own_partner)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let changed = fixture.load().await.unwrap();
    assert!(!changed.groups.iter().any(|group| group.id == own_partner));
    assert!(
        changed
            .validate_policy_references(&group_policy(own_partner), &fixture.main_guild)
            .is_err()
    );
    assert!(
        changed
            .validate_policy_references(
                &policy(vec![vec![EventAccessCondition::DiscordRole {
                    guild_id: fixture.partner_guild.clone(),
                    role_id: "role".into()
                }]]),
                &fixture.main_guild
            )
            .is_err()
    );
}

#[tokio::test]
async fn event_access_squad_keys_include_mission_and_faction_and_deleted_missions_leave_operational_views()
 {
    let fixture = Fixture::new().await;
    let (live_mission, live) = fixture.mission(fixture.event).await;
    let (deleted_mission, deleted) = fixture.mission(fixture.event).await;
    let other_event = Fixture::insert_event(&fixture.pool, &fixture.actor).await;
    let (_, other) = fixture.mission(other_event).await;
    let closed = policy(vec![]);
    let open = open_policy();
    fixture.squad(live, "blue", "Alpha", &open).await;
    fixture.squad(live, "red", "Alpha", &closed).await;
    fixture.squad(deleted, "blue", "Alpha", &open).await;
    fixture.squad(other, "blue", "Alpha", &open).await;
    let explicit_slot = fixture.slot(live, "red", 0, Some(&open)).await;
    let inherited_slot = fixture.slot(live, "blue", 0, None).await;
    let deleted_slot = fixture.slot(deleted, "blue", 0, Some(&open)).await;
    let other_slot = fixture.slot(other, "blue", 0, Some(&open)).await;
    let before = fixture.load().await.unwrap();
    assert_eq!(before.squad_policies.len(), 3);
    assert_eq!(before.slot_policies.len(), 2);
    assert_eq!(
        before.squad_policies[&(live, "blue".into(), "Alpha".into())],
        open
    );
    assert_eq!(
        before.squad_policies[&(live, "red".into(), "Alpha".into())],
        closed
    );
    assert!(!before.slot_policies.contains_key(&inherited_slot));
    assert!(!before.slot_policies.contains_key(&other_slot));
    let guest = fixture.subject(&before).await.unwrap();
    let decision = evaluate_access(
        &before.policy,
        before
            .squad_policies
            .get(&(live, "red".into(), "Alpha".into())),
        before.slot_policies.get(&explicit_slot),
        &guest,
        constraints(),
    )
    .unwrap();
    assert_eq!(decision.policy_source, PolicySource::Slot);
    assert_eq!(decision.denial, None);
    sqlx::query("UPDATE missions SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(deleted_mission)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let after = fixture.load().await.unwrap();
    assert_eq!(after.squad_policies.len(), 2);
    assert_eq!(after.slot_policies.len(), 1);
    assert!(!after.slot_policies.contains_key(&deleted_slot));
    assert!(
        after
            .squad_policies
            .keys()
            .all(|(mission, _, _)| *mission == live)
    );
    let retained: (i64, i64, i64) = sqlx::query_as(
        "SELECT
        (SELECT count(*) FROM event_missions WHERE id = $1),
        (SELECT count(*) FROM orbat_slots WHERE id = $2),
        (SELECT count(*) FROM missions WHERE id = $3 AND deleted_at IS NULL)",
    )
    .bind(deleted)
    .bind(deleted_slot)
    .bind(live_mission)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(retained, (1, 1, 1));
}

#[tokio::test]
async fn event_access_corrupt_stored_policies_and_group_sources_fail_closed() {
    let fixture = Fixture::new().await;
    for invalid in [
        json!({}),
        json!({"grants": null}),
        json!({"grants": [{"conditions": []}]}),
        json!({"grants": [{"conditions": [{"kind": "authenticated", "unexpected": true}]}]}),
    ] {
        sqlx::query("UPDATE events SET access_policy = $2 WHERE id = $1")
            .bind(fixture.event)
            .bind(invalid)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(
            fixture.load().await.unwrap_err().status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    sqlx::query("UPDATE events SET access_policy = $2, access_revision = 7 WHERE id = $1")
        .bind(fixture.event)
        .bind(serde_json::to_value(open_policy()).unwrap())
        .execute(&fixture.pool)
        .await
        .unwrap();
    let (_, mission) = fixture.mission(fixture.event).await;
    let slot = fixture.slot(mission, "blue", 0, Some(&open_policy())).await;
    sqlx::query("UPDATE orbat_slots SET access_policy = $2 WHERE id = $1")
        .bind(slot)
        .bind(json!({"grants": [{"conditions": []}]}))
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert!(fixture.load().await.is_err());
    sqlx::query("UPDATE orbat_slots SET access_policy = NULL WHERE id = $1")
        .bind(slot)
        .execute(&fixture.pool)
        .await
        .unwrap();
    fixture
        .squad(mission, "blue", "Alpha", &open_policy())
        .await;
    sqlx::query(
        "UPDATE event_squad_access_policies SET access_policy = $2 WHERE event_mission_id = $1",
    )
    .bind(mission)
    .bind(json!({"grants": [{"conditions": [{"kind": "unknown"}]}]}))
    .execute(&fixture.pool)
    .await
    .unwrap();
    assert!(fixture.load().await.is_err());
    sqlx::query("DELETE FROM event_squad_access_policies WHERE event_mission_id = $1")
        .bind(mission)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let group = fixture
        .group(fixture.event, EventGroupSource::ManagedRoster {})
        .await;
    for source in [
        json!({"kind": "managed_roster", "guild_id": "unexpected"}),
        json!({"kind": "partner_guild", "guild_id": "", "required_role_ids": []}),
        json!({"kind": "partner_guild", "guild_id": "guild", "required_role_ids": [""]}),
        json!({"kind": "partner_guild", "guild_id": "guild", "required_role_ids": null}),
        json!({"kind": "unverified_self_assertion"}),
    ] {
        sqlx::query("UPDATE event_groups SET source = $2 WHERE id = $1")
            .bind(group)
            .bind(source)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(
            fixture.load().await.unwrap_err().status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    sqlx::query("UPDATE event_groups SET source = $2 WHERE id = $1")
        .bind(group)
        .bind(serde_json::to_value(EventGroupSource::ManagedRoster {}).unwrap())
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(fixture.load().await.unwrap().revision, 7);
    sqlx::query("UPDATE events SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(fixture.event)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(
        fixture.load().await.unwrap_err().status,
        StatusCode::NOT_FOUND
    );
}
