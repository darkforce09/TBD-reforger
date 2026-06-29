import axios from 'axios'
import { useAuthStore } from '@/store/useAuthStore'
import { refreshSession } from '@/api/refresh'

/**
 * Shared axios client for the TBD API (base `/api/v1`, overridable via VITE_API_URL). Injects the
 * bearer token on every request and, on a 401, runs a single-flight token refresh
 * ({@link refreshSession}) and retries the original request once.
 */
export const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL ?? '/api/v1',
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.request.use((config) => {
  const token = useAuthStore.getState().accessToken
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

api.interceptors.response.use(
  (res) => res,
  async (error) => {
    const original = error.config
    if (error.response?.status === 401 && original && !original._retry) {
      original._retry = true
      const refreshToken = useAuthStore.getState().refreshToken
      if (refreshToken) {
        try {
          // Shared single-flight refresh: never double-spends the token even if
          // several requests 401 at once or the bootstrap is refreshing too.
          const data = await refreshSession()
          const user = useAuthStore.getState().user
          if (user) {
            useAuthStore.getState().setSession({
              access_token: data.access_token,
              refresh_token: data.refresh_token,
              expires_at: data.expires_at,
              user,
              arma_linked: Boolean(user.arma_id),
            })
          } else {
            useAuthStore.getState().setAccessToken(data.access_token, data.expires_at)
          }
          original.headers.Authorization = `Bearer ${data.access_token}`
          return api(original)
        } catch {
          useAuthStore.getState().clearSession()
        }
      }
    }
    return Promise.reject(error)
  },
)
