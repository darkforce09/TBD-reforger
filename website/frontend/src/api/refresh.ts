import axios from 'axios'
import { useAuthStore } from '@/store/useAuthStore'

export type RefreshResponse = {
  access_token: string
  refresh_token: string
  expires_at: string
}

// Refresh tokens are single-use: the server rotates and revokes the old token on
// every /auth/refresh call. Multiple callers can want a refresh at the same time
// (the app-load bootstrap and the axios 401 interceptor, React StrictMode's
// double-invoked effects, several 401s at once). Without coordination they would
// each present the same token and all but the first would get a 401 — which would
// wrongly clear the session. This single-flight helper guarantees the token is
// spent at most once at a time; concurrent callers share one in-flight request.
let inflight: Promise<RefreshResponse> | null = null

export function refreshSession(): Promise<RefreshResponse> {
  if (!inflight) {
    const base = import.meta.env.VITE_API_URL ?? '/api/v1'
    const refreshToken = useAuthStore.getState().refreshToken
    inflight = axios
      .post<RefreshResponse>(`${base}/auth/refresh`, { refresh_token: refreshToken })
      .then((res) => res.data)
      .finally(() => {
        inflight = null
      })
  }
  return inflight
}
