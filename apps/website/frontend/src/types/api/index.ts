// Hand-written API response types — the snake_case fields mirror the Go handler DTOs and
// GORM models (the API contract). Every exported type carries an @model (the GORM model it
// projects) or @contract (the schema it projects) so its source of truth is one grep away;
// this is enforced by verify-contract-citations.mjs (TS-6, DOCUMENTATION_STANDARDS §3).
import type { User } from '@/types/models/user'
import type { Server, ServerStatus } from '@/types/models/telemetry'

/**
 * A CMS announcement / news item.
 *
 * @model models.Announcement
 */
export interface Announcement {
  id: string
  title: string
  body: string
  snippet?: string
  tag: string
  thumbnail_url?: string
  author_id: string
  status: string
  is_pinned: boolean
  published_at?: string
  created_at: string
}

/**
 * The standard list-endpoint envelope: a page of `data` plus paging counters
 * (the platform-wide list contract; audit logs use a cursor instead).
 *
 * @typeParam T - the row type carried in `data`.
 */
export interface Paginated<T> {
  data: T[]
  total: number
  limit: number
  offset: number
}

/**
 * Aggregated landing-page payload: the caller's next event, current assignment, live
 * server status, current modpack, and recent announcements.
 *
 * @model models.Event
 */
export interface DashboardResponse {
  next_event?: {
    event_id: string
    name: string
    terrain: string
    start_time: string
    registered: number
    max_slots: number
    status: string
  } | null
  my_assignment?: {
    event_id: string
    name: string
    faction: string
    squad: string
    role: string
  } | null
  server_status?: ServerStatus | null
  current_modpack?: ModpackDTO | null
  recent_announcements: Announcement[]
}

/**
 * The auth session returned on login / refresh: token pair, expiry, the user, and whether
 * an Arma identity is linked.
 *
 * @model models.RefreshToken
 */
export interface AuthSession {
  access_token: string
  refresh_token: string
  expires_at: string
  user: User
  arma_linked: boolean
}

/**
 * One ranked row on a leaderboard; the stat fields populated depend on the category.
 *
 * @model models.MatchPlayerStat
 */
export interface LeaderboardRow {
  rank: number
  discord_id: string
  username: string
  avatar_url?: string
  kills?: number
  deaths?: number
  kd_ratio?: number
  team_kills?: number
  command_win_rate?: number
  missions_played?: number
  longest_kill_m?: number
}

/**
 * A leaderboard response: the requested category plus its ranked rows.
 *
 * @model models.MatchPlayerStat
 */
export interface LeaderboardResponse {
  category: string
  data: LeaderboardRow[]
}

/**
 * One mod entry within a modpack.
 *
 * @model models.ModpackMod
 */
export interface ModpackMod {
  id: string
  modpack_id: string
  name: string
  is_key_dependency: boolean
  sort_order: number
}

/**
 * A modpack the community runs (workshop bundle metadata).
 *
 * @model models.Modpack
 */
export interface Modpack {
  id: string
  name: string
  version: string
  total_size_bytes: number
  workshop_url?: string
  is_current: boolean
  created_at: string
}

/**
 * A modpack with its expanded mod list.
 *
 * @model models.Modpack
 */
export interface ModpackDTO extends Modpack {
  mods: ModpackMod[]
}

/**
 * A server with its live status and required modpack expanded (Server Intel page).
 *
 * @model models.Server
 */
export interface ServerIntel extends Server {
  status?: ServerStatus | null
  required_modpack?: ModpackDTO | null
}

/**
 * The My Deployments service record: totals, upcoming registrations, and past operations.
 *
 * @model models.EventRegistration
 */
export interface DeploymentsResponse {
  total_operations: number
  attendance_rate: number
  upcoming: {
    event_id: string
    event_mission_id: string
    name: string
    terrain: string
    start_time: string
    state: string
    faction?: string
    squad?: string
    role?: string
  }[]
  service_history: {
    date: string
    operation: string
    role: string
    outcome: string
    aar_replay_url?: string
  }[]
}

/**
 * A mission library list item (mirrors the Go `missionCard` DTO: the model plus
 * denormalized author + bookmark state).
 *
 * @model models.Mission
 */
export interface MissionCard {
  id: string
  title: string
  author_id: string
  terrain: string
  custom_terrain_name?: string
  game_mode: string
  weather: string
  time_of_day: string
  max_players: number
  status: string
  thumbnail_url?: string
  briefing?: string
  author_name: string
  author_avatar: string
  bookmarked: boolean
}

/**
 * One armory entry attached to a mission (faction-scoped item with an optional cap).
 *
 * @model models.MissionArmory
 */
export interface MissionArmory {
  id: string
  mission_id: string
  faction: string
  category: string
  item_name: string
  quantity?: number | null // null/omitted = unlimited
  icon?: string
  sort_order: number
}

/**
 * The full Mission Overview (mirrors the Go `missionDetail` DTO): a card plus armory and the
 * current version, whose `json_payload` is the editor superset.
 *
 * @model models.Mission
 */
export interface MissionDetail extends MissionCard {
  armory: MissionArmory[]
  current_version?: {
    id: string
    semver: string
    json_payload?: Record<string, unknown>
  } | null
}

/**
 * An event row in the schedule list (rollup counts for the card).
 *
 * @model models.Event
 */
export interface EventListItem {
  id: string
  name_override?: string
  start_time: string
  briefing?: string
  banner_image_url?: string
  status: string
  registration_locked: boolean
  max_slots: number
  mission_count: number
  registered: number
  filled: number
  total_slots: number
  percent: number
}

/**
 * One mission "dossier" inside an Event Hub (factions, armory-by-faction, fill, caller state).
 *
 * @model models.EventMission
 */
export interface EventMissionDossier {
  event_mission_id: string
  mission_id: string
  title: string
  terrain: string
  game_mode: string
  briefing?: string
  thumbnail_url?: string
  start_time: string
  factions: string[]
  armory_by_faction: { faction: string; items: MissionArmory[] }[]
  filled: number
  total: number
  my_state?: string
  my_slot_id?: string | null
}

/**
 * The Event Hub: an event container plus its nested mission dossiers.
 *
 * @model models.Event
 */
export interface EventHub {
  id: string
  name_override?: string
  start_time: string
  briefing?: string
  banner_image_url?: string
  status: string
  registration_locked: boolean
  max_slots: number
  missions: EventMissionDossier[]
}

/**
 * A single ORBAT slot row inside a squad (mirrors the Go `orbatSlotDTO`).
 *
 * @model models.OrbatSlot
 */
export interface OrbatSlot {
  id: string
  number: number // 1-based position within the squad
  role: string
  loadout?: string
  tag?: string
  slot_index: number
  assigned_to?: string | null
  assigned_name?: string
}

/**
 * A squad grouping of ORBAT slots (mirrors the Go `orbatSquadDTO`).
 *
 * @model models.OrbatSlot
 */
export interface OrbatSquad {
  faction: string
  callsign?: string
  squad: string
  filled: number
  total: number
  reserved_by?: string
  reserved_by_name?: string
  slots: OrbatSlot[]
}

/**
 * A slim member row for the leader's assignee picker (mirrors the Go `memberDTO`).
 *
 * @model models.User
 */
export interface Member {
  discord_id: string
  username: string
  avatar_url?: string
}

/**
 * A doctrine wiki page.
 *
 * @model models.WikiPage
 */
export interface WikiPage {
  id: string
  slug: string
  category: string
  title: string
  icon?: string
  body_md: string
  nav_order: number
  updated_at: string
}

/**
 * A vehicle database row.
 *
 * @model models.VehicleDatabase
 */
export interface VehicleRow {
  id: string
  name: string
  faction: string
  armor_type: string
  amphibious?: string
  primary_threat?: string
  profile_image_url?: string
}

/**
 * A mission pending approval in the admin review queue.
 *
 * @model models.Mission
 */
export interface ApprovalRow {
  mission_id: string
  title: string
  terrain: string
  author_id: string
  author_name: string
  submitted_at: string
}

/**
 * A personnel roster row (admin directory).
 *
 * @model models.User
 */
export interface RosterRow {
  discord_id: string
  username: string
  discord_handle: string
  arma_id?: string | null
  arma_character: string
  role: string
  is_banned: boolean
  warnings: number
}

/**
 * One audit-log entry.
 *
 * @model models.AuditLog
 */
export interface AuditLog {
  id: number
  severity: string
  actor_id?: string | null
  actor_name?: string
  action: string
  message: string
  created_at: string
}

/**
 * A page of audit logs, paged by an opaque cursor instead of offset.
 *
 * @model models.AuditLog
 */
export interface AuditLogResponse {
  data: AuditLog[]
  next_cursor?: number | null
}

/**
 * A computed mortar fire solution (mortar calculator output).
 *
 * @model models.FireMission
 */
export interface FireSolution {
  weapon_system: string
  distance_m: number
  azimuth_deg: number
  azimuth_mils: number
  elevation_mils: number
  charge: number
  time_of_flight_s: number
}

/**
 * Arma identity link status for the current user.
 *
 * @model models.IdentityLinkCode
 */
export interface LinkStatus {
  linked: boolean
  arma_id?: string | null
  arma_character?: string
  pending_code?: boolean // true = an unconsumed link code is outstanding
}

/**
 * The current user plus whether an Arma identity is linked (GET /api/v1/me).
 *
 * @model models.User
 */
export interface MeResponse {
  user: User
  arma_linked: boolean
}
