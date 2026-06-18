import { useState } from 'react'
import { Link, useMatches } from 'react-router-dom'
import { useAuthStore } from '@/store/useAuthStore'
import { MaterialIcon } from '@/components/MaterialIcon'
import { StatusPill } from '@/components/StatusPill'
import { useLogout } from '@/hooks/mutations'
import { DEFAULT_AVATAR } from '@/lib/avatar'

interface RouteHandle {
  breadcrumb?: { parent: string; current: string }
}

export function TopNav() {
  const [open, setOpen] = useState(false)
  const user = useAuthStore((s) => s.user)
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated())
  const logout = useLogout()
  const matches = useMatches()
  const handle = [...matches].reverse().find((m) => (m.handle as RouteHandle)?.breadcrumb)
    ?.handle as RouteHandle | undefined
  const breadcrumb = handle?.breadcrumb

  return (
    <header className="flex h-16 shrink-0 items-center justify-between bg-surface-container-low px-6">
      <div className="flex h-full min-w-0 items-center gap-2 pl-12 lg:pl-0">
        {breadcrumb ? (
          <>
            <span className="text-sm text-on-surface-variant">{breadcrumb.parent}</span>
            <span className="text-on-surface-variant">/</span>
            <span className="text-sm font-semibold text-on-surface">{breadcrumb.current}</span>
          </>
        ) : (
          <span className="text-sm font-semibold text-on-surface">TBD Reforger</span>
        )}
      </div>
      <div className="relative flex h-full items-center gap-4">
        {!isAuthenticated ? (
          <Link
            to="/login"
            className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary"
          >
            Sign in with Discord
          </Link>
        ) : (
          <>
            <StatusPill linked={Boolean(user?.arma_id)} armaId={user?.arma_id} />
            <button
              type="button"
              onClick={() => setOpen(!open)}
              className="flex items-center gap-2 rounded-lg p-1 pr-3 hover:bg-surface-container-high"
            >
              <img
                src={user?.avatar_url || DEFAULT_AVATAR}
                alt=""
                className="h-8 w-8 rounded-full border border-border-subtle object-cover"
              />
              <span className="text-sm font-medium">{user?.username}</span>
              <MaterialIcon name="expand_more" className="text-on-surface-variant" />
            </button>
            {open && (
              <div className="absolute top-full right-0 z-50 mt-2 w-56 overflow-hidden rounded-lg border border-border-subtle bg-surface-container shadow-xl">
                <Link
                  to="/settings"
                  className="block px-4 py-2 text-sm hover:bg-surface-container-high"
                  onClick={() => setOpen(false)}
                >
                  Settings
                </Link>
                <Link
                  to="/settings"
                  className="block px-4 py-2 text-sm hover:bg-surface-container-high"
                  onClick={() => setOpen(false)}
                >
                  Link Arma Identity
                </Link>
                <hr className="border-border-subtle" />
                <button
                  type="button"
                  className="block w-full px-4 py-2 text-left text-sm text-error hover:bg-surface-container-high"
                  onClick={() => {
                    logout.mutate()
                    setOpen(false)
                  }}
                >
                  Sign Out
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </header>
  )
}
