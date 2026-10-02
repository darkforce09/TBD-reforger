//! The administrator routes of one operation's access: policies, groups, rosters and pools.
//!
//! **Role:** the path of every access administration route, and one call per route — the two reads
//! and the eleven changes.
//! **Position:** called by the event manager's access panel.
//! **Signals & state:** none.
//! **Invariants:** every change carries the access revision it was prepared against — in the body
//! of a `PUT`, `POST` or `PATCH`, and as the `expected_access_revision` query parameter of a
//! `DELETE`, which has no body — and answers the access view after it together with the
//! reservations it released or promoted. A stale revision answers `409` with the reason
//! `ACCESS_REVISION_CONFLICT`. Removing a squad's or a slot's policy makes it inherit again; it is
//! not the same change as saving a policy with no grants, which admits nobody.

use super::encode_path_segment as segment;

/// `GET /events/:id/access`: the operation's whole access configuration.
pub fn event_access_path(event_id: &str) -> String {
    format!("/events/{}/access", segment(event_id))
}

/// `GET /events/:id/access/participants`: the evidence behind each participant's eligibility.
pub fn access_participants_path(event_id: &str) -> String {
    format!("/events/{}/access/participants", segment(event_id))
}

/// `PUT /events/:id/access-policy`: the operation's own policy.
pub fn event_access_policy_path(event_id: &str) -> String {
    format!("/events/{}/access-policy", segment(event_id))
}

/// `PUT` and `DELETE /event-missions/:emid/squads/:faction/:squad/access-policy`.
pub fn squad_access_policy_path(event_mission_id: &str, faction: &str, squad: &str) -> String {
    format!(
        "/event-missions/{}/squads/{}/{}/access-policy",
        segment(event_mission_id),
        segment(faction),
        segment(squad)
    )
}

/// `PUT` and `DELETE /event-missions/:emid/slots/:slotId/access-policy`.
pub fn slot_access_policy_path(event_mission_id: &str, slot_id: &str) -> String {
    format!(
        "/event-missions/{}/slots/{}/access-policy",
        segment(event_mission_id),
        segment(slot_id)
    )
}

/// `PUT /events/:id/reservation-quotas`: the three pools, replaced together.
pub fn reservation_quotas_path(event_id: &str) -> String {
    format!("/events/{}/reservation-quotas", segment(event_id))
}

/// `POST /events/:id/groups`: create an event group.
pub fn event_groups_path(event_id: &str) -> String {
    format!("/events/{}/groups", segment(event_id))
}

/// `PATCH` and `DELETE /events/:id/groups/:groupId`.
pub fn event_group_path(event_id: &str, group_id: &str) -> String {
    format!("/events/{}/groups/{}", segment(event_id), segment(group_id))
}

/// `PUT` and `DELETE /events/:id/groups/:groupId/members/:discordId`: one managed-roster entry.
pub fn event_group_member_path(event_id: &str, group_id: &str, discord_id: &str) -> String {
    format!(
        "/events/{}/groups/{}/members/{}",
        segment(event_id),
        segment(group_id),
        segment(discord_id)
    )
}

/// `path` with the revision precondition a `DELETE` carries in place of a body.
pub fn with_expected_revision(path: &str, access_revision: i64) -> String {
    format!("{path}?expected_access_revision={access_revision}")
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{
        api_delete_keeping_refusal, api_get, api_patch_keeping_refusal, api_post_keeping_refusal,
        api_put_keeping_refusal, ApiErr, ApiRefusal,
    };
    use crate::v2::core::api::dto::{
        AccessChangeOutcome, AccessPolicyChange, AccessRevisionPrecondition,
        EventAccessAdministration, EventGroupChange, EventGroupCreation,
        ParticipantAccessExplanation, ReservationQuotaChange,
    };
    use crate::v2::core::api::endpoints::json_body;
    use crate::v2::core::auth::AuthStore;

    /// Read the operation's whole access configuration.
    pub async fn load_event_access(
        store: AuthStore,
        event_id: &str,
    ) -> Result<EventAccessAdministration, ApiErr> {
        api_get(store, &event_access_path(event_id)).await
    }

    /// Read the evidence behind each participant's eligibility.
    pub async fn load_access_participants(
        store: AuthStore,
        event_id: &str,
    ) -> Result<Vec<ParticipantAccessExplanation>, ApiErr> {
        api_get(store, &access_participants_path(event_id)).await
    }

    /// Replace the operation's own policy.
    pub async fn put_event_access_policy(
        store: AuthStore,
        event_id: &str,
        change: &AccessPolicyChange,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_access_policy_path(event_id);
        api_put_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Give a squad an explicit policy of its own.
    pub async fn put_squad_access_policy(
        store: AuthStore,
        event_mission_id: &str,
        faction: &str,
        squad: &str,
        change: &AccessPolicyChange,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = squad_access_policy_path(event_mission_id, faction, squad);
        api_put_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Remove a squad's explicit policy, so it inherits the operation's again.
    pub async fn remove_squad_access_policy(
        store: AuthStore,
        event_mission_id: &str,
        faction: &str,
        squad: &str,
        access_revision: i64,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = squad_access_policy_path(event_mission_id, faction, squad);
        api_delete_keeping_refusal(store, &with_expected_revision(&path, access_revision)).await
    }

    /// Give a slot an explicit policy of its own.
    pub async fn put_slot_access_policy(
        store: AuthStore,
        event_mission_id: &str,
        slot_id: &str,
        change: &AccessPolicyChange,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = slot_access_policy_path(event_mission_id, slot_id);
        api_put_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Remove a slot's explicit policy, so it inherits its squad's, or the operation's, again.
    pub async fn remove_slot_access_policy(
        store: AuthStore,
        event_mission_id: &str,
        slot_id: &str,
        access_revision: i64,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = slot_access_policy_path(event_mission_id, slot_id);
        api_delete_keeping_refusal(store, &with_expected_revision(&path, access_revision)).await
    }

    /// Replace the three reservation pools together.
    pub async fn put_reservation_quotas(
        store: AuthStore,
        event_id: &str,
        change: &ReservationQuotaChange,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = reservation_quotas_path(event_id);
        api_put_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Create an event group; the backend answers `201` with the access view after it.
    pub async fn create_event_group(
        store: AuthStore,
        event_id: &str,
        creation: &EventGroupCreation,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_groups_path(event_id);
        api_post_keeping_refusal(store, &path, json_body(creation)?).await
    }

    /// Rename a group or change its source.
    pub async fn change_event_group(
        store: AuthStore,
        event_id: &str,
        group_id: &str,
        change: &EventGroupChange,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_group_path(event_id, group_id);
        api_patch_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Remove a group. A group some policy still names is refused with `409`.
    pub async fn remove_event_group(
        store: AuthStore,
        event_id: &str,
        group_id: &str,
        access_revision: i64,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_group_path(event_id, group_id);
        api_delete_keeping_refusal(store, &with_expected_revision(&path, access_revision)).await
    }

    /// Add an account to a managed roster.
    pub async fn add_event_group_member(
        store: AuthStore,
        event_id: &str,
        group_id: &str,
        discord_id: &str,
        access_revision: i64,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_group_member_path(event_id, group_id, discord_id);
        let body = AccessRevisionPrecondition {
            expected_access_revision: access_revision,
        };
        api_put_keeping_refusal(store, &path, json_body(&body)?).await
    }

    /// Remove an account from a managed roster.
    pub async fn remove_event_group_member(
        store: AuthStore,
        event_id: &str,
        group_id: &str,
        discord_id: &str,
        access_revision: i64,
    ) -> Result<AccessChangeOutcome, ApiRefusal> {
        let path = event_group_member_path(event_id, group_id, discord_id);
        api_delete_keeping_refusal(store, &with_expected_revision(&path, access_revision)).await
    }
}
