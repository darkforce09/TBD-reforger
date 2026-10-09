//! PostgreSQL eligibility projections preserve membership provenance and event policy boundaries.

use crate::common;

use api_foundation::error_handling::api_error::ApiError;
use api_identifiers::{
    DiscordGuildId, DiscordUserId, EventGroupId, EventId, EventMissionId, MissionId, OrbatSlotId,
};
use api_operations::{
    models::{
        event_access_policy::{EventAccessCondition, EventAccessGrant, EventAccessPolicy},
        event_group::EventGroupSource,
    },
    services::event_access::{
        context::EventAccessContext,
        evaluation::{
            AccessDenial, EventAccessSubject, MandatoryAccessConstraints, evaluate_access,
        },
    },
};
use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    pool: PgPool,
    actor: String,
    main_guild: String,
    event: EventId,
}

impl Fixture {
    /// The main guild as the typed Discord guild id the access services take.
    fn main_guild_id(&self) -> DiscordGuildId {
        DiscordGuildId::new(self.main_guild.as_str())
    }

    async fn new() -> Self {
        let url = common::require_test_database_url().expect("event access requires PostgreSQL");
        let pool = api_database::connect(&url)
            .await
            .expect("the test database accepts a connection");
        api_database::migrate(&pool)
            .await
            .expect("the migrations apply to the test database");
        let actor = format!("event-access-{}", Uuid::new_v4());
        sqlx::query("INSERT INTO users(discord_id, username, created_at, updated_at) VALUES ($1, $1, now(), now())")
            .bind(&actor).execute(&pool).await.expect("the insert into users succeeds");
        let event = Self::insert_event(&pool, &actor).await;
        Self {
            pool,
            actor,
            main_guild: format!("main-{}", Uuid::new_v4()),
            event,
        }
    }

    async fn insert_event(pool: &PgPool, author: &str) -> EventId {
        sqlx::query_scalar("INSERT INTO events(name_override, start_time, created_by, created_at) VALUES ('Eligibility fixture', now(), $1, now()) RETURNING id")
            .bind(author).fetch_one(pool).await.expect("the insert into events returns its row")
    }

    async fn load(&self) -> Result<EventAccessContext, ApiError> {
        EventAccessContext::load(
            &mut self
                .pool
                .acquire()
                .await
                .expect("the pool hands out a connection"),
            self.event,
        )
        .await
    }

    async fn subject(&self, context: &EventAccessContext) -> Result<EventAccessSubject, ApiError> {
        context
            .subject(
                &mut self
                    .pool
                    .acquire()
                    .await
                    .expect("the pool hands out a connection"),
                &DiscordUserId::new(self.actor.as_str()),
                &self.main_guild_id(),
            )
            .await
    }

    async fn group(&self, event: EventId, source: EventGroupSource) -> EventGroupId {
        sqlx::query_scalar("INSERT INTO event_groups(event_id, name, source, created_by) VALUES ($1, 'Eligibility group', $2, $3) RETURNING id")
            .bind(event).bind(serde_json::to_value(source).expect("the value serialises to JSON")).bind(&self.actor)
            .fetch_one(&self.pool).await.expect("the insert into event_groups returns its row")
    }

    async fn snapshot(&self, guild: &str, status: &str, age_hours: Option<i32>, roles: &[&str]) {
        let mut transaction = self.pool.begin().await.expect("a transaction begins");
        sqlx::query("INSERT INTO discord_membership_snapshots(discord_id, guild_id, membership_status, verified_at, last_error)
            VALUES ($1, $2, $3, CASE WHEN $4::integer IS NULL THEN NULL ELSE clock_timestamp() - make_interval(hours => $4) END, 'fixture transport unavailable')
            ON CONFLICT (discord_id, guild_id) DO UPDATE SET membership_status = EXCLUDED.membership_status,
                verified_at = EXCLUDED.verified_at, last_error = EXCLUDED.last_error")
            .bind(&self.actor).bind(guild).bind(status).bind(age_hours).execute(&mut *transaction).await.expect("the insert into discord_membership_snapshots succeeds");
        sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1 AND guild_id = $2")
            .bind(&self.actor)
            .bind(guild)
            .execute(&mut *transaction)
            .await
            .expect("the delete from user_discord_roles succeeds");
        for role in roles {
            sqlx::query("INSERT INTO user_discord_roles(discord_id, guild_id, discord_role_id) VALUES ($1, $2, $3)")
                .bind(&self.actor).bind(guild).bind(role).execute(&mut *transaction).await.expect("the insert into user_discord_roles succeeds");
        }
        transaction.commit().await.expect("the transaction commits");
    }

    async fn mission(&self, event: EventId) -> (MissionId, EventMissionId) {
        let mission: MissionId = sqlx::query_scalar("INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at)
            VALUES ('Eligibility mission', $1, 'everon', 'pve_coop', 32, 'live', now()) RETURNING id")
            .bind(&self.actor).fetch_one(&self.pool).await.expect("the insert into missions returns its row");
        let event_mission = sqlx::query_scalar("INSERT INTO event_missions(event_id, mission_id, start_time, created_at) VALUES ($1, $2, now(), now()) RETURNING id")
            .bind(event).bind(mission).fetch_one(&self.pool).await.expect("the insert into event_missions returns its row");
        (mission, event_mission)
    }

    async fn squad(
        &self,
        mission: EventMissionId,
        faction: &str,
        squad: &str,
        policy: &EventAccessPolicy,
    ) {
        sqlx::query("INSERT INTO event_squad_access_policies(event_mission_id, faction, squad, access_policy) VALUES ($1, $2, $3, $4)")
            .bind(mission).bind(faction).bind(squad).bind(serde_json::to_value(policy).expect("the value serialises to JSON"))
            .execute(&self.pool).await.expect("the insert into event_squad_access_policies succeeds");
    }

    async fn slot(
        &self,
        mission: EventMissionId,
        faction: &str,
        index: i64,
        policy: Option<&EventAccessPolicy>,
    ) -> OrbatSlotId {
        sqlx::query_scalar("INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, access_policy)
            VALUES ($1, $2, 'Alpha', 'Rifleman', $3, $4) RETURNING id")
            .bind(mission).bind(faction).bind(index).bind(policy.map(|policy| serde_json::to_value(policy).expect("the value serialises to JSON")))
            .fetch_one(&self.pool).await.expect("the insert into orbat_slots returns its row")
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

fn open_policy() -> EventAccessPolicy {
    policy(vec![vec![EventAccessCondition::Authenticated {}]])
}

fn constraints() -> MandatoryAccessConstraints {
    MandatoryAccessConstraints::SATISFIED
}

fn permitted(policy: &EventAccessPolicy, subject: &EventAccessSubject) -> bool {
    evaluate_access(policy, None, None, subject, constraints())
        .expect("the access evaluation succeeds")
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
    assert_eq!(guest.discord_id.as_str(), fixture.actor);
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
