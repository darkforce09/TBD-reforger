// camelCase mod-export envelope (Ultra Plan §8) — mirrors the server's `missionJSON`
// (internal/handlers/missions.go buildMissionDoc), wrapping the compiled json_payload so a
// local "Export" download is shape-compatible with GET /missions/:id/export (what the Arma
// mod consumes). Fields the editor doesn't hold locally (gameMode/maxPlayers/armory) default
// to empty — the server fills them authoritatively from the DB on its own /export.

import type { MissionMeta } from '@/features/tactical-map'
import type { MissionPayload } from './compile'

export interface MissionExport {
  schemaVersion: 1
  missionId: string
  title: string
  terrain: string
  gameMode: string
  weather: string
  timeOfDay: string
  maxPlayers: number
  version: string
  briefing: string
  armory: unknown[]
  payload: MissionPayload
  exportedAt: string
}

export function toMissionExport(
  meta: MissionMeta | null,
  payload: MissionPayload,
  version: string,
): MissionExport {
  return {
    schemaVersion: 1,
    missionId: meta?.id ?? '',
    title: meta?.title ?? 'Untitled Mission',
    terrain: meta?.terrain ?? 'everon',
    gameMode: '',
    weather: meta?.environment?.weather ?? 'clear',
    timeOfDay: meta?.environment?.time ?? '06:00',
    maxPlayers: 0,
    version,
    briefing: '',
    armory: [],
    payload,
    exportedAt: new Date().toISOString(),
  }
}
