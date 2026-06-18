import { useEffect } from 'react'
import { api } from '@/api/client'
import { refreshSession } from '@/api/refresh'
import { useAuthStore } from '@/store/useAuthStore'
import type { MeResponse } from '@/types/api'

/** Restore access token from persisted refresh token on app load. */
export function useAuthBootstrap() {
  const accessToken = useAuthStore((s) => s.accessToken)
  const refreshToken = useAuthStore((s) => s.refreshToken)
  const setSession = useAuthStore((s) => s.setSession)
  const setBootstrapping = useAuthStore((s) => s.setBootstrapping)
  const clearSession = useAuthStore((s) => s.clearSession)

  useEffect(() => {
    if (accessToken) return
    if (!refreshToken) return

    let cancelled = false
    setBootstrapping(true)

    refreshSession()
      .then(async (data) => {
        if (cancelled) return
        // Fetch the profile with the fresh token passed explicitly. We must NOT
        // mutate the store (e.g. setAccessToken) before setSession: accessToken is
        // an effect dependency, so changing it mid-flight re-runs this effect and
        // cancels the in-flight closure — which would skip the setSession below and
        // drop the newly rotated (single-use) refresh token, logging the user out
        // on the next load.
        const me = await api.get<MeResponse>('/me', {
          headers: { Authorization: `Bearer ${data.access_token}` },
        })
        if (cancelled) return
        setSession({
          access_token: data.access_token,
          refresh_token: data.refresh_token,
          expires_at: data.expires_at,
          user: me.data.user,
          arma_linked: me.data.arma_linked,
        })
      })
      .catch(() => {
        if (!cancelled) clearSession()
      })
      .finally(() => {
        if (!cancelled) setBootstrapping(false)
      })

    return () => {
      cancelled = true
    }
  }, [accessToken, refreshToken, setSession, setBootstrapping, clearSession])
}
