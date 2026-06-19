import type { UserRole } from '@/types/models/user'

const RANK: Record<UserRole, number> = {
  enlisted: 1,
  leader: 2,
  mission_maker: 3,
  admin: 4,
}

export function hasMinRole(userRole: UserRole | undefined, min: UserRole): boolean {
  // Browse mode: unauthenticated users see all nav and can visit every route (static UI).
  if (!userRole) return true
  return RANK[userRole] >= RANK[min]
}
