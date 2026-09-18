# Administration Domain Models (`administration/models/`)

Database and wire models for user accounts, role tiers, disciplinary warnings, and administrative audit logs.

---

## 1. Model Catalog

### `User` & `UserRole` (`user.rs`)
- **Postgres Table**: `users`
- **Postgres ENUM**: `user_role`
- **Role Hierarchy**:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type)]
  #[sqlx(type_name = "user_role", rename_all = "snake_case")]
  pub enum UserRole {
      Enlisted = 1,
      Leader = 2,
      MissionMaker = 3,
      Admin = 4,
  }
  ```
- **Fields**:
  - `discord_id: String` (Primary Key, Discord snowflake)
  - `username: String` (Display name)
  - `discord_handle: String` (Unique handle)
  - `avatar_url: Option<String>` (Discord CDN URL, validated via `is_cdn_path_segment`)
  - `role: UserRole` (Platform authorization tier)
  - `arma_id: Option<String>` (Linked Bohemia Interactive / Steam UID)
  - `arma_character: Option<String>` (Ingame player name)
  - `total_deployments: i64` (Denormalized count of attended missions)
  - `attendance_rate: f64` (Float projection of `numeric(5,2)`)
  - `is_banned: bool`
  - `ban_reason: Option<String>`
  - `banned_by: Option<String>`
  - `banned_at: Option<DateTime<Utc>>`
  - `created_at: DateTime<Utc>`
  - `updated_at: DateTime<Utc>`

### `Warning` (`user.rs`)
- **Postgres Table**: `warnings`
- **Fields**:
  - `id: Uuid` (Primary Key)
  - `discord_id: String` (Target user snowflake)
  - `issued_by: String` (Admin actor snowflake)
  - `reason: String` (Non-empty trimmed disciplinary reason)
  - `created_at: DateTime<Utc>`

### `DiscordRole` & `UserDiscordRole` (`user.rs`)
- **Postgres Tables**: `discord_roles`, `user_discord_roles`
- Maps external Discord guild role snowflakes to platform `mapped_role: Option<UserRole>` with integer `priority`.

### `AuditLog` & `AuditSeverity` (`audit.rs`)
- **Postgres Table**: `audit_logs`
- **Postgres ENUM**: `audit_severity` (`info`, `warn`, `crit`)
- **Fields**:
  - `id: i64` (Bigint sequence, used for keyset pagination `id < $1`)
  - `severity: AuditSeverity`
  - `actor_id: Option<String>`
  - `actor_name: String`
  - `action: String` (Hierarchical verb, e.g. `user.ban`, `server.rcon`)
  - `message: String`
  - `target_type: String`
  - `target_id: String`
  - `metadata: Option<RawJson>` (Nullable JSONB payload)
  - `created_at: DateTime<Utc>`

---

## 2. Invariants & Rules

1. **FireMission Relocation**:
   `FireMission` is **not** present in this domain. It was historically misplaced in legacy `models/admin.rs` and has been re-homed to `operations/models/fire_mission.rs`.
2. **Denormalized Columns**:
   `users.total_deployments` and `users.attendance_rate` are maintained by `command_center::services::user_stats`.
3. **Strict Serde Mapping**:
   Date serialization uses `go_time` formatting to maintain contract parity with existing frontend clients.
