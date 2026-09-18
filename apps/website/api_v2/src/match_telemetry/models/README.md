# Match Telemetry Models (`match_telemetry/models/`)

Data definitions, binary wire payloads, and database entities for real-time in-game telemetry ticks, combat events, player session tokens, and After-Action Review (AAR) playback streams.

---

## 1. Domain Entities & Schemas

### `telemetry_tick.rs` (<350 LOC)
- **Database Table**: `telemetry_ticks` (partitioned by session)
- **Fields**:
  - `session_id: Uuid`: Match session reference.
  - `timestamp_ms: u64`: Relative match elapsed milliseconds.
  - `player_id: Uuid`: Registered player ID or guest token.
  - `position_x: f32`, `position_y: f32`, `position_z: f32`: World coordinates in meters.
  - `azimuth: f32`: Compass heading (0..360 degrees).
  - `is_alive: bool`: Life state.
  - `vehicle_id: Option<Uuid>`: Entity ID if mounted inside a vehicle.

### `combat_event.rs` (<320 LOC)
- **Database Table**: `combat_events`
- **Fields**:
  - `id: Uuid`: Unique event identifier.
  - `session_id: Uuid`: Match session reference.
  - `event_type: CombatEventType`: `Kill`, `Wound`, `FriendlyFire`, `VehicleDestroyed`, `ObjectiveCaptured`.
  - `instigator_id: Option<Uuid>`: Attacker player identity.
  - `victim_id: Option<Uuid>`: Target player identity.
  - `weapon_prefab: Option<String>`: Weapon or projectile GUID used.
  - `distance_meters: Option<f32>`: Engagement distance.
  - `occurred_at_ms: u64`: Relative session timeline offset.

### `session_token.rs` (<220 LOC)
- **Database Table**: `session_tokens`
- **Fields**:
  - `token: String`: Cryptographically secure random reservation token.
  - `discord_id: String`: Discord user identifier.
  - `event_mission_id: Uuid`: Target operation mission.
  - `assigned_slot_id: Uuid`: Pre-reserved ORBAT slot.
  - `expires_at: DateTime<Utc>`: Ephemeral token TTL (default 15 minutes).

### `aar_replay.rs` (<300 LOC)
- **Wire Payloads**:
  - `ReplayManifest`: Session metadata, duration, map name, participant roster.
  - `ReplayKeyframe`: Full-state snapshot emitted every N seconds.
  - `ReplayDeltaFrame`: Vector difference encoding position and state deltas between keyframes.
