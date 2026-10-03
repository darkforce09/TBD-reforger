//! Every identifier is its inner value on the wire, in text and in SQL.
//!
//! **Role:** proves, per declared id, that the JSON, the `Display` / `FromStr` text and the
//! Postgres type of the newtype equal those of the bare `Uuid`, `String` or `i64`.
//! **Position:** unit tests of the crate root.
//! **Signals & state:** none.
//! **Invariants:** a newtype that changed any of the three would change a wire shape, a golden or
//! a SQL bind, so each case compares against the bare value, never against a literal of its own.

use sqlx::postgres::{PgHasArrayType, Postgres};
use uuid::Uuid;

use crate::prelude::*;

const SAMPLE_UUID: &str = "67e55044-10b1-426f-9247-bb680e5fe0c8";

/// Asserts the JSON, text and SQL identity of one UUID id type.
fn assert_uuid_identity<Id>()
where
    Id: From<Uuid>
        + Into<Uuid>
        + Copy
        + PartialEq
        + std::fmt::Debug
        + std::fmt::Display
        + std::str::FromStr
        + serde::Serialize
        + serde::de::DeserializeOwned
        + sqlx::Type<Postgres>
        + PgHasArrayType,
    <Id as std::str::FromStr>::Err: std::fmt::Debug,
{
    let uuid: Uuid = SAMPLE_UUID.parse().unwrap();
    let id = Id::from(uuid);

    let id_json = serde_json::to_string(&id).unwrap();
    assert_eq!(id_json, serde_json::to_string(&uuid).unwrap());
    let decoded: Id = serde_json::from_str(&id_json).unwrap();
    assert_eq!(decoded, id);
    assert_eq!(
        serde_json::from_value::<Id>(serde_json::json!(uuid)).unwrap(),
        id
    );

    assert_eq!(id.to_string(), uuid.to_string());
    assert_eq!(SAMPLE_UUID.parse::<Id>().unwrap(), id);
    assert!("not-a-uuid".parse::<Id>().is_err());
    assert_eq!(Into::<Uuid>::into(id), uuid);

    assert_eq!(
        <Id as sqlx::Type<Postgres>>::type_info(),
        <Uuid as sqlx::Type<Postgres>>::type_info()
    );
    assert_eq!(
        <Id as PgHasArrayType>::array_type_info(),
        <Uuid as PgHasArrayType>::array_type_info()
    );
}

macro_rules! uuid_identity_tests {
    ($($test:ident => $id:ty;)*) => {
        $(
            #[test]
            fn $test() {
                assert_uuid_identity::<$id>();
            }
        )*
    };
}

uuid_identity_tests! {
    event_id_is_its_uuid => EventId;
    event_mission_id_is_its_uuid => EventMissionId;
    orbat_slot_id_is_its_uuid => OrbatSlotId;
    orbat_reservation_id_is_its_uuid => OrbatReservationId;
    event_registration_id_is_its_uuid => EventRegistrationId;
    event_group_id_is_its_uuid => EventGroupId;
    event_participant_allocation_id_is_its_uuid => EventParticipantAllocationId;
    leave_request_id_is_its_uuid => LeaveRequestId;
    live_slot_occupancy_id_is_its_uuid => LiveSlotOccupancyId;
    fire_mission_id_is_its_uuid => FireMissionId;
    mission_id_is_its_uuid => MissionId;
    mission_version_id_is_its_uuid => MissionVersionId;
    mission_artifact_id_is_its_uuid => MissionArtifactId;
    mission_review_id_is_its_uuid => MissionReviewId;
    mission_review_comment_id_is_its_uuid => MissionReviewCommentId;
    mission_deployment_id_is_its_uuid => MissionDeploymentId;
    mission_armory_id_is_its_uuid => MissionArmoryId;
    registry_item_id_is_its_uuid => RegistryItemId;
    registry_compatibility_id_is_its_uuid => RegistryCompatibilityId;
    user_faction_id_is_its_uuid => UserFactionId;
    match_id_is_its_uuid => MatchId;
    match_player_stat_id_is_its_uuid => MatchPlayerStatId;
    server_id_is_its_uuid => ServerId;
    machine_credential_id_is_its_uuid => MachineCredentialId;
    runtime_session_id_is_its_uuid => RuntimeSessionId;
    fleet_command_id_is_its_uuid => FleetCommandId;
    announcement_id_is_its_uuid => AnnouncementId;
    modpack_id_is_its_uuid => ModpackId;
    modpack_mod_id_is_its_uuid => ModpackModId;
    vehicle_database_id_is_its_uuid => VehicleDatabaseId;
    wiki_page_id_is_its_uuid => WikiPageId;
    authentication_session_id_is_its_uuid => AuthenticationSessionId;
    refresh_token_id_is_its_uuid => RefreshTokenId;
    warning_id_is_its_uuid => WarningId;
}

const SAMPLE_TEXT: &str = "123456789012345678";

/// Asserts the JSON and text identity of one string id type.
fn assert_string_text_identity<Id>()
where
    Id: From<String>
        + for<'a> From<&'a str>
        + Into<String>
        + Clone
        + PartialEq
        + for<'a> PartialEq<&'a str>
        + std::fmt::Debug
        + std::fmt::Display
        + std::str::FromStr
        + serde::Serialize
        + serde::de::DeserializeOwned,
    <Id as std::str::FromStr>::Err: std::fmt::Debug,
{
    let id = Id::from(SAMPLE_TEXT);

    let id_json = serde_json::to_string(&id).unwrap();
    assert_eq!(id_json, serde_json::to_string(SAMPLE_TEXT).unwrap());
    assert_eq!(serde_json::from_str::<Id>(&id_json).unwrap(), id);
    assert_eq!(serde_json::from_str::<Id>("\"\"").unwrap(), Id::from(""));

    assert_eq!(id.to_string(), SAMPLE_TEXT);
    assert_eq!(SAMPLE_TEXT.parse::<Id>().unwrap(), id);
    assert_eq!(Into::<String>::into(id.clone()), SAMPLE_TEXT);
    assert_eq!(Id::from(SAMPLE_TEXT.to_string()), id);
    assert!(id == SAMPLE_TEXT);
}

/// Asserts the JSON, text and SQL identity of one database-bound string id type.
fn assert_string_identity<Id>()
where
    Id: From<String>
        + for<'a> From<&'a str>
        + Into<String>
        + Clone
        + PartialEq
        + for<'a> PartialEq<&'a str>
        + std::fmt::Debug
        + std::fmt::Display
        + std::str::FromStr
        + serde::Serialize
        + serde::de::DeserializeOwned
        + sqlx::Type<Postgres>
        + PgHasArrayType,
    <Id as std::str::FromStr>::Err: std::fmt::Debug,
{
    assert_string_text_identity::<Id>();
    assert_eq!(
        <Id as sqlx::Type<Postgres>>::type_info(),
        <String as sqlx::Type<Postgres>>::type_info()
    );
    assert_eq!(
        <Id as PgHasArrayType>::array_type_info(),
        <String as PgHasArrayType>::array_type_info()
    );
}

/// Asserts the JSON, text and SQL identity of one `i64` id type.
fn assert_integer_identity<Id>()
where
    Id: From<i64>
        + Into<i64>
        + Copy
        + PartialEq
        + std::fmt::Debug
        + std::fmt::Display
        + std::str::FromStr
        + serde::Serialize
        + serde::de::DeserializeOwned
        + sqlx::Type<Postgres>
        + PgHasArrayType,
    <Id as std::str::FromStr>::Err: std::fmt::Debug,
{
    let value: i64 = 9_007_199_254_740_993;
    let id = Id::from(value);

    let id_json = serde_json::to_string(&id).unwrap();
    assert_eq!(id_json, serde_json::to_string(&value).unwrap());
    assert_eq!(serde_json::from_str::<Id>(&id_json).unwrap(), id);

    assert_eq!(id.to_string(), value.to_string());
    assert_eq!(value.to_string().parse::<Id>().unwrap(), id);
    assert!("1.5".parse::<Id>().is_err());
    assert_eq!(Into::<i64>::into(id), value);

    assert_eq!(
        <Id as sqlx::Type<Postgres>>::type_info(),
        <i64 as sqlx::Type<Postgres>>::type_info()
    );
    assert_eq!(
        <Id as PgHasArrayType>::array_type_info(),
        <i64 as PgHasArrayType>::array_type_info()
    );
}

macro_rules! identity_tests {
    ($check:ident: $($test:ident => $id:ty;)*) => {
        $(
            #[test]
            fn $test() {
                $check::<$id>();
            }
        )*
    };
}

identity_tests! { assert_string_identity:
    discord_user_id_is_its_string => DiscordUserId;
    discord_guild_id_is_its_string => DiscordGuildId;
    discord_role_id_is_its_string => DiscordRoleId;
    discord_message_id_is_its_string => DiscordMessageId;
    arma_player_id_is_its_string => ArmaPlayerId;
    audit_target_id_is_its_string => AuditTargetId;
    workshop_item_id_is_its_string => WorkshopItemId;
    mod_guid_is_its_string => ModGuid;
    scenario_id_is_its_string => ScenarioId;
    source_match_id_is_its_string => SourceMatchId;
    source_event_id_is_its_string => SourceEventId;
    match_event_id_is_its_string => MatchEventId;
    mission_slot_uid_is_its_string => MissionSlotUid;
    player_life_id_is_its_string => PlayerLifeId;
    ballistics_catalog_id_is_its_string => BallisticsCatalogId;
    ballistics_weapon_id_is_its_string => BallisticsWeaponId;
    ballistics_shell_id_is_its_string => BallisticsShellId;
    equipment_generation_id_is_its_string => EquipmentGenerationId;
    equipment_resource_id_is_its_string => EquipmentResourceId;
    equipment_node_id_is_its_string => EquipmentNodeId;
    equipment_native_instance_id_is_its_string => EquipmentNativeInstanceId;
}

identity_tests! { assert_string_text_identity:
    discord_client_id_is_its_text => DiscordClientId;
    loadout_modpack_id_is_its_text => LoadoutModpackId;
    submitted_mission_id_is_its_text => SubmittedMissionId;
    submitted_event_id_is_its_text => SubmittedEventId;
}

identity_tests! { assert_integer_identity:
    audit_log_entry_id_is_its_integer => AuditLogEntryId;
    server_status_sample_id_is_its_integer => ServerStatusSampleId;
    equipment_field_id_is_its_integer => EquipmentFieldId;
}

#[test]
fn optional_text_ids_default_to_the_empty_text_they_skip() {
    assert!(AuditTargetId::default().is_empty());
    assert!(WorkshopItemId::default().is_empty());
    assert!(ModGuid::default().is_empty());
    assert!(DiscordMessageId::default().is_empty());
    assert_eq!(AuditTargetId::default(), "");
    assert!(!AuditTargetId::from("events").is_empty());
    assert!(!WorkshopItemId::from("5965550F24A0C152").is_empty());
    assert!(!ModGuid::from("5965550F24A0C152").is_empty());
    assert!(!DiscordMessageId::from(SAMPLE_TEXT).is_empty());
}
