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

export interface Server {
  id: string
  name: string
  ip: string
  port: number
  required_modpack_id?: string | null
  is_active: boolean
}
