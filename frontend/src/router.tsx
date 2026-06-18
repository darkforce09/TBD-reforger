import { createBrowserRouter } from 'react-router-dom'
import { AppLayout } from '@/components/layout/AppLayout'
import { ProtectedRoute } from '@/components/ProtectedRoute'
import {
  AnnouncementsPage,
  DeploymentsPage,
  EventSchedulePage,
  LeaderboardsPage,
} from '@/pages/operations'
import { EventHubPage, OrbatSelectionPage } from '@/pages/events'
import { DashboardPage } from '@/pages/Dashboard'
import { ServerIntelPage } from '@/pages/ServerIntel'
import {
  MissionCreatorPage,
  MissionLibraryPage,
  MissionOverviewPage,
} from '@/pages/missions'
import { ModpacksPage, MortarCalculatorPage, WikiPage } from '@/pages/doctrine'
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

const breadcrumb = (parent: string, current: string) => ({
  handle: { breadcrumb: { parent, current } },
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
      { index: true, element: <DashboardPage />, ...breadcrumb('Command Center', 'Dashboard') },
      {
        path: 'server-intel',
        element: <ServerIntelPage />,
        ...breadcrumb('Command Center', 'Server Intel'),
      },
      {
        path: 'announcements',
        element: <AnnouncementsPage />,
        ...breadcrumb('Command Center', 'Announcements'),
      },
      {
        path: 'deployments',
        element: <DeploymentsPage />,
        ...breadcrumb('Operations', 'My Deployments'),
      },
      {
        path: 'leaderboards',
        element: <LeaderboardsPage />,
        ...breadcrumb('Operations', 'Global Leaderboards'),
      },
      {
        path: 'missions',
        element: <MissionLibraryPage />,
        ...breadcrumb('Mission Hub', 'Mission Library'),
      },
      {
        path: 'missions/:id',
        element: <MissionOverviewPage />,
        ...breadcrumb('Mission Hub', 'Mission Overview'),
      },
      {
        path: 'events',
        element: <EventSchedulePage />,
        ...breadcrumb('Operations', 'Event Schedule'),
      },
      {
        path: 'events/:id',
        element: <EventHubPage />,
        ...breadcrumb('Operations', 'Event Hub'),
      },
      {
        path: 'events/:id/missions/:emid/orbat',
        element: <OrbatSelectionPage />,
        ...breadcrumb('Operations', 'ORBAT Selection'),
      },
      { path: 'wiki', element: <WikiPage />, ...breadcrumb('Doctrine & Info', 'SOPs & Manuals') },
      { path: 'wiki/:slug', element: <WikiPage />, ...breadcrumb('Doctrine & Info', 'SOPs & Manuals') },
      { path: 'modpacks', element: <ModpacksPage />, ...breadcrumb('Doctrine & Info', 'Modpacks') },
      {
        path: 'tools/mortar',
        element: <MortarCalculatorPage />,
        ...breadcrumb('Field Tools', 'Mortar Calculator'),
      },
      { path: 'settings', element: <SettingsPage />, ...breadcrumb('Account', 'Settings') },
      {
        element: <ProtectedRoute minRole="mission_maker" />,
        children: [
          {
            path: 'missions/create',
            element: <MissionCreatorPage />,
            ...breadcrumb('Mission Hub', 'Mission Creator'),
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
            ...breadcrumb('Administration', 'Mission Approvals'),
          },
          {
            path: 'admin/server',
            element: <ServerControlPage />,
            ...breadcrumb('Administration', 'Server Control'),
          },
          {
            path: 'admin/personnel',
            element: <PersonnelRosterPage />,
            ...breadcrumb('Administration', 'Personnel Roster'),
          },
          {
            path: 'admin/content',
            element: <ContentManagerPage />,
            ...breadcrumb('Administration', 'Content Manager'),
          },
          {
            path: 'admin/audit',
            element: <AuditLogsPage />,
            ...breadcrumb('Administration', 'Audit Logs'),
          },
        ],
      },
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
