import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { api } from '@/api/client'
import { useAuthStore } from '@/store/useAuthStore'
import type { User } from '@/types/models/user'

export function LoginPage() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated())

  useEffect(() => {
    if (isAuthenticated) window.location.href = '/'
  }, [isAuthenticated])

  const startLogin = () => {
    window.location.href = `${import.meta.env.VITE_API_URL ?? '/api/v1'}/auth/discord/login`
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-6">
      <div className="w-full max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
        <h1 className="text-2xl font-bold">
          <span className="text-primary">TBD</span> Reforger
        </h1>
        <p className="mt-2 text-on-surface-variant">
          Sign in to register, deploy, and manage operations.
        </p>
        <button
          type="button"
          onClick={startLogin}
          className="mt-6 w-full rounded-lg bg-primary py-3 font-medium text-on-primary"
        >
          Sign in with Discord
        </button>
        <Link to="/" className="mt-4 block text-sm text-on-surface-variant hover:text-primary">
          Continue browsing without signing in
        </Link>
      </div>
    </div>
  )
}

// Human-readable copy for the error codes the backend returns in the fragment
// (see redirectAuthError in internal/handlers/auth.go).
const AUTH_ERROR_COPY: Record<string, string> = {
  missing_code: 'Discord did not return an authorization code. Please try again.',
  invalid_state: 'The sign-in request expired or was tampered with. Please try again.',
  discord_unreachable: 'Could not reach Discord. Please try again in a moment.',
  banned: 'This account is banned from the platform.',
  server_error: 'Something went wrong completing sign-in. Please try again.',
  no_session: 'No sign-in details were found. Please start from the login page.',
}

// AuthCallbackPage completes the Discord OAuth round-trip. The backend redirects
// here with the token pair in the URL fragment (kept out of the query string so
// tokens are not logged upstream). We parse the fragment, persist the session,
// fetch the profile via GET /me, then land the user on the dashboard.
// parseCallback reads the OAuth callback fragment once. The fragment is present
// synchronously at mount, so we derive the initial error / tokens during render
// (via a lazy useState initializer) rather than calling setState inside an effect.
function parseCallback(): { error?: string; tokens?: CallbackTokens } {
  const params = new URLSearchParams(window.location.hash.replace(/^#/, ''))
  const errCode = params.get('error')
  if (errCode) return { error: AUTH_ERROR_COPY[errCode] ?? AUTH_ERROR_COPY.server_error }

  const accessToken = params.get('access_token')
  const refreshToken = params.get('refresh_token')
  const expiresAt = params.get('expires_at')
  if (!accessToken || !refreshToken || !expiresAt) return { error: AUTH_ERROR_COPY.no_session }

  return {
    tokens: {
      accessToken,
      refreshToken,
      expiresAt,
      armaLinked: params.get('arma_linked') === 'true',
    },
  }
}

interface CallbackTokens {
  accessToken: string
  refreshToken: string
  expiresAt: string
  armaLinked: boolean
}

export function AuthCallbackPage() {
  const navigate = useNavigate()
  const setSession = useAuthStore((s) => s.setSession)
  const setAccessToken = useAuthStore((s) => s.setAccessToken)
  const [parsed] = useState(parseCallback)
  const [error, setError] = useState<string | null>(parsed.error ?? null)

  useEffect(() => {
    // Scrub the fragment so tokens do not linger in the address bar / history.
    window.history.replaceState(null, '', window.location.pathname)
    if (!parsed.tokens) return
    const { accessToken, refreshToken, expiresAt, armaLinked } = parsed.tokens

    // Set the access token first so the api client attaches it to GET /me.
    setAccessToken(accessToken, expiresAt)
    api
      .get<{ user: User; arma_linked: boolean }>('/me')
      .then(({ data }) => {
        setSession({
          access_token: accessToken,
          refresh_token: refreshToken,
          expires_at: expiresAt,
          user: data.user,
          arma_linked: data.arma_linked ?? armaLinked,
        })
        navigate('/', { replace: true })
      })
      .catch(() => {
        useAuthStore.getState().clearSession()
        setError(AUTH_ERROR_COPY.server_error)
      })
  }, [navigate, setSession, setAccessToken, parsed])

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
        {error ? (
          <>
            <h1 className="text-xl font-semibold text-error">Sign-in failed</h1>
            <p className="mt-2 text-sm text-on-surface-variant">{error}</p>
            <Link to="/login" className="mt-4 inline-block text-primary hover:underline">
              Back to login
            </Link>
          </>
        ) : (
          <>
            <h1 className="text-xl font-semibold">Signing you in…</h1>
            <p className="mt-2 text-sm text-on-surface-variant">
              Completing the Discord handshake and loading your profile.
            </p>
          </>
        )}
      </div>
    </div>
  )
}
