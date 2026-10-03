# API identifiers source

The source of `api_identifiers`: one module per owning domain declaring that domain's ids, the
crate root that re-exports them, the prelude and the identity tests.

## Contents

```text
crates/api/api_identifiers/src/
├── administration.rs         `WarningId`, `AuditTargetId` (text), `AuditLogEntryId` (integer)
├── ballistics.rs             `BallisticsCatalogId`, `BallisticsWeaponId`, `BallisticsShellId` (text)
├── community_content.rs      `AnnouncementId`, `ModpackId`, `ModpackModId`, `VehicleDatabaseId`, `WikiPageId`, `WorkshopItemId` and `ModGuid` (text)
├── discord.rs                `DiscordUserId`, `DiscordGuildId`, `DiscordRoleId`, `DiscordMessageId`, `DiscordClientId` (text)
├── equipment_datasets.rs     the generation, resource, node and native instance ids (text), `EquipmentFieldId` (integer)
├── identity_and_access.rs    `AuthenticationSessionId`, `RefreshTokenId`, `ArmaPlayerId` (text)
├── lib.rs                    the crate root: module header, `mod` lines and the re-exports
├── match_telemetry.rs        `MatchId`, `MatchPlayerStatId`, `SourceMatchId`, `SourceEventId` and `MatchEventId` (text)
├── missions.rs               `MissionId`, `MissionVersionId`, `MissionArtifactId`, the review, deployment, armory, registry and faction ids; `MissionSlotUid`, `LoadoutModpackId`, `SubmittedMissionId` (text)
├── operations.rs             `EventId`, `EventMissionId`, `OrbatSlotId`, the reservation, registration, group, allocation, leave, occupancy and fire-mission ids; `PlayerLifeId`, `SubmittedEventId` (text)
├── prelude.rs                every id for glob import
├── server_infrastructure.rs  `ServerId`, `MachineCredentialId`, `RuntimeSessionId`, `FleetCommandId`, `ScenarioId` (text), `ServerStatusSampleId` (integer)
└── tests/                    the JSON, text and Postgres type identity of every id
```

## How it works

Each domain module is private and holds only `newtype_ids::uuid_id!` / `string_id!` /
`integer_id!` declarations, with the `sqlx,` arm for every id bound to SQL, and the `is_empty` and
`From` helpers of the optional text ids; `lib.rs` re-exports every id at the crate root and
`prelude.rs` re-exports them again for glob import. `tests/wire_identity.rs` runs one generic check
per id: the UUID, text, SQL-free text and integer identities, and the empty default of the
optional text ids.

## Boundaries

- Depends on: `newtype_ids`, sqlx.
- Used by: callers through the crate root or `prelude`.
- Rules: the `sqlx::Type` derive resolves against this crate's own sqlx dependency, so the crate
  keeps the `postgres`, `uuid` and `macros` features.
