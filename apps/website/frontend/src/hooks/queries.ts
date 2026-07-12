// React Query read hooks. Each hook documents the endpoint it calls with an @route tag
// (DOCUMENTATION_STANDARDS §5) so a route string maps to its hook + Go handler in one grep.
import { useQuery } from '@tanstack/react-query'
import { api } from '@/api/client'
import { useAuthStore } from '@/store/useAuthStore'
import type { FactionListResponse } from '@/types/models/faction'
import type {
  Announcement,
  ApprovalRow,
  AuditLogResponse,
  DashboardResponse,
  DeploymentsResponse,
  EventHub,
  EventListItem,
  LeaderboardResponse,
  Member,
  OrbatSquad,
  LinkStatus,
  MeResponse,
  MissionCard,
  MissionDetail,
  ModpackDTO,
  Paginated,
  RosterRow,
  ServerIntel,
  VehicleRow,
  WikiPage,
} from '@/types/api'
import type { RegistryResponse } from '@/types/models/registry'

function useAuthed() {
  return useAuthStore((s) => s.isAuthenticated())
}

function useIsAdmin() {
  return useAuthStore((s) => s.hasMinRole('admin'))
}

/**
 * The current authenticated user.
 *
 * @route GET /api/v1/me
 */
export function useMe() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['me'],
    queryFn: async () => (await api.get<MeResponse>('/me')).data,
    enabled: authed,
    staleTime: 60_000,
  })
}

/**
 * Virtual Arsenal catalog (T-068.3) — feeds the Mission Creator Asset Browser palette.
 *
 * @route GET /api/v1/registry
 */
export function useRegistry() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['registry'],
    queryFn: async () => (await api.get<RegistryResponse>('/registry')).data,
    enabled: authed,
    staleTime: 5 * 60_000,
  })
}

/**
 * The caller's Arma identity link status.
 *
 * @route GET /api/v1/me/link/status
 */
export function useLinkStatus() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['me', 'link'],
    queryFn: async () => (await api.get<LinkStatus>('/me/link/status')).data,
    enabled: authed,
  })
}

/**
 * Aggregated landing-page payload (next event, assignment, server, modpack, news).
 *
 * @route GET /api/v1/dashboard
 */
export function useDashboard() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['dashboard'],
    queryFn: async () => (await api.get<DashboardResponse>('/dashboard')).data,
    enabled: authed,
    staleTime: 30_000,
  })
}

/**
 * A leaderboard for the given category (optional name filter).
 *
 * @route GET /api/v1/leaderboards
 */
export function useLeaderboards(category: string, q?: string) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['leaderboards', category, q],
    queryFn: async () =>
      (await api.get<LeaderboardResponse>('/leaderboards', { params: { category, q } })).data,
    enabled: authed,
    staleTime: 60_000,
  })
}

/**
 * The list of registered game servers with live status.
 *
 * @route GET /api/v1/servers
 */
export function useServers() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['servers'],
    queryFn: async () => (await api.get<{ data: ServerIntel[] }>('/servers')).data.data,
    enabled: authed,
    staleTime: 15_000,
  })
}

/**
 * Live status for a single server.
 *
 * @route GET /api/v1/servers/:id/status
 */
export function useServer(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['servers', id],
    queryFn: async () => (await api.get<ServerIntel>(`/servers/${id}/status`)).data,
    enabled: authed && Boolean(id),
    staleTime: 10_000,
  })
}

/**
 * A page of CMS announcements.
 *
 * @route GET /api/v1/announcements
 */
export function useAnnouncements(limit = 20, offset = 0) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['announcements', limit, offset],
    queryFn: async () =>
      (await api.get<Paginated<Announcement>>('/announcements', { params: { limit, offset } }))
        .data,
    enabled: authed,
  })
}

/**
 * The caller's My Deployments service record.
 *
 * @route GET /api/v1/me/deployments
 */
export function useDeployments() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['deployments'],
    queryFn: async () => (await api.get<DeploymentsResponse>('/me/deployments')).data,
    enabled: authed,
  })
}

/**
 * A page of events for the given scope (upcoming/past).
 *
 * @route GET /api/v1/events
 */
export function useEvents(scope = 'upcoming') {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['events', scope],
    queryFn: async () =>
      (await api.get<Paginated<EventListItem>>('/events', { params: { scope } })).data,
    enabled: authed,
  })
}

/**
 * One Event Hub with its nested mission dossiers.
 *
 * @route GET /api/v1/events/:id
 */
export function useEvent(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['events', id],
    queryFn: async () => (await api.get<EventHub>(`/events/${id}`)).data,
    enabled: authed && Boolean(id),
  })
}

/**
 * The ORBAT (squads + slots) for one event-mission.
 *
 * @route GET /api/v1/event-missions/:emid/orbat
 */
export function useOrbat(emid: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['orbat', emid],
    queryFn: async () =>
      (await api.get<{ data: OrbatSquad[] }>(`/event-missions/${emid}/orbat`)).data.data,
    enabled: authed && Boolean(emid),
  })
}

/**
 * Leader-only member directory for picking assignees for a reserved squad.
 *
 * @route GET /api/v1/members
 */
export function useMemberSearch(q: string, enabled = true) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['members', q],
    queryFn: async () =>
      (await api.get<{ data: Member[] }>('/members', { params: { q } })).data.data,
    enabled: authed && enabled,
    staleTime: 30_000,
  })
}

/**
 * A page of mission library cards for the given scope + filters.
 *
 * @route GET /api/v1/missions
 */
export function useMissions(scope = 'global', filters?: Record<string, string>) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['missions', scope, filters],
    queryFn: async () =>
      (
        await api.get<Paginated<MissionCard>>('/missions', {
          params: { scope, ...filters },
        })
      ).data,
    enabled: authed,
  })
}

/**
 * The full Mission Overview for one mission.
 *
 * @route GET /api/v1/missions/:id
 */
export function useMission(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['missions', id],
    queryFn: async () => (await api.get<MissionDetail>(`/missions/${id}`)).data,
    enabled: authed && Boolean(id),
  })
}

/**
 * All modpacks with their mod lists.
 *
 * @route GET /api/v1/modpacks
 */
export function useModpacks() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['modpacks'],
    queryFn: async () => (await api.get<{ data: ModpackDTO[] }>('/modpacks')).data.data,
    enabled: authed,
  })
}

/**
 * The current (is_current) modpack.
 *
 * @route GET /api/v1/modpacks/current
 */
export function useCurrentModpack() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['modpacks', 'current'],
    queryFn: async () => (await api.get<ModpackDTO>('/modpacks/current')).data,
    enabled: authed,
  })
}

/**
 * The doctrine wiki page index.
 *
 * @route GET /api/v1/wiki
 */
export function useWikiPages() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['wiki'],
    queryFn: async () => (await api.get<{ data: WikiPage[] }>('/wiki')).data.data,
    enabled: authed,
  })
}

/**
 * One doctrine wiki page by slug.
 *
 * @route GET /api/v1/wiki/:slug
 */
export function useWikiPage(slug: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['wiki', slug],
    queryFn: async () => (await api.get<WikiPage>(`/wiki/${slug}`)).data,
    enabled: authed && Boolean(slug),
  })
}

/**
 * The vehicle database.
 *
 * @route GET /api/v1/vehicle-database
 */
export function useVehicles() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['vehicle-database'],
    queryFn: async () => (await api.get<{ data: VehicleRow[] }>('/vehicle-database')).data.data,
    enabled: authed,
  })
}

/**
 * Admin: missions pending approval.
 *
 * @route GET /api/v1/approvals
 */
export function useApprovals() {
  const authed = useAuthed()
  const isAdmin = useIsAdmin()
  return useQuery({
    queryKey: ['approvals'],
    queryFn: async () => (await api.get<Paginated<ApprovalRow>>('/approvals')).data,
    enabled: authed && isAdmin,
  })
}

/**
 * Admin: personnel roster (optional name filter).
 *
 * @route GET /api/v1/admin/users
 */
export function usePersonnel(q?: string) {
  const authed = useAuthed()
  const isAdmin = useIsAdmin()
  return useQuery({
    queryKey: ['personnel', q],
    queryFn: async () =>
      (await api.get<Paginated<RosterRow>>('/admin/users', { params: { q } })).data,
    enabled: authed && isAdmin,
  })
}

/**
 * Admin: a cursor page of audit logs (optional severity / search filter).
 *
 * @route GET /api/v1/admin/audit-logs
 */
export function useAuditLogs(params?: { severity?: string; q?: string }) {
  const authed = useAuthed()
  const isAdmin = useIsAdmin()
  return useQuery({
    queryKey: ['audit-logs', params],
    queryFn: async () => (await api.get<AuditLogResponse>('/admin/audit-logs', { params })).data,
    enabled: authed && isAdmin,
  })
}

/**
 * The caller's faction library (T-152) - side -> faction -> roles/vehicles for the palette
 * and the Faction Manager.
 *
 * @route GET /api/v1/factions
 */
export function useFactionLibrary() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['factions'],
    queryFn: async () => (await api.get<FactionListResponse>('/factions')).data,
    enabled: authed,
    staleTime: 60_000,
  })
}
