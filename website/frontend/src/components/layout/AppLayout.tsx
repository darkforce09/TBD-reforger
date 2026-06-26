import { Outlet, useMatches } from 'react-router-dom'
import { Sidebar, SidebarMobileToggle } from '@/components/layout/Sidebar'
import { TopNav } from '@/components/layout/TopNav'
import { useAuthBootstrap } from '@/hooks/useAuthBootstrap'
import { cn } from '@/lib/utils'

interface RouteHandle {
  fullBleed?: boolean
  /** Chromeless routes (the Mission Creator editor) own the full viewport — no
   *  platform Sidebar/TopNav. See ux_spec.md. */
  chromeless?: boolean
}

export function AppLayout() {
  useAuthBootstrap()
  const matches = useMatches()
  const fullBleed = matches.some((m) => (m.handle as RouteHandle)?.fullBleed)
  const chromeless = matches.some((m) => (m.handle as RouteHandle)?.chromeless)

  // Fullscreen Eden editor: render the route alone, no Sidebar/TopNav.
  if (chromeless) {
    return (
      <div className="h-screen w-screen overflow-hidden bg-background">
        <Outlet />
      </div>
    )
  }

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <SidebarMobileToggle />
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopNav />
        {/* Full-bleed pages (split-pane master/detail) manage their own height
            and padding; classic pages keep the padded, scrollable container. */}
        <main
          className={cn(
            'min-h-0 flex-1 bg-background',
            fullBleed ? 'overflow-hidden' : 'overflow-y-auto p-6',
          )}
        >
          <Outlet />
        </main>
      </div>
    </div>
  )
}
