import { Outlet } from 'react-router-dom'
import { Sidebar, SidebarMobileToggle } from '@/components/layout/Sidebar'
import { TopNav } from '@/components/layout/TopNav'
import { useAuthBootstrap } from '@/hooks/useAuthBootstrap'

export function AppLayout() {
  useAuthBootstrap()
  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <SidebarMobileToggle />
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopNav />
        <main className="flex-1 overflow-y-auto bg-background p-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
