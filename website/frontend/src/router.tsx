import { Suspense } from 'react'
import { createBrowserRouter } from 'react-router-dom'
import { AppLayout } from '@/components/layout/AppLayout'
import { ProtectedRoute } from '@/components/ProtectedRoute'
import { MissionEditorPage } from '@/features/mission-creator/routes'
import {
  AnnouncementsPage,
  DeploymentsPage,
  EventSchedulePage,
  LeaderboardsPage,
} from '@/pages/operations'
import { EventHubPage, OrbatSelectionPage } from '@/pages/events'
import { DashboardPage } from '@/pages/Dashboard'
import { ServerIntelPage } from '@/pages/ServerIntel'
import { MissionLibraryPage, MissionOverviewPage } from '@/pages/missions'
import {
  ModpacksPage,
  MortarCalculatorPage,
  VehicleDatabasePage,
  WikiPage,
} from '@/pages/doctrine'
import {
  AuditLogsPage,
  ContentManagerPage,
  EventManagerPage,
  MissionApprovalsPage,
  PersonnelRosterPage,
} from '@/pages/admin'
import { SettingsPage } from '@/pages/Settings'
import { AuthCallbackPage, LoginPage } from '@/pages/auth'
import { NotFoundPage, ServerControlPage } from '@/pages/utility'

const breadcrumb = (
  parent: string,
  current: string,
  opts?: { fullBleed?: boolean; chromeless?: boolean },
) => ({
  handle: {
    breadcrumb: { parent, current },
    fullBleed: opts?.fullBleed,
    chromeless: opts?.chromeless,
  },
})

export const router = createBrowserRouter([
  {
    path: '/login',
    element: <LoginPage />,
  },
  {
    path: '/auth/callback',
    element: <AuthCallbackPage />,
  },
  {
    path: '/',
    element: <AppLayout />,
    children: [
      { index: true, element: <DashboardPage />, ...breadcrumb('Command Center', 'Dashboard', { fullBleed: true }) },
      {
        path: 'server-intel',
        element: <ServerIntelPage />,
        ...breadcrumb('Command Center', 'Server Intel', { fullBleed: true }),
      },
      {
        path: 'announcements',
        element: <AnnouncementsPage />,
        ...breadcrumb('Command Center', 'Announcements', { fullBleed: true }),
      },
      {
        path: 'deployments',
        element: <DeploymentsPage />,
        ...breadcrumb('Operations', 'My Deployments', { fullBleed: true }),
      },
      {
        path: 'leaderboards',
        element: <LeaderboardsPage />,
        ...breadcrumb('Operations', 'Global Leaderboards', { fullBleed: true }),
      },
      {
        path: 'missions',
        element: <MissionLibraryPage />,
        ...breadcrumb('Mission Hub', 'Mission Library', { fullBleed: true }),
      },
      {
        path: 'missions/:id',
        element: <MissionOverviewPage />,
        ...breadcrumb('Mission Hub', 'Mission Overview'),
      },
      {
        path: 'events',
        element: <EventSchedulePage />,
        ...breadcrumb('Operations', 'Event Schedule', { fullBleed: true }),
      },
      {
        path: 'events/:id',
        element: <EventHubPage />,
        ...breadcrumb('Operations', 'Event Hub', { fullBleed: true }),
      },
      {
        path: 'events/:id/missions/:emid/orbat',
        element: <OrbatSelectionPage />,
        ...breadcrumb('Operations', 'ORBAT Selection'),
      },
      { path: 'wiki', element: <WikiPage />, ...breadcrumb('Doctrine & Info', 'SOPs & Manuals', { fullBleed: true }) },
      { path: 'wiki/:slug', element: <WikiPage />, ...breadcrumb('Doctrine & Info', 'SOPs & Manuals', { fullBleed: true }) },
      {
        path: 'vehicles',
        element: <VehicleDatabasePage />,
        ...breadcrumb('Doctrine & Info', 'Vehicle Database', { fullBleed: true }),
      },
      {
        path: 'modpacks',
        element: <ModpacksPage />,
        ...breadcrumb('Doctrine & Info', 'Modpacks', { fullBleed: true }),
      },
      {
        path: 'tools/mortar',
        element: <MortarCalculatorPage />,
        ...breadcrumb('Field Tools', 'Mortar Calculator', { fullBleed: true }),
      },
      { path: 'settings', element: <SettingsPage />, ...breadcrumb('Account', 'Settings') },
      {
        element: <ProtectedRoute minRole="mission_maker" />,
        children: [
          {
            path: 'missions/:id/edit',
            element: (
              <Suspense
                fallback={
                  <div className="flex h-full items-center justify-center text-on-surface-variant">
                    Loading editor…
                  </div>
                }
              >
                <MissionEditorPage />
              </Suspense>
            ),
            ...breadcrumb('Mission Hub', 'Mission Creator', {
              fullBleed: true,
              chromeless: true,
            }),
          },
        ],
      },
      {
        element: <ProtectedRoute minRole="admin" />,
        children: [
          {
            path: 'admin/events',
            element: <EventManagerPage />,
            ...breadcrumb('Administration', 'Event Manager'),
          },
          {
            path: 'admin/approvals',
            element: <MissionApprovalsPage />,
            ...breadcrumb('Administration', 'Mission Approvals', { fullBleed: true }),
          },
          {
            path: 'admin/server',
            element: <ServerControlPage />,
            ...breadcrumb('Administration', 'Server Control', { fullBleed: true }),
          },
          {
            path: 'admin/personnel',
            element: <PersonnelRosterPage />,
            ...breadcrumb('Administration', 'Personnel Roster', { fullBleed: true }),
          },
          {
            path: 'admin/content',
            element: <ContentManagerPage />,
            ...breadcrumb('Administration', 'Comms Broadcaster', { fullBleed: true }),
          },
          {
            path: 'admin/audit',
            element: <AuditLogsPage />,
            ...breadcrumb('Administration', 'Audit Logs', { fullBleed: true }),
          },
        ],
      },
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
