import type { UserRole } from '@/types/models/user'

export interface NavItem {
  label: string
  path: string
  icon: string
  minRole: UserRole
}

export interface NavSection {
  title: string
  items: NavItem[]
  admin?: boolean
}

export const navigation: NavSection[] = [
  {
    title: 'Command Center',
    items: [
      { label: 'Dashboard', path: '/', icon: 'grid_view', minRole: 'enlisted' },
      { label: 'Server Intel', path: '/server-intel', icon: 'dns', minRole: 'enlisted' },
      { label: 'Announcements', path: '/announcements', icon: 'campaign', minRole: 'enlisted' },
    ],
  },
  {
    title: 'Operations',
    items: [
      { label: 'Event Schedule', path: '/events', icon: 'calendar_month', minRole: 'enlisted' },
      { label: 'My Deployments', path: '/deployments', icon: 'military_tech', minRole: 'enlisted' },
      { label: 'Global Leaderboards', path: '/leaderboards', icon: 'leaderboard', minRole: 'enlisted' },
    ],
  },
  {
    title: 'Mission Hub',
    items: [
      { label: 'Mission Library', path: '/missions', icon: 'library_books', minRole: 'enlisted' },
    ],
  },
  {
    title: 'Field Tools',
    items: [{ label: 'Mortar Calculator', path: '/tools/mortar', icon: 'calculate', minRole: 'enlisted' }],
  },
  {
    title: 'Doctrine & Info',
    items: [
      { label: 'SOPs & Manuals', path: '/wiki', icon: 'menu_book', minRole: 'enlisted' },
      { label: 'Vehicle Database', path: '/vehicles', icon: 'directions_car', minRole: 'enlisted' },
      { label: 'Modpacks', path: '/modpacks', icon: 'extension', minRole: 'enlisted' },
    ],
  },
  {
    title: 'Administration',
    admin: true,
    items: [
      { label: 'Event Manager', path: '/admin/events', icon: 'event_available', minRole: 'admin' },
      { label: 'Mission Approvals', path: '/admin/approvals', icon: 'fact_check', minRole: 'admin' },
      { label: 'Server Control', path: '/admin/server', icon: 'settings_system_daydream', minRole: 'admin' },
      { label: 'Personnel Roster', path: '/admin/personnel', icon: 'groups', minRole: 'admin' },
      { label: 'Comms Broadcaster', path: '/admin/content', icon: 'campaign', minRole: 'admin' },
      { label: 'Audit Logs', path: '/admin/audit', icon: 'receipt_long', minRole: 'admin' },
    ],
  },
]
