import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api/client'
import { useAuthStore } from '@/store/useAuthStore'
import type { FireSolution } from '@/types/api'

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

export function useRegisterEvent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (eventId: string) => {
      const { data } = await api.post(`/events/${eventId}/register`, {})
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['events'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
      qc.invalidateQueries({ queryKey: ['deployments'] })
    },
  })
}

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

export function useCreateEvent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: {
      mission_id: string
      start_time: string
      max_slots: number
      name_override?: string
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

export function useDeleteEvent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (id: string) => {
      await api.delete(`/events/${id}`)
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['events'] }),
  })
}

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

export function useGenerateLinkCode() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => (await api.post<{ code: string; expires_at: string }>('/me/link')).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['me', 'link'] }),
  })
}

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
