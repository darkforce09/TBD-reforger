import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { User, UserRole } from '@/types/models/user'
import type { AuthSession } from '@/types/api'
import { hasMinRole } from '@/lib/roles'

interface AuthState {
  accessToken: string | null
  refreshToken: string | null
  expiresAt: string | null
  user: User | null
  bootstrapping: boolean
  setBootstrapping: (v: boolean) => void
  setSession: (session: AuthSession) => void
  setAccessToken: (token: string, expiresAt: string) => void
  clearSession: () => void
  isAuthenticated: () => boolean
  hasMinRole: (role: UserRole) => boolean
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      accessToken: null,
      refreshToken: null,
      expiresAt: null,
      user: null,
      bootstrapping: false,
      setBootstrapping: (bootstrapping) => set({ bootstrapping }),
      setSession: (session) =>
        set({
          accessToken: session.access_token,
          refreshToken: session.refresh_token,
          expiresAt: session.expires_at,
          user: session.user,
          bootstrapping: false,
        }),
      setAccessToken: (token, expiresAt) => set({ accessToken: token, expiresAt }),
      clearSession: () =>
        set({
          accessToken: null,
          refreshToken: null,
          expiresAt: null,
          user: null,
          bootstrapping: false,
        }),
      isAuthenticated: () => Boolean(get().accessToken && get().user),
      hasMinRole: (role) => hasMinRole(get().user?.role, role),
    }),
    {
      name: 'tbd-auth',
      partialize: (s) => ({
        refreshToken: s.refreshToken,
        user: s.user,
        expiresAt: s.expiresAt,
      }),
    },
  ),
)
