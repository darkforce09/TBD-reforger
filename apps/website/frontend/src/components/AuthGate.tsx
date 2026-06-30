import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { useAuthStore } from '@/store/useAuthStore'

interface AuthGateProps {
  children: ReactNode
}

/** API routes require Discord auth — show sign-in CTA for guests. */
export function AuthGate({ children }: AuthGateProps) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated())
  const bootstrapping = useAuthStore((s) => s.bootstrapping)

  if (bootstrapping) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
        Loading session…
      </div>
    )
  }

  if (!isAuthenticated) {
    return (
      <div className="flex min-h-[40vh] flex-col items-center justify-center gap-4 text-center">
        <p className="text-on-surface-variant">Sign in to load live data from the platform.</p>
        <Link
          to="/login"
          className="rounded-lg bg-primary px-6 py-2.5 text-sm font-medium text-on-primary"
        >
          Sign in with Discord
        </Link>
      </div>
    )
  }

  return <>{children}</>
}
