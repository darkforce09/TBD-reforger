export type UserRole = 'enlisted' | 'leader' | 'mission_maker' | 'admin'

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
