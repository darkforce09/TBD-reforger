//! The operations events part's world: a shared visible event with one attached mission and one
//! registered participant, an event only one named account may see, catalog missions to attach, and a fresh event (with
//! its attachment, seat and roster group) for every probe that changes one.
//!
//! **Role:** implements [`PartWorld`] for the event, event access administration, ORBAT read,
//! member directory and leave review routes.
//!
//! **Position:** mounted by `tests/route_acceptance_operations_events.rs` with `#[path]`; its
//! specs are `specs/operations_events.rs`. Rows are written by direct SQL into the world's own
//! database, so every probe reaches the handler with exactly the rows it names.
//!
//! **Signals & state:** none beyond the ids of the rows seeded once in [`PartWorld::build`].
//!
//! **Invariants:** every changing route gets a freshly minted event per probe, so its access
//! revision is 0 and no probe observes another probe's change; the shared event admits every
//! signed-in account and lists the primary enlisted account as its one participant (a seatless
//! registration holding a member allocation), so the participants explanation is never an empty
//! array; the restricted event admits only the primary enlisted account; every
//! seeded timestamp is a fixed instant with a quarter-second fraction, so each run sends the
//! same fractional-second wire form through the contract parity check.

use api::core::application_state::AppState;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The faction of every seeded ORBAT seat.
pub const FACTION: &str = "BLUFOR";
/// The squad of every seeded ORBAT seat.
pub const SQUAD: &str = "Alpha";
/// A start time far enough ahead that every scheduling rule treats it as pre-start.
const FUTURE_START: &str = "2030-06-01T19:00:00Z";
/// The start of every seeded event and attachment: pre-start for every scheduling rule.
const SEEDED_START: &str = "2030-06-01T19:00:00.25Z";
/// The creation instant of every seeded event, group and roster entry (also the instant the
/// event's member reservation pool opens).
const SEEDED_CREATION: &str = "2026-09-01T10:00:00.25Z";
/// The policy of the shared event: any signed-in account may see and join it.
const ANY_SIGNED_IN: &str = r#"{"grants":[{"conditions":[{"kind":"authenticated"}]}]}"#;

/// One event minted for a single probe, with one attached mission, one seat and one roster.
struct FreshEvent {
    event: Uuid,
    attachment: Uuid,
    slot: Uuid,
    group: Uuid,
}

/// The operations events world.
pub struct OperationsEventsWorld {
    event: Uuid,
    attachment: Uuid,
    restricted_event: Uuid,
    restricted_attachment: Uuid,
    mission: Uuid,
    archived_mission: Uuid,
}

async fn insert_event(state: &AppState, administrator: &str, policy: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, max_slots, created_by, created_at,
             updated_at, access_policy)
         VALUES ('Route acceptance operation', $3::timestamptz, 'open', 16, $1,
             $4::timestamptz, $4::timestamptz, $2::jsonb) RETURNING id",
    )
    .bind(administrator)
    .bind(policy)
    .bind(SEEDED_START)
    .bind(SEEDED_CREATION)
    .fetch_one(&state.pool)
    .await
    .expect("insert an event")
}

async fn insert_mission(state: &AppState, author: &str, status: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
         VALUES ('Route acceptance mission', $1, 'everon', 'pve_coop', 64, $2::mission_status)
         RETURNING id",
    )
    .bind(author)
    .bind(status)
    .fetch_one(&state.pool)
    .await
    .expect("insert a mission")
}

/// Attach `mission` to `event` with one Rifleman seat in [`FACTION`]/[`SQUAD`].
async fn attach(state: &AppState, event: Uuid, mission: Uuid) -> (Uuid, Uuid) {
    let attachment: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time)
         VALUES ($1, $2, $3::timestamptz) RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .bind(SEEDED_START)
    .fetch_one(&state.pool)
    .await
    .expect("attach a mission");
    let slot: Uuid = sqlx::query_scalar(
        "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index)
         VALUES ($1, $2, $3, 'Rifleman', 0) RETURNING id",
    )
    .bind(attachment)
    .bind(FACTION)
    .bind(SQUAD)
    .fetch_one(&state.pool)
    .await
    .expect("seed an ORBAT seat");
    (attachment, slot)
}

/// Register `account` on `attachment` without a seat, holding its event's member allocation, both
/// stamped [`SEEDED_CREATION`].
async fn register_participant(state: &AppState, attachment: Uuid, account: &str) {
    sqlx::query(
        "WITH allocation AS (
             INSERT INTO event_participant_allocations(event_id, discord_id, quota_kind,
                 acquired_at)
             SELECT event_id, $2, 'member', $3::timestamptz FROM event_missions WHERE id = $1
             RETURNING id)
         INSERT INTO event_registrations(event_mission_id, discord_id, reservation_state,
             allocation_id, registered_at, queue_entered_at)
         SELECT $1, $2, 'registered', allocation.id, $3::timestamptz, $3::timestamptz
         FROM allocation",
    )
    .bind(attachment)
    .bind(account)
    .bind(SEEDED_CREATION)
    .execute(&state.pool)
    .await
    .expect("register the shared event's participant");
}

async fn insert_group(state: &AppState, event: Uuid, administrator: &str, source: Value) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO event_groups(event_id, name, source, created_by, created_at, updated_at)
         VALUES ($1, 'Route acceptance roster', $2, $3, $4::timestamptz, $4::timestamptz)
         RETURNING id",
    )
    .bind(event)
    .bind(source)
    .bind(administrator)
    .bind(SEEDED_CREATION)
    .fetch_one(&state.pool)
    .await
    .expect("insert an event group")
}

async fn add_to_roster(state: &AppState, group: Uuid, member: &str, administrator: &str) {
    sqlx::query(
        "INSERT INTO event_group_roster(group_id, discord_id, added_by, added_at)
         VALUES ($1, $2, $3, $4::timestamptz)",
    )
    .bind(group)
    .bind(member)
    .bind(administrator)
    .bind(SEEDED_CREATION)
    .execute(&state.pool)
    .await
    .expect("add a roster entry");
}

fn administrator(actors: &Actors) -> String {
    actors.user(Role::Admin).discord_id.clone()
}

fn enlisted(actors: &Actors) -> String {
    actors.user(Role::Enlisted).discord_id.clone()
}

fn named_account_policy(discord_id: &str) -> String {
    json!({"grants": [{"conditions": [{"kind": "named_account", "discord_id": discord_id}]}]})
        .to_string()
}

fn any_signed_in_policy() -> Value {
    serde_json::from_str(ANY_SIGNED_IN).expect("the shared policy is JSON")
}

fn policy_change() -> Value {
    json!({"expected_access_revision": 0, "policy": any_signed_in_policy()})
}

fn revision_query() -> &'static str {
    "expected_access_revision=0"
}

fn reservation_quotas() -> Value {
    let opens_at = "2026-01-01T00:00:00Z";
    json!({"expected_access_revision": 0, "reservation_quotas": {
        "member": {"seats": null, "opens_at": opens_at},
        "guest": {"seats": 4, "opens_at": opens_at},
        "open": {"seats": 0, "opens_at": opens_at},
    }})
}

fn attachment_body(mission: Uuid) -> Value {
    json!({"mission_id": mission.to_string(), "start_time": FUTURE_START, "orbat": [{
        "faction": FACTION, "callsign": "A1", "squad": SQUAD,
        "slots": [{"role": "Squad Leader"}, {"role": "Rifleman"}],
    }]})
}

impl OperationsEventsWorld {
    /// A fresh event with its own attachment, seat and managed roster group.
    async fn fresh(&self, core: &WorldCore) -> FreshEvent {
        let administrator = administrator(&core.actors);
        let event = insert_event(&core.state, &administrator, ANY_SIGNED_IN).await;
        let (attachment, slot) = attach(&core.state, event, self.mission).await;
        let roster = json!({"kind": "managed_roster"});
        let group = insert_group(&core.state, event, &administrator, roster).await;
        FreshEvent {
            event,
            attachment,
            slot,
            group,
        }
    }

    /// A fresh event with nothing attached, for the attachment route.
    async fn bare_event(core: &WorldCore) -> Uuid {
        insert_event(&core.state, &administrator(&core.actors), ANY_SIGNED_IN).await
    }

    fn event_param(event: Uuid) -> Fixture {
        Fixture::new().param("id", event.to_string())
    }

    fn group_params(fresh: &FreshEvent) -> Fixture {
        Self::event_param(fresh.event).param("groupId", fresh.group.to_string())
    }

    fn squad_params(fresh: &FreshEvent) -> Fixture {
        Fixture::new()
            .param("emid", fresh.attachment.to_string())
            .param("faction", FACTION)
            .param("squad", SQUAD)
    }

    fn slot_params(fresh: &FreshEvent) -> Fixture {
        Fixture::new()
            .param("emid", fresh.attachment.to_string())
            .param("slotId", fresh.slot.to_string())
    }

    /// Fixtures of the event and attachment routes.
    async fn event_fixture(&self, core: &WorldCore, key: &str) -> Option<Fixture> {
        let fixture = match key {
            "POST /api/v1/events" => Fixture::new().body(json!({
                "start_time": FUTURE_START, "name_override": "Route acceptance operation",
                "briefing": "Hold the crossroads.", "max_slots": 24, "status": "open",
            })),
            "GET /api/v1/events/{id}"
            | "GET /api/v1/events/{id}/access"
            | "GET /api/v1/events/{id}/access/participants" => Self::event_param(self.event),
            "restricted-event" => Self::event_param(self.restricted_event),
            "PATCH /api/v1/events/{id}" => Self::event_param(self.fresh(core).await.event)
                .body(json!({"name_override": "Route acceptance renamed", "max_slots": 20})),
            "DELETE /api/v1/events/{id}" => Self::event_param(self.fresh(core).await.event),
            "deleted-event" => {
                let event = self.fresh(core).await.event;
                sqlx::query("UPDATE events SET deleted_at = now() WHERE id = $1")
                    .bind(event)
                    .execute(&core.state.pool)
                    .await
                    .expect("delete the event");
                Self::event_param(event)
            }
            "POST /api/v1/events/{id}/missions" => {
                Self::event_param(Self::bare_event(core).await).body(attachment_body(self.mission))
            }
            "archived-mission" => Self::event_param(Self::bare_event(core).await)
                .body(attachment_body(self.archived_mission)),
            "DELETE /api/v1/events/{id}/missions/{emid}" => {
                let fresh = self.fresh(core).await;
                Self::event_param(fresh.event).param("emid", fresh.attachment.to_string())
            }
            "attachment-of-another-event" => Self::event_param(self.fresh(core).await.event)
                .param("emid", self.attachment.to_string()),
            "GET /api/v1/event-missions/{emid}/orbat" => {
                Fixture::new().param("emid", self.attachment.to_string())
            }
            "restricted-orbat" => {
                Fixture::new().param("emid", self.restricted_attachment.to_string())
            }
            _ => return None,
        };
        Some(fixture)
    }

    /// Fixtures of the event access administration routes.
    async fn access_fixture(&self, core: &WorldCore, key: &str) -> Option<Fixture> {
        let administrator = administrator(&core.actors);
        let fixture = match key {
            "PUT /api/v1/events/{id}/access-policy" => {
                Self::event_param(self.fresh(core).await.event).body(policy_change())
            }
            "PUT /api/v1/events/{id}/reservation-quotas" => {
                Self::event_param(self.fresh(core).await.event).body(reservation_quotas())
            }
            "POST /api/v1/events/{id}/groups" => Self::event_param(self.fresh(core).await.event)
                .body(
                    json!({"expected_access_revision": 0, "name": "Route acceptance guests",
                    "source": {"kind": "managed_roster"}}),
                ),
            "PATCH /api/v1/events/{id}/groups/{groupId}" => {
                Self::group_params(&self.fresh(core).await)
                    .body(json!({"expected_access_revision": 0, "name": "Renamed roster"}))
            }
            "group-of-another-event" => {
                let other = self.fresh(core).await;
                Self::event_param(self.fresh(core).await.event)
                    .param("groupId", other.group.to_string())
                    .body(json!({"expected_access_revision": 0, "name": "Renamed roster"}))
            }
            "DELETE /api/v1/events/{id}/groups/{groupId}" => {
                Self::group_params(&self.fresh(core).await).query(revision_query())
            }
            "group-named-by-the-event-policy" => {
                let fresh = self.fresh(core).await;
                let policy = json!({"grants": [{"conditions": [
                    {"kind": "event_group", "group_id": fresh.group.to_string()}]}]});
                sqlx::query("UPDATE events SET access_policy = $2 WHERE id = $1")
                    .bind(fresh.event)
                    .bind(policy)
                    .execute(&core.state.pool)
                    .await
                    .expect("name the group in the event policy");
                Self::group_params(&fresh).query(revision_query())
            }
            "PUT /api/v1/events/{id}/groups/{groupId}/members/{discordId}" => {
                Self::group_params(&self.fresh(core).await)
                    .param("discordId", enlisted(&core.actors))
                    .body(json!({"expected_access_revision": 0}))
            }
            "partner-guild-group" => {
                let fresh = self.fresh(core).await;
                let source = json!({"kind": "partner_guild", "guild_id": "route-acceptance-partner",
                    "required_role_ids": []});
                let group = insert_group(&core.state, fresh.event, &administrator, source).await;
                Self::event_param(fresh.event)
                    .param("groupId", group.to_string())
                    .param("discordId", enlisted(&core.actors))
                    .body(json!({"expected_access_revision": 0}))
            }
            "DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}" => {
                let fresh = self.fresh(core).await;
                let member = enlisted(&core.actors);
                add_to_roster(&core.state, fresh.group, &member, &administrator).await;
                Self::group_params(&fresh)
                    .param("discordId", member)
                    .query(revision_query())
            }
            "account-not-on-the-roster" => Self::group_params(&self.fresh(core).await)
                .param("discordId", enlisted(&core.actors))
                .query(revision_query()),
            "PUT /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy" => {
                Self::squad_params(&self.fresh(core).await).body(policy_change())
            }
            "DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy" => {
                Self::squad_params(&self.fresh(core).await).query(revision_query())
            }
            "PUT /api/v1/event-missions/{emid}/slots/{slotId}/access-policy" => {
                Self::slot_params(&self.fresh(core).await).body(policy_change())
            }
            "DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy" => {
                Self::slot_params(&self.fresh(core).await).query(revision_query())
            }
            _ => return None,
        };
        Some(fixture)
    }

    /// Fixtures of the leave review routes.
    async fn leave_fixture(core: &WorldCore, key: &str) -> Option<Fixture> {
        match key {
            "PATCH /api/v1/admin/leave-requests/{id}" => {
                let request: Uuid = sqlx::query_scalar(
                    "INSERT INTO leave_requests(discord_id, starts_on, ends_on, reason, created_at)
                     VALUES ($1, '2026-10-01', '2026-10-03', 'Route acceptance', now())
                     RETURNING id",
                )
                .bind(enlisted(&core.actors))
                .fetch_one(&core.state.pool)
                .await
                .expect("file a leave request");
                Some(
                    Fixture::new()
                        .param("id", request.to_string())
                        .body(json!({"status": "approved"})),
                )
            }
            _ => None,
        }
    }
}

impl PartWorld for OperationsEventsWorld {
    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let administrator = administrator(actors);
        let mission = insert_mission(state, &administrator, "live").await;
        let archived_mission = insert_mission(state, &administrator, "archived").await;
        let event = insert_event(state, &administrator, ANY_SIGNED_IN).await;
        let (attachment, _) = attach(state, event, mission).await;
        let roster = json!({"kind": "managed_roster"});
        let group = insert_group(state, event, &administrator, roster).await;
        add_to_roster(state, group, &enlisted(actors), &administrator).await;
        register_participant(state, attachment, &enlisted(actors)).await;
        let only_enlisted = named_account_policy(&enlisted(actors));
        let restricted_event = insert_event(state, &administrator, &only_enlisted).await;
        let (restricted_attachment, _) = attach(state, restricted_event, mission).await;
        sqlx::query(
            "INSERT INTO leave_requests(discord_id, starts_on, ends_on, reason, created_at)
             VALUES ($1, '2026-11-02', '2026-11-04', 'Route acceptance', now())",
        )
        .bind(enlisted(actors))
        .execute(&state.pool)
        .await
        .expect("file the listed leave request");
        OperationsEventsWorld {
            event,
            attachment,
            restricted_event,
            restricted_attachment,
            mission,
            archived_mission,
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, _actor: Actor) -> Option<Fixture> {
        if let Some(fixture) = self.event_fixture(core, key).await {
            return Some(fixture);
        }
        if let Some(fixture) = self.access_fixture(core, key).await {
            return Some(fixture);
        }
        Self::leave_fixture(core, key).await
    }
}
