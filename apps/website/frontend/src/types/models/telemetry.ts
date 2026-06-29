/**
 * Live server telemetry snapshot (player count, FPS, current match, in-game time/weather).
 *
 * @model models.ServerStatus
 */
export interface ServerStatus {
  server_id: string
  is_online: boolean
  player_count: number
  max_players: number
  server_fps: number
  uptime_seconds: number
  current_match_id?: string | null
  ingame_time?: string
  ingame_weather?: string
  updated_at: string
}

/**
 * A registered game server (connection info + active flag).
 *
 * @model models.Server
 */
export interface Server {
  id: string
  name: string
  ip: string
  port: number
  required_modpack_id?: string | null
  is_active: boolean
}
