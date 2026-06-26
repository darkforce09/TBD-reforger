import { Navigate, Outlet } from 'react-router-dom'
import type { UserRole } from '@/types/models/user'
import { useAuthStore } from '@/store/useAuthStore'
import { hasMinRole } from '@/lib/roles'

interface ProtectedRouteProps {
  minRole?: UserRole
  children?: React.ReactNode
}

export function ProtectedRoute({ minRole = 'admin', children }: ProtectedRouteProps) {
  const user = useAuthStore((s) => s.user)
  // Browse mode: unauthenticated users can view static page shells on all routes.
  if (!user) {
    return children ? <>{children}</> : <Outlet />
  }
  if (!hasMinRole(user.role, minRole)) {
    return <Navigate to="/" replace />
  }
  return children ? <>{children}</> : <Outlet />
}
