/**
 * Platform role, lowest → highest privilege (enlisted < leader < mission_maker < admin).
 *
 * @model models.User
 */
export type UserRole = 'enlisted' | 'leader' | 'mission_maker' | 'admin'

/**
 * Authenticated user identity plus service-record stats. The snake_case fields are the
 * API contract (from the Go GORM tags) — keep this interface in lockstep with the model.
 *
 * @model models.User
 */
export interface User {
  discord_id: string
  username: string
  discord_handle: string
  avatar_url: string
  arma_id?: string | null
  arma_character: string
  role: UserRole
  is_banned: boolean
  total_deployments: number
  attendance_rate: number
  created_at: string
  updated_at: string
}
