# Operations Domain Models (`operations/models/`)

Database and wire models for community operations, event missions, ORBAT slots, registrations, and saved mortar fire missions.

---

## 1. Model Catalog

### `Event` & `EventMission` (`event.rs`)
- **Postgres Tables**: `events`, `event_missions`
- **Fields (`Event`)**:
  - `id: Uuid` (Primary Key)
  - `name_override: String` (Custom operation name, trimmed, non-blank)
  - `start_time: DateTime<Utc>`
  - `briefing: String`
  - `banner_image_url: Option<String>`
  - `status: EventStatus` (`scheduled`, `open`, `locked`, `live`, `completed`)
  - `server_id: Option<Uuid>`
  - `modpack_id: Option<Uuid>`
  - `deleted_at: Option<DateTime<Utc>>`

### `OrbatSlot` & `OrbatReservation` (`event.rs`)
- **Postgres Tables**: `orbat_slots`, `orbat_reservations`
- **Fields (`OrbatSlot`)**:
  - `id: Uuid`
  - `event_mission_id: Uuid`
  - `faction_name: String`
  - `squad_name: String`
  - `role_name: String`
  - `assigned_to: Option<String>` (Discord snowflake of occupant)
  - `assigned_by: Option<String>` (Admin/leader snowflake)
  - `is_locked: bool`

### `EventRegistration` (`event.rs`)
- **Postgres Table**: `event_registrations`
- **Fields**:
  - `id: Uuid`
  - `event_mission_id: Uuid`
  - `discord_id: String`
  - `slot_id: Option<Uuid>`
  - `state: RegistrationState` (`signed_up`, `waitlisted`, `attended`, `no_show`)
  - `registered_at: DateTime<Utc>`

### `FireMission` (`fire_mission.rs` - Relocated from Admin!)
- **Postgres Table**: `fire_missions`
- **Fields (all 17 columns projected)**:
  - `id: Uuid`
  - `event_id: Option<Uuid>`
  - `created_by: String`
  - `weapon_system: String`
  - `fp_grid: String`
  - `target_grid: String`
  - `distance_m: i64`
  - `azimuth_deg: f64` (`numeric(5,1)` in DB, cast to float8)
  - `elevation_mils: i64`
  - `fp_x: Option<f64>` (Nullable, game-world coordinates)
  - `fp_y: Option<f64>`
  - `tgt_x: Option<f64>`
  - `tgt_y: Option<f64>`
  - `azimuth_mils: Option<i64>`
  - `charge: Option<i64>`
  - `time_of_flight_s: Option<f64>`
  - `created_at: DateTime<Utc>`

### `LeaveRequest` (`event.rs`)
- **Postgres Table**: `leave_requests`
- **Fields**: `id: Uuid`, `discord_id: String`, `start_date: DateTime<Utc>`, `end_date: DateTime<Utc>`, `reason: String`, `status: LeaveStatus` (`pending`, `approved`, `denied`).
