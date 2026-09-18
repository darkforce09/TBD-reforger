# Missions Domain Models (`missions/models/`)

Database and wire models for scenarios, editor version payloads, virtual arsenal armories, item registries, and compatibility graphs.

---

## 1. Model Catalog

### `Mission` & `MissionVersion` (`mission.rs`)
- **Postgres Tables**: `missions`, `mission_versions`
- **Fields (`Mission`)**:
  - `id: Uuid` (Primary Key)
  - `title: String` (Trimmed non-blank)
  - `author_id: String` (Discord snowflake)
  - `terrain: TerrainType` (`everon`, `arland`, `custom`)
  - `custom_terrain_name: Option<String>`
  - `game_mode: GameMode` (`pve_coop`, `pvp`, `zeus`)
  - `weather: WeatherType` (`clear`, `overcast`, `heavy_rain`, `dense_fog`)
  - `time_of_day: String` (Formatted `HH:MM` or `HH:MM:SS`)
  - `briefing: String`
  - `thumbnail_url: Option<String>` (Validated HTTP URL)
  - `status: MissionStatus` (`draft`, `pending_approval`, `live`, `rejected`, `archived`)
  - `current_version_id: Option<Uuid>`
  - `rejection_reason: Option<String>`
  - `reviewed_by: Option<String>`
  - `reviewed_at: Option<DateTime<Utc>>`
  - `deleted_at: Option<DateTime<Utc>>`
- **Fields (`MissionVersion`)**:
  - `id: Uuid`
  - `mission_id: Uuid`
  - `semver: String` (SemVer 2.0.0, e.g. `0.1.0`)
  - `json_payload: RawJson` (Complete authored editor document)
  - `created_at: DateTime<Utc>`

### `MissionArmory` (`mission.rs`)
- **Postgres Table**: `mission_armories`
- **Fields**: `id: Uuid`, `mission_id: Uuid`, `faction: String`, `item_name: String`, `quantity: i64`, `sort_order: i64`.

### `RegistryItem` & `RegistryCompatEdge` (`registry.rs`)
- **Postgres Tables**: `registry_items`, `registry_compat`
- **Fields (`RegistryItem`)**: 22 fields including `resource_name`, `display_name`, `weight_kg`, `volume_cm3`, `max_weight_kg`, `max_volume_cm3`, `cargo_grid_w`, `cargo_grid_h`, `abstract_`, `variant_of`.
- **Fields (`RegistryCompatEdge`)**: `from_node`, `to_node`, `edge_type`, `qty`, `evidence`.

### `UserFaction` (`faction.rs`)
- **Postgres Table**: `user_factions`
- **Fields**: `id: Uuid`, `owner_id: String`, `side: String`, `name: String`, `doc: RawJson`.
