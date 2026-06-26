import { useState } from 'react'
import { NavLink } from 'react-router-dom'
import { navigation } from '@/config/navigation'
import { useAuthStore } from '@/store/useAuthStore'
import { hasMinRole } from '@/lib/roles'
import { MaterialIcon } from '@/components/MaterialIcon'
import { cn } from '@/lib/utils'

function SidebarNav({ onNavigate }: { onNavigate?: () => void }) {
  const user = useAuthStore((s) => s.user)

  return (
    <nav className="custom-scrollbar flex-1 overflow-y-auto px-3 py-4">
      {navigation.map((section) => {
        if (section.admin && !hasMinRole(user?.role, 'admin')) return null
        const items = section.items.filter((item) => hasMinRole(user?.role, item.minRole))
        if (items.length === 0) return null
        return (
          <div
            key={section.title}
            className={cn('mb-6', section.admin && 'rounded-lg border border-red-500/20 bg-red-900/10 p-3')}
          >
            <h3
              className={cn(
                'mb-2 px-2 text-xs font-bold tracking-widest uppercase',
                section.admin ? 'text-red-400' : 'text-gray-500',
              )}
            >
              {section.title}
            </h3>
            <ul className="space-y-1">
              {items.map((item) => (
                <li key={item.path}>
                  <NavLink
                    to={item.path}
                    end={item.path === '/'}
                    onClick={onNavigate}
                    className={({ isActive }) =>
                      cn(
                        'flex items-center gap-3 rounded-md px-3 py-2.5 text-label-md font-medium transition-colors',
                        isActive
                          ? 'nav-item-active text-primary'
                          : 'text-on-surface-variant hover:bg-surface-variant/40 hover:text-on-surface',
                      )
                    }
                  >
                    <MaterialIcon name={item.icon} className="text-[22px]" />
                    {item.label}
                  </NavLink>
                </li>
              ))}
            </ul>
          </div>
        )
      })}
    </nav>
  )
}

function SidebarBrand() {
  return (
    <header className="relative flex h-16 shrink-0 items-center px-6">
      <div className="flex items-center gap-2">
        <span className="text-2xl font-bold tracking-wide text-primary">TBD</span>
        <span className="text-2xl font-bold tracking-wide text-on-surface">Reforger</span>
      </div>
    </header>
  )
}

export function Sidebar() {
  return (
    <aside className="hidden h-screen w-80 shrink-0 flex-col bg-surface-container-low lg:flex">
      <SidebarBrand />
      <SidebarNav />
    </aside>
  )
}

export function SidebarMobileToggle() {
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <>
      <button
        type="button"
        className="fixed top-3 left-3 z-50 rounded-md bg-surface-container p-2 lg:hidden"
        onClick={() => setMobileOpen(true)}
        aria-label="Open menu"
      >
        <MaterialIcon name="menu" />
      </button>
      {mobileOpen && (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div className="absolute inset-0 bg-black/60" onClick={() => setMobileOpen(false)} />
          <aside className="absolute top-0 left-0 flex h-full w-80 flex-col bg-surface-container-low shadow-xl">
            <SidebarBrand />
            <SidebarNav onNavigate={() => setMobileOpen(false)} />
          </aside>
        </div>
      )}
    </>
  )
}
