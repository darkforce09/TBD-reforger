// React Query write hooks. Each hook documents the endpoint it calls with an @route tag
// (DOCUMENTATION_STANDARDS §5) so a route string maps to its hook + Go handler in one grep.
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api/client'
import { useAuthStore } from '@/store/useAuthStore'
import type { FireSolution } from '@/types/api'

/**
 * Log out: revoke the refresh token server-side (best-effort) and clear the local session.
 *
 * @route POST /api/v1/auth/logout
 */
export function useLogout() {
  const clearSession = useAuthStore((s) => s.clearSession)
  const refreshToken = useAuthStore((s) => s.refreshToken)
  return useMutation({
    mutationFn: async () => {
      if (refreshToken) {
        await api.post('/auth/logout', { refresh_token: refreshToken }).catch(() => undefined)
      }
      clearSession()
    },
  })
}

/**
 * Register for a specific mission within an event, optionally claiming a slot.
 *
 * @route POST /api/v1/event-missions/:emid/register
 */
export function useRegisterMission(emid: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (slotId?: string) => {
      const { data } = await api.post(`/event-missions/${emid}/register`, {
        slot_id: slotId,
      })
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['orbat', emid] })
      qc.invalidateQueries({ queryKey: ['events'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
      qc.invalidateQueries({ queryKey: ['deployments'] })
    },
  })
}

/**
 * Withdraw the caller's registration from an event-mission.
 *
 * @route DELETE /api/v1/event-missions/:emid/register
 */
export function useWithdrawMission(emid: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      const { data } = await api.delete(`/event-missions/${emid}/register`)
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['orbat', emid] })
      qc.invalidateQueries({ queryKey: ['events'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
      qc.invalidateQueries({ queryKey: ['deployments'] })
    },
  })
}

/**
 * Leader: reserve a whole squad in one click.
 *
 * @route POST /api/v1/event-missions/:emid/squads/reserve
 */
export function useReserveSquad(emid: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (squad: string) => {
      const { data } = await api.post(`/event-missions/${emid}/squads/reserve`, { squad })
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['orbat', emid] }),
  })
}

/**
 * Leader: release a previously reserved squad.
 *
 * @route POST /api/v1/event-missions/:emid/squads/release
 */
export function useReleaseSquad(emid: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (squad: string) => {
      const { data } = await api.post(`/event-missions/${emid}/squads/release`, { squad })
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['orbat', emid] }),
  })
}

/**
 * Leader/admin: assign a member to a slot in a reserved squad.
 *
 * @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
 */
export function useAssignSlot(emid: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ slotId, discordId }: { slotId: string; discordId: string }) => {
      const { data } = await api.put(`/event-missions/${emid}/slots/${slotId}/assign`, {
        discord_id: discordId,
      })
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['orbat', emid] })
      qc.invalidateQueries({ queryKey: ['events'] })
    },
  })
}

/**
 * Attach a mission to an event; the backend auto-materializes the ORBAT from the
 * mission payload.
 *
 * @route POST /api/v1/events/:id/missions
 */
export function useAddEventMission(eventId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: { mission_id: string; start_time: string }) => {
      const { data } = await api.post(`/events/${eventId}/missions`, body)
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['events'] })
      qc.invalidateQueries({ queryKey: ['events', eventId] })
    },
  })
}

/**
 * Admin: approve a pending mission.
 *
 * @route POST /api/v1/approvals/:id/approve
 */
export function useApproveMission() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (missionId: string) => {
      const { data } = await api.post(`/approvals/${missionId}/approve`)
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['approvals'] }),
  })
}

/**
 * Admin: reject a pending mission with an optional reason.
 *
 * @route POST /api/v1/approvals/:id/reject
 */
export function useRejectMission() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ id, reason }: { id: string; reason?: string }) => {
      const { data } = await api.post(`/approvals/${id}/reject`, { reason })
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['approvals'] }),
  })
}

/**
 * Create a draft mission (plus its initial version).
 *
 * @route POST /api/v1/missions
 */
export function useCreateMission() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: {
      title: string
      terrain: string
      game_mode: string
      weather?: string
      time_of_day?: string
      max_players?: number
    }) => {
      const { data } = await api.post('/missions', body)
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['missions'] }),
  })
}

/**
 * Admin: create an event container.
 *
 * @route POST /api/v1/events
 */
export function useCreateEvent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: {
      start_time: string
      name_override?: string
      briefing?: string
      banner_image_url?: string
      max_slots?: number
      registration_locked?: boolean
    }) => {
      const { data } = await api.post('/events', body)
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['events'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
    },
  })
}

/**
 * Admin: delete an event.
 *
 * @route DELETE /api/v1/events/:id
 */
export function useDeleteEvent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (id: string) => {
      await api.delete(`/events/${id}`)
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['events'] }),
  })
}

/**
 * Admin: publish a CMS announcement (optionally pushed to Discord).
 *
 * @route POST /api/v1/cms/announcements
 */
export function usePublishAnnouncement() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: {
      title: string
      body: string
      tag?: string
      is_pinned?: boolean
      push_to_discord?: boolean
      status?: string
    }) => {
      const { data } = await api.post('/cms/announcements', body)
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['announcements'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
    },
  })
}

/**
 * Generate a one-time Arma identity link code for the current user.
 *
 * @route POST /api/v1/me/link
 */
export function useGenerateLinkCode() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => (await api.post<{ code: string; expires_at: string }>('/me/link')).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['me', 'link'] }),
  })
}

/**
 * Unlink the caller's Arma identity.
 *
 * @route DELETE /api/v1/me/link
 */
export function useUnlinkArma() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => (await api.delete('/me/link')).data,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['me'] })
      qc.invalidateQueries({ queryKey: ['me', 'link'] })
    },
  })
}

/**
 * Admin: issue an RCON action to a server (restart / change map / kick / custom).
 *
 * @route POST /api/v1/admin/servers/:serverId/rcon
 */
export function useServerRcon() {
  return useMutation({
    mutationFn: async ({
      serverId,
      action,
      map,
      command,
    }: {
      serverId: string
      action: 'restart' | 'change_map' | 'kick' | 'custom'
      map?: string
      command?: string
    }) => {
      const { data } = await api.post(`/admin/servers/${serverId}/rcon`, {
        action,
        map,
        command,
      })
      return data
    },
  })
}

/**
 * Solve a mortar fire mission for the given firing point and target.
 *
 * @route POST /api/v1/fire-missions/solve
 */
export function useSolveFireMission() {
  return useMutation({
    mutationFn: async (body: {
      weapon_system?: string
      fp_x: number
      fp_y: number
      tgt_x: number
      tgt_y: number
    }) => {
      const { data } = await api.post<FireSolution>('/fire-missions/solve', body)
      return data
    },
  })
}

/**
 * Archive or unarchive a mission (author or admin; T-130.6). The server accepts only
 * these two status transitions through PATCH — the review lifecycle (submit/approve/
 * reject) keeps its own routes. 409 while attached to an upcoming event.
 *
 * @route PATCH /api/v1/missions/:id
 */
export function useSetMissionStatus(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (status: 'archived' | 'draft') => {
      const { data } = await api.patch(`/missions/${id}`, { status })
      return data
    },
    // Prefix-invalidates both the library lists (['missions', scope, filters]) and
    // this mission's dossier (['missions', id]).
    onSuccess: () => qc.invalidateQueries({ queryKey: ['missions'] }),
  })
}

/**
 * Soft-delete a mission (author or admin; T-130.6). Recoverable by an operator; the
 * server refuses (409) while the mission is attached to any event.
 *
 * @route DELETE /api/v1/missions/:id
 */
export function useDeleteMission(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      await api.delete(`/missions/${id}`)
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['missions'] }),
  })
}

/**
 * Admin: change a member's role.
 *
 * @route PATCH /api/v1/admin/users/:discordId
 */
export function useUpdateUserRole() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ discordId, role }: { discordId: string; role: string }) => {
      const { data } = await api.patch(`/admin/users/${discordId}`, { role })
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['personnel'] }),
  })
}

/**
 * Admin: ban a member with an optional reason.
 *
 * @route POST /api/v1/admin/users/:discordId/ban
 */
export function useBanUser() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ discordId, reason }: { discordId: string; reason?: string }) => {
      const { data } = await api.post(`/admin/users/${discordId}/ban`, { reason })
      return data
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['personnel'] }),
  })
}
