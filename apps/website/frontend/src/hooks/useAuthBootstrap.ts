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
  const setTokens = useAuthStore((s) => s.setTokens)
  const setBootstrapping = useAuthStore((s) => s.setBootstrapping)
  const clearSession = useAuthStore((s) => s.clearSession)

  useEffect(() => {
    if (accessToken) return
    if (!refreshToken) return

    let cancelled = false
    setBootstrapping(true)

    // Rotation and profile fetch fail for different reasons and must be handled
    // differently (T-126 S6): a failed rotation means the stored refresh token is
    // dead → clear the session; a rotation that SUCCEEDED followed by a transient
    // /me failure must KEEP the freshly rotated (single-use) pair, or a network
    // blip at boot logs the user out and burns their only valid refresh token.
    ;(async () => {
      let data: Awaited<ReturnType<typeof refreshSession>>
      try {
        data = await refreshSession()
      } catch {
        // Rotation itself failed — the persisted refresh token is invalid/expired.
        if (!cancelled) clearSession()
        if (!cancelled) setBootstrapping(false)
        return
      }
      // Rotation succeeded: the old refresh token is now revoked server-side, so we
      // must persist the rotated pair regardless of what /me does next.
      try {
        // Pass the fresh token explicitly. We must NOT mutate the store before the
        // profile resolves: accessToken is an effect dependency, so writing it
        // mid-flight re-runs this effect and cancels the in-flight closure.
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
      } catch {
        // Rotation OK but /me failed (transient 5xx/network): retain the rotated
        // pair so the next navigation/reload re-fetches the profile — never discard
        // the only valid refresh token over a profile blip.
        if (!cancelled) setTokens(data)
      } finally {
        if (!cancelled) setBootstrapping(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [accessToken, refreshToken, setSession, setTokens, setBootstrapping, clearSession])
}
