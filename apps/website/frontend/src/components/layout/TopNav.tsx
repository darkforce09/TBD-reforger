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
    <header className="flex h-16 shrink-0 items-center justify-between border-b border-outline-variant/30 bg-surface-container-low/70 px-6 backdrop-blur-xl">
      <div className="flex h-full min-w-0 items-center gap-2 pl-12 lg:pl-0">
        {breadcrumb ? (
          <>
            <span className="text-label-md text-on-surface-variant">{breadcrumb.parent}</span>
            <span className="text-outline">/</span>
            <span className="text-label-md font-semibold text-on-surface">{breadcrumb.current}</span>
          </>
        ) : (
          <span className="text-label-md font-semibold text-on-surface">TBD Reforger</span>
        )}
      </div>
      <div className="relative flex h-full items-center gap-4">
        {!isAuthenticated ? (
          <Link
            to="/login"
            className="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
          >
            Sign in with Discord
          </Link>
        ) : (
          <>
            <StatusPill linked={Boolean(user?.arma_id)} armaId={user?.arma_id} />
            <button
              type="button"
              onClick={() => setOpen(!open)}
              className="flex items-center gap-2 rounded-lg p-1 pr-3 transition-colors hover:bg-surface-variant/50"
            >
              <img
                src={user?.avatar_url || DEFAULT_AVATAR}
                alt=""
                className="h-8 w-8 rounded-full border border-outline-variant/50 object-cover"
              />
              <span className="text-label-md font-medium">{user?.username}</span>
              <MaterialIcon name="expand_more" className="text-on-surface-variant" />
            </button>
            {open && (
              <>
                <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
                <div className="glass absolute top-full right-0 z-50 mt-2 w-56 overflow-hidden rounded-lg shadow-2xl">
                  <Link
                    to="/settings"
                    className="block px-4 py-2 text-label-md transition-colors hover:bg-surface-variant/60"
                    onClick={() => setOpen(false)}
                  >
                    Settings
                  </Link>
                  <Link
                    to="/settings"
                    className="block px-4 py-2 text-label-md transition-colors hover:bg-surface-variant/60"
                    onClick={() => setOpen(false)}
                  >
                    Link Arma Identity
                  </Link>
                  <hr className="border-outline-variant/30" />
                  <button
                    type="button"
                    className="block w-full px-4 py-2 text-left text-label-md text-error-alert transition-colors hover:bg-surface-variant/60"
                    onClick={() => {
                      logout.mutate()
                      setOpen(false)
                    }}
                  >
                    Sign Out
                  </button>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </header>
  )
}
