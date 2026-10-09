# API identifiers

The `api_identifiers` crate: one typed identifier per id concept the
[API](/documentation/glossary/a_to_f.md#api) passes across a public boundary, so an event id, a
mission id, a server id and a Discord user id are distinct types the compiler keeps apart while
their JSON, their text and their SQL binds stay those of the bare `Uuid`, `String` or `i64`.

## Contents

```text
crates/api/api_identifiers/
├── Cargo.toml  the package: `newtype_ids` and sqlx (`postgres`, `uuid`, `macros`), layout tier 1
└── src/        the ids grouped by owning domain, the prelude and the identity tests
```

## How it works

Each id is declared with a `newtype_ids` macro: `uuid_id!` for the UUID key of a table,
`string_id!` for a text key (a Discord snowflake, an Arma player id, a slug or GUID, a key the game
runtime chooses) and `integer_id!` for a `BIGINT` key. The `sqlx,` arm adds
`#[derive(sqlx::Type)] #[sqlx(transparent)]`; every declaration derives a transparent serde
`Serialize` / `Deserialize`, so:

- the JSON of an id is the JSON of its inner value, and a response golden or a stored digest that
  holds one is byte-equal to the one the bare value produced;
- an id binds and decodes as its inner Postgres type (`UUID`, `TEXT`, `BIGINT`, and their arrays),
  so the SQL text and the `sqlx::FromRow` rows that carry ids are unchanged;
- an axum `Path<EventId>` or `Query` field parses exactly what `Path<Uuid>` parsed.

The ids are `Copy` (UUID, integer) or `Clone` (text), compare, order and hash as their inner
value, and convert both ways with `From`. One concept has one type everywhere: a plain `id` field
takes the type of the record it identifies, and a foreign key such as `event_mission_id` takes the
type of the table it references.

Three kinds of text id carry more than a key:

- **Optional text ids** (`AuditTargetId`, `WorkshopItemId`, `ModGuid`, `DiscordUserId`,
  `DiscordMessageId`) derive `Default` as the empty text and have `is_empty`, for the rows and
  wire fields where `""` means "none".
- **Submitted ids** (`SubmittedEventId`, `SubmittedMissionId`) carry a client's text unparsed: the
  handler parses it into `EventId` / `MissionId` and answers its own refusal message, so a blank
  or malformed value is refused with the contract's message rather than a deserialisation error.
- **Text-only ids** (`DiscordClientId`, `LoadoutModpackId` and the submitted ids) are never bound
  to SQL and are declared without the `sqlx,` arm.

| Module | Ids |
|---|---|
| `operations` | `EventId`, `EventMissionId`, `OrbatSlotId`, `OrbatReservationId`, `EventRegistrationId`, `EventGroupId`, `EventParticipantAllocationId`, `LeaveRequestId`, `LiveSlotOccupancyId`, `FireMissionId`; text: `PlayerLifeId`, `SubmittedEventId` |
| `missions` | `MissionId`, `MissionVersionId`, `MissionArtifactId`, `MissionReviewId`, `MissionReviewCommentId`, `MissionDeploymentId`, `MissionArmoryId`, `RegistryItemId`, `RegistryCompatibilityId`, `UserFactionId`; text: `MissionSlotUid`, `LoadoutModpackId`, `SubmittedMissionId` |
| `match_telemetry` | `MatchId`, `MatchPlayerStatId`; text: `SourceMatchId`, `SourceEventId`, `MatchEventId` |
| `server_infrastructure` | `ServerId`, `MachineCredentialId`, `RuntimeSessionId`, `FleetCommandId`; text: `ScenarioId`; integer: `ServerStatusSampleId` |
| `community_content` | `AnnouncementId`, `ModpackId`, `ModpackModId`, `VehicleDatabaseId`, `WikiPageId`; text: `WorkshopItemId`, `ModGuid` |
| `identity_and_access` | `AuthenticationSessionId`, `RefreshTokenId`; text: `ArmaPlayerId` |
| `administration` | `WarningId`; text: `AuditTargetId`; integer: `AuditLogEntryId` |
| `discord` | text: `DiscordUserId`, `DiscordGuildId`, `DiscordRoleId`, `DiscordMessageId`, `DiscordClientId` |
| `ballistics` | text: `BallisticsCatalogId`, `BallisticsWeaponId`, `BallisticsShellId` |
| `equipment_datasets` | text: `EquipmentGenerationId`, `EquipmentResourceId`, `EquipmentNodeId`, `EquipmentNativeInstanceId`; integer: `EquipmentFieldId` |

## Getting started

Run from the repository root:

```bash
cargo test -p api_identifiers   # the JSON, text and Postgres type identity of every id
```

To add an id, declare it with its `///` doc naming the table it keys in the module of the domain
that owns the table, export it from `lib.rs` and `prelude.rs`, and add its line to the identity
tests in `src/tests/wire_identity.rs`.

## Configuration

No features and no environment variables.

## Public surface

- Every id of the table above, at the crate root and in `prelude`.
- Each UUID id has `new`, `as_uuid`, `into_inner`, `Display`, `FromStr`, `From<Uuid>` and
  `From<Id> for Uuid`.
- Each text id has `new`, `as_str`, `into_inner`, `Display`, `FromStr`, `From<String>`,
  `From<&str>`, `From<Id> for String`, `Borrow<str>`, `AsRef<str>` and `PartialEq<str>`; the
  optional text ids add `Default` and `is_empty`.
- Each integer id has `new`, `get`, `into_inner`, `Display`, `FromStr` and `From` both ways with
  its `i64`.
- `AuditTargetId` also converts from `&String` and from `&DiscordUserId`, so every audit writer
  takes its target as `impl Into<AuditTargetId>`.

## Boundaries

- Depends on: `newtype_ids` (the macros); sqlx (the `Type` derive, `postgres` and `uuid`).
- Used by: `api_configuration` (the Discord client and guild ids); the API (`apps/api`): every
  domain crate, the kernel crates (sessions included), `api_background_workers`, the API's
  binaries, and the integration tests under `apps/api/tests`.
- Rules: every id is serde-transparent and every SQL-bound id sqlx-transparent
  (`src/tests/wire_identity.rs` compares each with its bare value: JSON, text, and for the SQL-bound
  ids the Postgres type and array type); an API crate's public `id` / `*_id`
  field or function parameter takes an id from this crate, never a bare primitive
  (`cargo xtask verify crate-anatomy`); API category, so it depends on foundation crates only
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Newtype identifiers](/crates/foundation/newtype_ids/README.md) — the macros the ids are
  declared with.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md#55-crate-anatomy-cargo-xtask-verify-crate-anatomy)
  — the crate-anatomy law on public id fields.
