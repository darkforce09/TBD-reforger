import { useQuery } from '@tanstack/react-query'
import { api } from '@/api/client'
import { useAuthStore } from '@/store/useAuthStore'
import type {
  Announcement,
  ApprovalRow,
  AuditLogResponse,
  DashboardResponse,
  DeploymentsResponse,
  EventHub,
  EventListItem,
  LeaderboardResponse,
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

function useAuthed() {
  return useAuthStore((s) => s.isAuthenticated())
}

function useIsAdmin() {
  return useAuthStore((s) => s.hasMinRole('admin'))
}

export function useMe() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['me'],
    queryFn: async () => (await api.get<MeResponse>('/me')).data,
    enabled: authed,
    staleTime: 60_000,
  })
}

export function useLinkStatus() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['me', 'link'],
    queryFn: async () => (await api.get<LinkStatus>('/me/link/status')).data,
    enabled: authed,
  })
}

export function useDashboard() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['dashboard'],
    queryFn: async () => (await api.get<DashboardResponse>('/dashboard')).data,
    enabled: authed,
    staleTime: 30_000,
  })
}

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

export function useServers() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['servers'],
    queryFn: async () => (await api.get<{ data: ServerIntel[] }>('/servers')).data.data,
    enabled: authed,
    staleTime: 15_000,
  })
}

export function useServer(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['servers', id],
    queryFn: async () => (await api.get<ServerIntel>(`/servers/${id}/status`)).data,
    enabled: authed && Boolean(id),
    staleTime: 10_000,
  })
}

export function useAnnouncements(limit = 20, offset = 0) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['announcements', limit, offset],
    queryFn: async () =>
      (await api.get<Paginated<Announcement>>('/announcements', { params: { limit, offset } })).data,
    enabled: authed,
  })
}

export function useDeployments() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['deployments'],
    queryFn: async () => (await api.get<DeploymentsResponse>('/me/deployments')).data,
    enabled: authed,
  })
}

export function useEvents(scope = 'upcoming') {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['events', scope],
    queryFn: async () =>
      (await api.get<Paginated<EventListItem>>('/events', { params: { scope } })).data,
    enabled: authed,
  })
}

export function useEvent(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['events', id],
    queryFn: async () => (await api.get<EventHub>(`/events/${id}`)).data,
    enabled: authed && Boolean(id),
  })
}

export function useOrbat(emid: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['orbat', emid],
    queryFn: async () =>
      (await api.get<{ data: OrbatSquad[] }>(`/event-missions/${emid}/orbat`)).data.data,
    enabled: authed && Boolean(emid),
  })
}

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

export function useMission(id: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['missions', id],
    queryFn: async () => (await api.get<MissionDetail>(`/missions/${id}`)).data,
    enabled: authed && Boolean(id),
  })
}

export function useModpacks() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['modpacks'],
    queryFn: async () => (await api.get<{ data: ModpackDTO[] }>('/modpacks')).data.data,
    enabled: authed,
  })
}

export function useCurrentModpack() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['modpacks', 'current'],
    queryFn: async () => (await api.get<ModpackDTO>('/modpacks/current')).data,
    enabled: authed,
  })
}

export function useWikiPages() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['wiki'],
    queryFn: async () => (await api.get<{ data: WikiPage[] }>('/wiki')).data.data,
    enabled: authed,
  })
}

export function useWikiPage(slug: string | undefined) {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['wiki', slug],
    queryFn: async () => (await api.get<WikiPage>(`/wiki/${slug}`)).data,
    enabled: authed && Boolean(slug),
  })
}

export function useVehicles() {
  const authed = useAuthed()
  return useQuery({
    queryKey: ['vehicle-database'],
    queryFn: async () => (await api.get<{ data: VehicleRow[] }>('/vehicle-database')).data.data,
    enabled: authed,
  })
}

export function useApprovals() {
  const authed = useAuthed()
  const isAdmin = useIsAdmin()
  return useQuery({
    queryKey: ['approvals'],
    queryFn: async () => (await api.get<Paginated<ApprovalRow>>('/approvals')).data,
    enabled: authed && isAdmin,
  })
}

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

export function useAuditLogs(params?: { severity?: string; q?: string }) {
  const authed = useAuthed()
  const isAdmin = useIsAdmin()
  return useQuery({
    queryKey: ['audit-logs', params],
    queryFn: async () =>
      (await api.get<AuditLogResponse>('/admin/audit-logs', { params })).data,
    enabled: authed && isAdmin,
  })
}
