# TBD Reforger Platform — Backend Architecture Plan

**Author:** Lead Backend Engineer (handoff design)
**Date:** 2026-06-17
**Scope:** PostgreSQL schema + Go REST API for the TBD Arma Reforger Event Platform.
**Sources:** `Claude_Context_Handoff.md` + the 21 Stitch page mockups in `frontend/src/stitch-exports/`.

> **Status:** Original design plan. Schema may pre-date T-008 campaign refactor — verify against
> [`internal/models/`](../../internal/models/). Current status: [`CLAUDE.md`](../../CLAUDE.md).
> Stitch exports are **archived** — live UI: `frontend/src/pages`.

---

## 0. Architectural Principles

1. **Discord is the identity root.** No local passwords. `discord_id` (snowflake, `BIGINT`/`TEXT`) is the canonical user key. JWT (access + refresh) issued after Discord OAuth2 exchange.
2. **Two-tier permission model.** Roles are *synced* from Discord but cached and authoritative in our DB so the API never has to round-trip to Discord on every request. A nightly + on-login + webhook-driven sync reconciles them.
3. **Relational core + JSONB documents.** Structured data (users, events, stats) is normalized. The mission "map payload" (spawns, vehicles, markers from the 2D editor) is a large opaque `JSONB` document — queryable but not normalized.
4. **Telemetry is append-only & idempotent.** Server ingest endpoints are authenticated with a service token (not a user JWT), accept batches, and dedupe on `(match_id, source_event_id)`.
5. **Soft-delete + audit everything.** Admin mutations write an `audit_logs` row. User/mission/event records use `deleted_at` rather than hard deletes.
6. **UUID primary keys** for all internal entities (`gen_random_uuid()` via `pgcrypto`), *except* `users` whose natural PK is `discord_id`. This keeps URLs non-enumerable.

**Stack:** Go (chi or gin router), `pgx` + `sqlc` for type-safe queries, `golang-migrate` for migrations, `lib/pq` LISTEN/NOTIFY (or Redis pub/sub) for the live audit/server-FPS feed via SSE/WebSocket.

---

## 1. PostgreSQL Schema

> Conventions: `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`, `updated_at` maintained by trigger. All FKs `ON DELETE` behavior noted per table. Enums implemented as Postgres `ENUM` types for stability.

### 1.1 Enum Types

```sql
CREATE TYPE user_role        AS ENUM ('enlisted','mission_maker','admin');
CREATE TYPE mission_status   AS ENUM ('draft','pending_approval','live','rejected','archived');
CREATE TYPE terrain_type     AS ENUM ('everon','arland','custom');
CREATE TYPE game_mode        AS ENUM ('pve_coop','pvp','zeus');
CREATE TYPE weather_type     AS ENUM ('clear','overcast','heavy_rain','dense_fog');
CREATE TYPE event_status     AS ENUM ('scheduled','open','locked','live','completed','cancelled');
CREATE TYPE registration_state AS ENUM ('registered','waitlisted','withdrawn','attended','no_show');
CREATE TYPE mission_outcome  AS ENUM ('success','failure','aborted','pending');
CREATE TYPE announcement_status AS ENUM ('draft','published','archived');
CREATE TYPE announcement_tag  AS ENUM ('update','event','modpack_update','important');
CREATE TYPE audit_severity   AS ENUM ('info','warn','crit');
CREATE TYPE leave_status     AS ENUM ('pending','approved','denied');
```

### 1.2 `users` — identity, roles, service record headline stats

Drives: TopBar (linked pill, avatar, username), Personnel Roster, My Deployments header, ORBAT assignment.

```sql
CREATE TABLE users (
    discord_id        TEXT PRIMARY KEY,                 -- Discord snowflake
    username          TEXT NOT NULL,                    -- "Admin Dave"
    discord_handle    TEXT,                             -- "Dave#1234" (legacy/displayed in roster)
    avatar_url        TEXT,
    arma_id           TEXT UNIQUE,                      -- Enfusion/Steam ID, NULL until linked
    arma_character    TEXT,                             -- "[TBD] Admin Dave" shown in roster
    role              user_role NOT NULL DEFAULT 'enlisted',
    is_banned         BOOLEAN NOT NULL DEFAULT false,
    ban_reason        TEXT,
    banned_by         TEXT REFERENCES users(discord_id),
    banned_at         TIMESTAMPTZ,
    -- denormalized headline metrics (recomputed from attendance + telemetry)
    total_deployments INT NOT NULL DEFAULT 0,           -- "Total Operations 42"
    attendance_rate   NUMERIC(5,2) NOT NULL DEFAULT 0,  -- "94%"
    last_login_at     TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
CREATE INDEX idx_users_role ON users(role) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_arma_id ON users(arma_id);
```

### 1.3 `discord_roles` + `user_discord_roles` — role sync source-of-truth

Lets us map *any* Discord role → web permission, supporting the "@Admin / @MissionMaker → web permissions" requirement without hardcoding.

```sql
CREATE TABLE discord_roles (
    discord_role_id   TEXT PRIMARY KEY,                 -- snowflake
    name              TEXT NOT NULL,                    -- "MissionMaker"
    mapped_role       user_role,                        -- which web role it grants (nullable = cosmetic)
    priority          INT NOT NULL DEFAULT 0            -- highest priority wins on conflict
);
CREATE TABLE user_discord_roles (
    discord_id        TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    discord_role_id   TEXT NOT NULL REFERENCES discord_roles(discord_role_id) ON DELETE CASCADE,
    synced_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (discord_id, discord_role_id)
);
```

### 1.4 `identity_link_codes` — the 6-digit Arma linking flow

Drives the "Link Arma Identity" flow (`POST /api/me/link`) and the 6-digit JetBrains-Mono input from the design system.

```sql
CREATE TABLE identity_link_codes (
    code        CHAR(6) PRIMARY KEY,                    -- one-time, JetBrains-mono input
    discord_id  TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    arma_id     TEXT,                                   -- filled when in-game mod confirms
    consumed_at TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,                   -- short TTL (~10 min)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_link_codes_discord ON identity_link_codes(discord_id) WHERE consumed_at IS NULL;
```

### 1.5 `missions` + `mission_versions` — Mission Library / Creator / Overview

Drives: Mission Library cards, Mission Creator wizard, Mission Overview, Mission Approvals. Versioning (`v1.2.0` seen in Overview) handled by a child table; `missions` points at the current live version.

```sql
CREATE TABLE missions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title             TEXT NOT NULL,                    -- "Operation Enduring Freedom"
    author_id         TEXT NOT NULL REFERENCES users(discord_id),
    terrain           terrain_type NOT NULL,
    custom_terrain_name TEXT,                           -- when terrain='custom' ("Custom Map")
    game_mode         game_mode NOT NULL,               -- pve_coop / pvp / zeus
    weather           weather_type NOT NULL DEFAULT 'clear',
    time_of_day       TIME NOT NULL DEFAULT '14:00',    -- insertion time slider
    max_players       INT NOT NULL,                     -- "64 Players max"
    status            mission_status NOT NULL DEFAULT 'draft',
    thumbnail_url     TEXT,
    briefing          TEXT,                             -- long-form briefing markdown (Overview)
    current_version_id UUID,                            -- FK set after first version (see ALTER)
    rejection_reason  TEXT,
    reviewed_by       TEXT REFERENCES users(discord_id),
    reviewed_at       TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);

CREATE TABLE mission_versions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id    UUID NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    semver        TEXT NOT NULL,                        -- "1.2.0"
    json_payload  JSONB NOT NULL,                       -- spawns, vehicles, map markers (2D editor)
    editor_notes  TEXT,
    created_by    TEXT NOT NULL REFERENCES users(discord_id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (mission_id, semver)
);
ALTER TABLE missions
    ADD CONSTRAINT fk_current_version
    FOREIGN KEY (current_version_id) REFERENCES mission_versions(id);

CREATE INDEX idx_missions_status   ON missions(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_missions_terrain  ON missions(terrain);
CREATE INDEX idx_missions_mode     ON missions(game_mode);
CREATE INDEX idx_missions_author   ON missions(author_id);
-- JSONB GIN index for querying marker/vehicle contents if needed:
CREATE INDEX idx_mission_payload_gin ON mission_versions USING GIN (json_payload);
```

### 1.6 `mission_armory` — Armory section of Mission Overview

The Overview page lists weapons/vehicles per faction with quantities ("M16A2 Rifle x45 Available"). Normalized rather than buried in JSON so the UI can render it as a list cheaply.

```sql
CREATE TABLE mission_armory (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id  UUID NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    faction     TEXT NOT NULL,                          -- "US Forces" / "USSR" / "FIA"
    category    TEXT NOT NULL,                          -- weapon / vehicle / equipment
    item_name   TEXT NOT NULL,                          -- "M16A2 Rifle"
    quantity    INT,                                    -- 45 (NULL = unlimited)
    icon        TEXT,                                   -- material symbol name
    sort_order  INT NOT NULL DEFAULT 0
);
CREATE INDEX idx_armory_mission ON mission_armory(mission_id);
```

### 1.7 `mission_bookmarks` — "Bookmarked" tab in Mission Library

```sql
CREATE TABLE mission_bookmarks (
    discord_id  TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    mission_id  UUID NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (discord_id, mission_id)
);
```

### 1.8 `events` — Event Manager / Upcoming Operations / Server Intel "Active Deployment"

```sql
CREATE TABLE events (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id        UUID NOT NULL REFERENCES missions(id),
    name_override     TEXT,                             -- usually derived from mission title
    start_time        TIMESTAMPTZ NOT NULL,             -- "Oct 28, 20:00 EST"
    status            event_status NOT NULL DEFAULT 'scheduled',
    registration_locked BOOLEAN NOT NULL DEFAULT false, -- "Registration Status: Open/Locked"
    max_slots         INT NOT NULL,                     -- denormalized capacity ("/60")
    created_by        TEXT NOT NULL REFERENCES users(discord_id),
    -- post-match linkage
    match_id          UUID,                             -- FK to matches once played (see ALTER)
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
CREATE INDEX idx_events_start  ON events(start_time);
CREATE INDEX idx_events_status ON events(status) WHERE deleted_at IS NULL;
```

### 1.9 `orbat_slots` + `event_registrations` — ORBAT & registration

The ORBAT (Order of Battle) is the squad/slot tree ("Platoon HQ 2/2", "Alpha 1-1 8/8", role "Combat Medic"). A *template* lives with the mission JSON, but the *filled* slots are per-event. Registrations also drive "40/60 • 66% OPEN", waitlist, and the My Deployments "ASSIGNED SLOT" badge.

```sql
CREATE TABLE orbat_slots (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    faction     TEXT NOT NULL,                          -- "US Army"
    squad       TEXT NOT NULL,                          -- "Alpha 1-1"
    callsign    TEXT,                                   -- optional grouping ("Platoon HQ")
    role        TEXT NOT NULL,                          -- "Combat Medic", "Rifleman", "Squad Leader"
    slot_index  INT NOT NULL,                           -- position within squad
    assigned_to TEXT REFERENCES users(discord_id) ON DELETE SET NULL,
    assigned_at TIMESTAMPTZ,
    UNIQUE (event_id, squad, slot_index)
);
CREATE INDEX idx_orbat_event    ON orbat_slots(event_id);
CREATE INDEX idx_orbat_assignee ON orbat_slots(assigned_to);

CREATE TABLE event_registrations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    discord_id  TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    slot_id     UUID REFERENCES orbat_slots(id) ON DELETE SET NULL,
    state       registration_state NOT NULL DEFAULT 'registered',
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (event_id, discord_id)
);
CREATE INDEX idx_reg_event ON event_registrations(event_id);
CREATE INDEX idx_reg_user  ON event_registrations(discord_id);
```

### 1.10 `leave_requests` — "Submit Leave of Absence (LOA)"

```sql
CREATE TABLE leave_requests (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_id  TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    starts_on   DATE NOT NULL,
    ends_on     DATE NOT NULL,
    reason      TEXT,
    status      leave_status NOT NULL DEFAULT 'pending',
    reviewed_by TEXT REFERENCES users(discord_id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.11 `matches` + `match_player_stats` — telemetry / leaderboards / AAR

Post-match telemetry ingest. `matches` = one completed operation instance; `match_player_stats` = per-player line items that feed the Global Leaderboards and the My Deployments service record (role, outcome, AAR replay).

```sql
CREATE TABLE matches (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id      UUID REFERENCES events(id),           -- nullable (unscheduled/PvP scrims)
    mission_id    UUID REFERENCES missions(id),
    terrain       terrain_type,
    started_at    TIMESTAMPTZ NOT NULL,
    ended_at      TIMESTAMPTZ,
    outcome       mission_outcome NOT NULL DEFAULT 'pending',
    winning_faction TEXT,
    aar_replay_url TEXT,                                -- "View Replay"
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE events ADD CONSTRAINT fk_event_match
    FOREIGN KEY (match_id) REFERENCES matches(id);

CREATE TABLE match_player_stats (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id            UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    discord_id          TEXT REFERENCES users(discord_id),  -- resolved via arma_id join
    arma_id             TEXT NOT NULL,                       -- raw ingest key
    role_played         TEXT,                                -- "Alpha 1-2 (Rifleman)"
    kills               INT NOT NULL DEFAULT 0,
    deaths              INT NOT NULL DEFAULT 0,
    team_kills          INT NOT NULL DEFAULT 0,              -- Wall of Shame
    longest_kill_m      INT NOT NULL DEFAULT 0,              -- "Longest Kill" board
    vehicles_destroyed  INT NOT NULL DEFAULT 0,
    is_command          BOOLEAN NOT NULL DEFAULT false,      -- played Platoon HQ
    command_win         BOOLEAN,                             -- Command Win Rate numerator
    source_event_id     TEXT NOT NULL,                       -- idempotency key from server
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (match_id, arma_id, source_event_id)
);
CREATE INDEX idx_mps_user  ON match_player_stats(discord_id);
CREATE INDEX idx_mps_match ON match_player_stats(match_id);
```

**Leaderboard strategy:** aggregate via a `MATERIALIZED VIEW leaderboard_totals` (sum kills/deaths/tk, max longest_kill, count missions, command win rate) refreshed on telemetry ingest, indexed per category. The 5 tabs (`K/D Ratio`, `Command Win Rate`, `Missions Played`, `Longest Kill`, `Wall of Shame`) are `ORDER BY` variants over this view.

```sql
CREATE MATERIALIZED VIEW leaderboard_totals AS
SELECT
    s.discord_id,
    SUM(s.kills)               AS kills,
    SUM(s.deaths)              AS deaths,
    CASE WHEN SUM(s.deaths)=0 THEN SUM(s.kills)
         ELSE ROUND(SUM(s.kills)::numeric / SUM(s.deaths), 2) END AS kd_ratio,
    SUM(s.team_kills)          AS team_kills,
    MAX(s.longest_kill_m)      AS longest_kill_m,
    SUM(s.vehicles_destroyed)  AS vehicles_destroyed,
    COUNT(DISTINCT s.match_id) AS missions_played,
    COUNT(*) FILTER (WHERE s.command_win)                                   AS command_wins,
    NULLIF(COUNT(*) FILTER (WHERE s.is_command),0)                          AS command_games
FROM match_player_stats s
WHERE s.discord_id IS NOT NULL
GROUP BY s.discord_id;
CREATE UNIQUE INDEX ON leaderboard_totals(discord_id);
```

### 1.12 Live server telemetry — `server_status` (current) + `server_status_history`

Server Intel Column 1 (uptime, player count, FPS) + Dashboard "Online / 45/64". Current state is a single hot row per server; history is time-series for the audit "FPS dropped below 20" alert.

```sql
CREATE TABLE servers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    ip              INET NOT NULL,
    port            INT NOT NULL,
    required_modpack_id UUID,                            -- FK to modpacks
    is_active       BOOLEAN NOT NULL DEFAULT true
);
CREATE TABLE server_status (
    server_id       UUID PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    is_online       BOOLEAN NOT NULL DEFAULT false,
    player_count    INT NOT NULL DEFAULT 0,
    max_players     INT NOT NULL DEFAULT 64,
    server_fps      NUMERIC(5,1) NOT NULL DEFAULT 0,     -- "44 Server FPS (Healthy)"
    uptime_seconds  BIGINT NOT NULL DEFAULT 0,           -- render as "02:14:33"
    current_match_id UUID REFERENCES matches(id),
    ingame_time     TEXT,                                -- "14:30 Local"
    ingame_weather  TEXT,                                -- "Overcast / Light Rain"
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE server_status_history (
    id           BIGGENERATED ALWAYS AS IDENTITY,       -- (see note) time-series
    server_id    UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    player_count INT NOT NULL,
    server_fps   NUMERIC(5,1) NOT NULL,
    recorded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_status_hist ON server_status_history(server_id, recorded_at DESC);
```
> Note: `server_status_history` is a partition/retention candidate (drop >30d). Use `BIGINT GENERATED ALWAYS AS IDENTITY` for the PK.

### 1.13 `modpacks` + `modpack_mods` — Modpacks page & Server Intel "Required Modpack"

```sql
CREATE TABLE modpacks (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT NOT NULL,                         -- "Core Modern Expansion"
    version       TEXT NOT NULL,                         -- "2.1"
    total_size_bytes BIGINT NOT NULL,                    -- render "45.2 GB"
    workshop_url  TEXT,                                  -- "View Collection in Reforger Workshop"
    is_current    BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE modpack_mods (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    modpack_id  UUID NOT NULL REFERENCES modpacks(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,                           -- "RHS: Status Quo", "TFAR", "ACE3"
    is_key_dependency BOOLEAN NOT NULL DEFAULT false,
    sort_order  INT NOT NULL DEFAULT 0
);
```

### 1.14 `announcements` — Announcements feed + Content Manager + Dashboard

```sql
CREATE TABLE announcements (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title         TEXT NOT NULL,
    body          TEXT NOT NULL,                         -- rich text / markdown HTML
    snippet       TEXT,                                  -- derived preview
    tag           announcement_tag NOT NULL DEFAULT 'update',
    thumbnail_url TEXT,
    author_id     TEXT NOT NULL REFERENCES users(discord_id),
    status        announcement_status NOT NULL DEFAULT 'draft',
    is_pinned     BOOLEAN NOT NULL DEFAULT false,        -- "PINNED"
    pushed_to_discord BOOLEAN NOT NULL DEFAULT false,    -- webhook toggle result
    discord_message_id TEXT,                             -- for edit/delete sync
    published_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at    TIMESTAMPTZ
);
CREATE INDEX idx_ann_published ON announcements(published_at DESC)
    WHERE status='published' AND deleted_at IS NULL;
```

### 1.15 `wiki_pages` — SOPs & Manuals

Left-nav categories + markdown content + the vehicle DB tables (stored as markdown tables or structured JSON). Versioned for the "Edit Wiki Page" CMS tab.

```sql
CREATE TABLE wiki_pages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT UNIQUE NOT NULL,                    -- "vehicle-database-iff"
    category    TEXT NOT NULL,                           -- "Vehicle Database & IFF"
    title       TEXT NOT NULL,
    icon        TEXT,                                    -- material symbol ("local_shipping")
    body_md     TEXT NOT NULL,                           -- markdown w/ tables + callouts
    nav_order   INT NOT NULL DEFAULT 0,
    updated_by  TEXT REFERENCES users(discord_id),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Optional structured vehicle table for the IFF database (instead of markdown):
CREATE TABLE vehicle_database (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT NOT NULL,                         -- "BTR-70"
    faction       TEXT NOT NULL,                         -- "USSR"
    armor_type    TEXT NOT NULL,                         -- "Light Armored"
    amphibious    TEXT,                                  -- "Yes (5km/h)" / "No"
    primary_threat TEXT,                                 -- "Heavy MG / Infantry"
    profile_image_url TEXT
);
```

### 1.16 `warnings` — Personnel Roster "Warnings" column

```sql
CREATE TABLE warnings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_id  TEXT NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    issued_by   TEXT NOT NULL REFERENCES users(discord_id),
    reason      TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_warnings_user ON warnings(discord_id);
-- roster "Warnings" count = COUNT(*) per discord_id
```

### 1.17 `audit_logs` — Audit Logs console

```sql
CREATE TABLE audit_logs (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    severity    audit_severity NOT NULL DEFAULT 'info',  -- INFO / WARN / CRIT
    actor_id    TEXT REFERENCES users(discord_id),       -- NULL for system events
    actor_name  TEXT,                                    -- denormalized ("Admin Dave")
    action      TEXT NOT NULL,                           -- "mission.approve"
    message     TEXT NOT NULL,                           -- rendered line
    target_type TEXT,                                    -- "mission" / "user" / "webhook"
    target_id   TEXT,
    metadata    JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_created  ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_severity ON audit_logs(severity);
```

### 1.18 `fire_missions` (optional) — Mortar Calculator

The calculator is mostly client-side trig, but persisting saved firing positions / target plots is useful for shared tactical planning.

```sql
CREATE TABLE fire_missions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id      UUID REFERENCES events(id) ON DELETE CASCADE,
    created_by    TEXT NOT NULL REFERENCES users(discord_id),
    weapon_system TEXT NOT NULL,                         -- "M252 81mm"
    fp_grid       TEXT NOT NULL,                         -- firing position grid ("FP Alpha")
    target_grid   TEXT NOT NULL,                         -- "TGT 001"
    distance_m    INT NOT NULL,
    azimuth_deg   NUMERIC(5,1) NOT NULL,
    elevation_mils INT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### Schema relationship summary

```
users ─< event_registrations >─ events ─> missions ─< mission_versions
  │           │                    │           │            (json_payload)
  │           └─ orbat_slots ──────┘           ├─< mission_armory
  │                                            └─< mission_bookmarks >─ users
  ├─< warnings                    events ─> matches ─< match_player_stats ─> leaderboard_totals (MV)
  ├─< leave_requests              servers ─> server_status / _history
  ├─ user_discord_roles >─ discord_roles    modpacks ─< modpack_mods
  ├─< announcements (author)      wiki_pages / vehicle_database
  └─ identity_link_codes          audit_logs / fire_missions
```

---

## 2. Go REST API

**Base:** `/api/v1`. **Auth:** `Authorization: Bearer <JWT>` (user) or `X-Service-Token` (game server ingest). **Errors:** RFC-7807 problem+json. **Pagination:** `?limit=&cursor=` (keyset) returning `{data, next_cursor}`.

### Middleware chain
`RequestID → Logger → Recover → CORS → RateLimit → AuthN(JWT/ServiceToken) → AuthZ(role) → AuditWriter(mutations)`

Authorization helpers: `RequireAuth`, `RequireRole(mission_maker)`, `RequireRole(admin)`, `RequireServiceToken`.

### 2.1 Auth & Identity

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET  | `/auth/discord/login` | public | Redirect to Discord OAuth2 consent |
| GET  | `/auth/discord/callback` | public | Exchange code → upsert user, sync roles, issue JWT pair |
| POST | `/auth/refresh` | refresh token | Rotate access token |
| POST | `/auth/logout` | user | Revoke refresh token |
| GET  | `/me` | user | Current profile: role, arma link status, headline stats (TopBar) |
| PATCH| `/me` | user | Update self settings |
| POST | `/me/link` | user | **(existing)** Generate/confirm 6-digit Arma link code |
| GET  | `/me/link/status` | user | Poll link status (in-game confirmation) |
| DELETE | `/me/link` | user | Unlink Arma identity |

### 2.2 Server Intel & Telemetry

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/servers` | user | List servers + summary |
| GET | `/servers/:id/status` | user | Server Intel card (uptime, players, FPS, current op, IP/port, modpack) |
| GET | `/servers/:id/status/stream` | user | SSE/WebSocket live FPS+player feed |
| POST | `/ingest/server-status` | **service** | Game server pushes live status (upserts `server_status`, appends history) |
| POST | `/ingest/match-results` | **service** | Post-match batch → `matches` + `match_player_stats`, refresh leaderboard MV (idempotent on `source_event_id`) |
| POST | `/ingest/link-confirm` | **service** | In-game mod confirms a 6-digit code → sets `arma_id` |

### 2.3 Announcements & Dashboard

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/announcements` | user | Published feed (pinned first), paginated |
| GET | `/announcements/:id` | user | Full briefing ("Read Full Briefing") |
| GET | `/dashboard` | user | Aggregate: next event countdown, server status, modpack sync, my assignment, recent announcements |

### 2.4 Mission Library / Creator / Overview / Approvals

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/missions` | user | Library w/ filters `?terrain=&mode=&player_count=&scope=global\|mine\|bookmarked&q=` |
| POST | `/missions` | mission_maker | Mission Creator wizard → create `draft` + initial version |
| GET | `/missions/:id` | user | Overview: briefing, armory, ORBAT template, version, command actions |
| PATCH | `/missions/:id` | author/admin | Update metadata |
| POST | `/missions/:id/versions` | author/admin | Save new `json_payload` version from 2D editor |
| GET | `/missions/:id/versions/:vid` | user | Fetch specific version payload |
| GET | `/missions/:id/export` | mission_maker | Export strict `mission.json` (Mission Injection) |
| POST | `/missions/:id/submit` | author | Move `draft → pending_approval` |
| POST | `/missions/:id/bookmark` / DELETE | user | Toggle bookmark |
| GET | `/missions/:id/armory` | user | Armory listing |
| GET | `/approvals` | admin | Pending-approval queue (Mission Approvals table) |
| POST | `/approvals/:id/approve` | admin | `pending → live` (writes audit log) |
| POST | `/approvals/:id/reject` | admin | `pending → rejected` + reason |

### 2.5 Events / ORBAT / My Deployments

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/events` | user | Upcoming Operations (list + calendar) w/ registration counts, status |
| POST | `/events` | admin | Event Manager → schedule operation (date/time/mission/lock) |
| GET | `/events/:id` | user | Event detail incl. registration totals |
| PATCH | `/events/:id` | admin | Edit time / lock / status |
| DELETE | `/events/:id` | admin | Cancel/delete event |
| GET | `/events/:id/orbat` | user | Full ORBAT tree w/ assigned players (filled/empty slots) |
| POST | `/events/:id/register` | user | Register / request slot → `registered` or `waitlisted` |
| DELETE | `/events/:id/register` | user | Withdraw |
| PUT | `/events/:id/slots/:slotId/assign` | admin | Assign/reassign a user to an ORBAT slot |
| DELETE | `/events/:id/slots/:slotId/assign` | admin | Clear slot |
| GET | `/me/deployments` | user | My Deployments: upcoming (assigned slot badge) + service history (role, outcome, AAR) |
| POST | `/me/leave-requests` | user | Submit LOA |
| GET | `/me/leave-requests` | user | List own LOAs |

### 2.6 Leaderboards

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/leaderboards?category=kd\|command_win\|missions\|longest_kill\|team_kills&q=&limit=` | user | Sorted board (podium top-3 + ranks 4–50), searchable |
| GET | `/users/:discordId/stats` | user | Individual stat card |

### 2.7 SOPs / Wiki / Modpacks

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/wiki` | user | Nav tree (categories) |
| GET | `/wiki/:slug` | user | Page content (markdown) |
| PUT | `/wiki/:slug` | admin | Content Manager "Edit Wiki Page" |
| GET | `/wiki/vehicles` | user | Vehicle DB / IFF table |
| GET | `/modpacks` | user | Modpacks page (size, mod list, workshop link) |
| GET | `/modpacks/current` | user | Required modpack for Server Intel verify pill |

### 2.8 Content Manager (CMS)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/cms/announcements` | admin | Create announcement (draft or publish; `push_to_discord` flag) |
| PATCH | `/cms/announcements/:id` | admin | Edit / pin / publish |
| DELETE | `/cms/announcements/:id` | admin | Archive |
| POST | `/cms/uploads` | admin | Thumbnail drag-drop (JPG/PNG/WEBP ≤5MB) → returns URL (S3/MinIO) |
| POST | `/cms/announcements/:id/push-discord` | admin | Manually (re)push webhook embed |

### 2.9 Admin — Personnel / Server Control / Audit

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/admin/users?q=` | admin | Personnel Roster (discord, arma, role, warnings count) |
| PATCH | `/admin/users/:discordId` | admin | Edit role / rank |
| POST | `/admin/users/:discordId/ban` | admin | Ban (+reason → audit) |
| DELETE | `/admin/users/:discordId/ban` | admin | Unban |
| POST | `/admin/users/:discordId/warnings` | admin | Issue warning |
| POST | `/admin/roles/sync` | admin | Force Discord role resync |
| POST | `/admin/servers/:id/rcon` | admin | RCON command: `{action: restart\|change_map, ...}` |
| GET | `/admin/audit-logs?severity=&q=&cursor=` | admin | Audit console (paginated) |
| GET | `/admin/audit-logs/stream` | admin | Live feed (SSE) |
| GET | `/admin/audit-logs/export.csv` | admin | "Export to CSV" |
| GET | `/admin/leave-requests` | admin | Review/approve LOAs |

### 2.10 Mortar / Fire Missions (optional)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/fire-missions` | user | Save a firing solution |
| GET | `/events/:id/fire-missions` | user | List shared plots for an op |

---

## 3. Cross-Cutting Concerns

- **Discord webhook push:** on announcement publish with `push_to_discord=true`, enqueue a job → build embed (title, snippet, thumbnail, tag) → POST webhook → store `discord_message_id`. Failures log a `CRIT` audit row (matches the "Webhook failed to push payload" log line).
- **Role sync:** runs on (a) every OAuth login, (b) a nightly cron, (c) optional Discord gateway event. Resolves highest-priority `mapped_role` from `user_discord_roles`.
- **JWT claims:** `sub=discord_id`, `role`, `arma_linked` bool, `exp`. Short access TTL (15m) + rotating refresh.
- **Idempotent ingest:** `match_player_stats` UNIQUE `(match_id, arma_id, source_event_id)`; status pushes upsert by `server_id`.
- **Leaderboard refresh:** `REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals` after each match ingest (debounced).
- **Derived metrics:** `users.total_deployments` / `attendance_rate` recomputed from `event_registrations` (attended vs scheduled) on match close.
- **Live feeds:** server status + audit log streams via SSE backed by Postgres `LISTEN/NOTIFY` (or Redis).
- **Storage:** thumbnails/AAR replays in object storage (S3/MinIO); DB holds URLs only.

---

## 4. Build Order (suggested milestones)

1. **M1 – Identity:** `users`, Discord OAuth, JWT, role sync, `/me`, `/me/link`.
2. **M2 – Content read paths:** announcements, wiki, modpacks, dashboard, server intel (static).
3. **M3 – Missions:** library, creator, versions, armory, approvals.
4. **M4 – Events & ORBAT:** scheduling, registration, slot assignment, my deployments, LOA.
5. **M5 – Telemetry:** server-status + match ingest, leaderboards MV, live SSE.
6. **M6 – Admin:** roster, bans/warnings, RCON, audit logs + CSV, CMS webhook push.
7. **M7 – Field tools:** mortar/fire-missions, mission.json export + injection.
```
