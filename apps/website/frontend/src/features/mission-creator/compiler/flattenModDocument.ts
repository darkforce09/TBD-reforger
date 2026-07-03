// Mission compile flatten (T-092.2) — traverse the editor snapshot into the CANONICAL
// mod mission document (mission.schema.json, string schemaVersion "1.1"/"1.2"), the shape
// TBD_MissionLoader consumes. This is a THIRD artifact, distinct from the Version-POST
// editor superset (compile.ts MissionPayload, locked) and the camelCase export wrapper
// (Go buildMissionDoc). The Go twin is internal/services/mission_compile.go — it mirrors
// this traversal EXACTLY (deriveOrbatFromEditor precedent) so GET /missions/:id/compiled
// and this client-side flatten produce identical documents for the same snapshot.
//
// Coordinate mapping (locked, t092_spawn_transform_program.md):
//   editor position.x -> slot.x · position.y -> slot.z · position.z -> slot.y (optional)
//   position.rotation -> slot.headingDeg
//
// Fields the editor never authors (zones/flow/winConditions/meta.templateId/playerRange/
// faction presetId) are SYNTHESIZED (operator decision 2026-07-03) so the document always
// validates and the mod always loads: per-faction spawn zones at the faction's slot
// centroid, plus fixed flow/winConditions defaults below.
//
// @contract mission.schema.json#/
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { Slot, Squad } from '@/features/tactical-map/state/schema'
import type { MapSnapshot } from '@/features/tactical-map/state/useMapStore'
import kitAliases from '@/types/contract/kit-aliases.json'

export interface ModSlot {
  id: string
  faction: string
  groupCallsign: string
  role: string
  kit: string
  x: number
  z: number
  y?: number
  headingDeg: number
}

export interface ModOrbatRole {
  slot: string
  kit: string
  count: number
}

export interface ModOrbatGroup {
  callsign: string
  type: string
  roles: ModOrbatRole[]
}

export interface ModZone {
  id: string
  type: string
  label?: string
  faction?: string
  shape: { circle: { x: number; z: number; r: number } } | { polygon: [number, number][] }
}

export interface ModMissionDocument {
  schemaVersion: '1.1' | '1.2'
  meta: {
    id: string
    name: string
    author?: string
    terrain: string
    templateId: string
    playerRange: [number, number]
  }
  environment?: { dateTime?: string; weatherPreset?: string }
  factions: { key: string; displayName: string; presetId: string; tickets?: number }[]
  orbat: Record<string, { groups: ModOrbatGroup[] }>
  slots: ModSlot[]
  zones: ModZone[]
  flow: { briefingSeconds: number; safeStartSeconds: number; timeLimitSeconds: number; jip: string }
  winConditions: { mode: string; endOn: string[] }
}

export interface FlattenOptions {
  /** Mission max_players from the mission row; playerRange upper bound. */
  maxPlayers?: number
  /** Author label for meta.author. */
  author?: string
}

interface KitAliasesFile {
  kits: { alias: string; resourceName: string }[]
  factionDefaults: Record<string, { kit: string; preset: string }>
  fallbackFaction: string
}

const ALIASES = kitAliases as KitAliasesFile
const RESOURCE_TO_KIT = new Map(ALIASES.kits.map((k) => [k.resourceName, k.alias]))

/** Fixed date anchor for environment.dateTime — the editor only authors HH:MM. */
const DATE_ANCHOR = '1989-06-14'
const SPAWN_ZONE_RADIUS_M = 150
const FLOW_DEFAULTS = {
  briefingSeconds: 600,
  safeStartSeconds: 300,
  timeLimitSeconds: 5400,
  jip: 'until_safestart_end',
}
const WIN_DEFAULTS = { mode: 'attrition', endOn: ['time_limit', 'faction_eliminated'] }

/** mission.schema.json factionKey / templateId pattern: ^[a-z][a-z0-9_]*$. */
function slugKey(raw: string, fallback: string): string {
  const s = raw
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, '_')
    .replace(/^_+|_+$/g, '')
  const keyed = /^[a-z]/.test(s) ? s : s ? `f_${s}` : ''
  return keyed || fallback
}

/** meta.id pattern: ^msn_[a-z0-9]+$ — mission UUIDs reduce to lowercase hex. */
function missionDocId(editorMetaId: string | undefined): string {
  const hex = (editorMetaId ?? '').toLowerCase().replace(/[^a-z0-9]+/g, '')
  return `msn_${hex || 'editor'}`
}

function normalizeHeading(rotation: number): number {
  if (!Number.isFinite(rotation)) return 0
  return ((rotation % 360) + 360) % 360
}

/** ResourceName -> kit: alias; unmapped/absent assetIds get the faction default kit. */
function kitForSlot(slot: Slot, factionKey: string): { kit: string; fellBack: boolean } {
  if (slot.assetId) {
    const alias = RESOURCE_TO_KIT.get(slot.assetId)
    if (alias) return { kit: alias, fellBack: false }
  }
  const defaults =
    ALIASES.factionDefaults[factionKey] ?? ALIASES.factionDefaults[ALIASES.fallbackFaction]
  return { kit: defaults.kit, fellBack: true }
}

/**
 * Flatten the editor snapshot into the mod-native mission document.
 *
 * Slot ids are deterministic — `{faction}:{groupCallsign}:{role}:{index}` with index the
 * per-(faction, callsign, role) occurrence counter in authored (slot.index) order — the
 * flatten-orbat-slots.mjs convention, so re-compiling an unchanged mission yields
 * byte-identical slot identity for the mod roster.
 *
 * The optional slot `y` (schema 1.2) is emitted only for a finite, non-zero editor
 * position.z: pre-DEM missions carry z=0 placeholders on every slot, and emitting those
 * would pin all spawns to sea level instead of the terrain surface fallback.
 *
 * Throws when the snapshot has no placed slots — a 1.1/1.2 document requires slots[].
 */
// eslint-disable-next-line complexity -- single authoritative traversal: faction/squad/slot walk + orbat aggregation + zone synthesis mirror the Go twin one-for-one
export function flattenEditorToModDocument(
  s: MapSnapshot,
  options?: FlattenOptions,
): ModMissionDocument {
  const terrainId = s.meta?.terrain ?? 'everon'
  const terrain = getTerrain(terrainId)

  const factions = Object.values(s.factionsById)
  const slots: ModSlot[] = []
  const orbat: Record<string, { groups: ModOrbatGroup[] }> = {}
  const docFactions: ModMissionDocument['factions'] = []
  const centroids = new Map<string, { sx: number; sz: number; n: number }>()
  let fallbackKits = 0
  let anyY = false

  for (const faction of factions) {
    const factionKey = slugKey(faction.key, 'faction')
    const defaults =
      ALIASES.factionDefaults[factionKey] ?? ALIASES.factionDefaults[ALIASES.fallbackFaction]
    const groups: ModOrbatGroup[] = []

    for (const sid of faction.squadIds) {
      const squad: Squad | undefined = s.squadsById[sid]
      if (!squad) continue
      const squadSlots = squad.slotIds
        .map((slid) => s.slotsById[slid])
        .filter(Boolean)
        .sort((a, b) => a.index - b.index)
      if (squadSlots.length === 0) continue

      const callsign = squad.callsign || squad.name
      const roleCounters = new Map<string, number>()
      const roles: ModOrbatRole[] = []
      const roleIndex = new Map<string, number>()

      for (const slot of squadSlots) {
        const occurrence = roleCounters.get(slot.role) ?? 0
        roleCounters.set(slot.role, occurrence + 1)
        const { kit, fellBack } = kitForSlot(slot, factionKey)
        if (fellBack) fallbackKits++

        const existing = roleIndex.get(slot.role)
        if (existing === undefined) {
          roleIndex.set(slot.role, roles.length)
          roles.push({ slot: slot.role, kit, count: 1 })
        } else {
          roles[existing].count++
        }

        const x = slot.position.x
        const z = slot.position.y // editor y (map north axis) -> mod z
        const y = slot.position.z // editor z (elevation) -> mod y (optional)
        const emitY = Number.isFinite(y) && y !== 0
        if (emitY) anyY = true

        slots.push({
          id: `${factionKey}:${callsign}:${slot.role}:${occurrence}`,
          faction: factionKey,
          groupCallsign: callsign,
          role: slot.role,
          kit,
          x,
          z,
          ...(emitY ? { y } : {}),
          headingDeg: normalizeHeading(slot.position.rotation),
        })

        const c = centroids.get(factionKey) ?? { sx: 0, sz: 0, n: 0 }
        c.sx += x
        c.sz += z
        c.n++
        centroids.set(factionKey, c)
      }

      groups.push({ callsign, type: 'rifle_squad', roles })
    }

    if (groups.length > 0) orbat[factionKey] = { groups }
    docFactions.push({
      key: factionKey,
      displayName: faction.name || factionKey,
      presetId: defaults.preset,
      tickets: 0,
    })
  }

  if (slots.length === 0) {
    throw new Error('cannot compile a mod mission document without placed slots')
  }
  if (fallbackKits > 0) {
    console.warn(
      `[flattenModDocument] ${fallbackKits} slot(s) had no kit-alias mapping — faction default kit used`,
    )
  }

  // Schema requires >= 2 factions; pad a stub opposing faction for single-faction drafts.
  if (docFactions.length < 2) {
    const missing = docFactions.some((f) => f.key === 'opfor') ? 'blufor' : 'opfor'
    const defaults = ALIASES.factionDefaults[missing] ?? ALIASES.factionDefaults[ALIASES.fallbackFaction]
    docFactions.push({ key: missing, displayName: missing.toUpperCase(), presetId: defaults.preset, tickets: 0 })
  }

  // Synthesized spawn zone per faction with slots (circle at its slot centroid).
  const zones: ModZone[] = [...centroids.entries()].map(([factionKey, c]) => ({
    id: `z_spawn_${factionKey}`,
    type: 'spawn',
    faction: factionKey,
    shape: {
      circle: {
        x: Math.round((c.sx / c.n) * 10) / 10,
        z: Math.round((c.sz / c.n) * 10) / 10,
        r: SPAWN_ZONE_RADIUS_M,
      },
    },
  }))

  const env = s.meta?.environment
  const slotCount = slots.length

  return {
    schemaVersion: anyY ? '1.2' : '1.1',
    meta: {
      id: missionDocId(s.meta?.id),
      name: s.meta?.title || 'Untitled Mission',
      ...(options?.author ? { author: options.author } : {}),
      terrain: slugKey(
        terrainId === 'custom' ? (s.meta?.customTerrainName ?? 'custom') : terrainId,
        'everon',
      ),
      templateId: 'editor_v1',
      playerRange: [1, Math.max(1, options?.maxPlayers ?? slotCount)],
    },
    ...(env
      ? {
          environment: {
            ...(env.time ? { dateTime: `${DATE_ANCHOR}T${env.time}:00Z` } : {}),
            ...(env.weather ? { weatherPreset: env.weather } : {}),
          },
        }
      : {}),
    factions: docFactions,
    orbat,
    slots,
    zones:
      zones.length > 0
        ? zones
        : [
            {
              id: 'z_bounds',
              type: 'boundary',
              shape: {
                polygon: [
                  [0, 0],
                  [terrain.width, 0],
                  [terrain.width, terrain.height],
                  [0, terrain.height],
                ],
              },
            },
          ],
    flow: { ...FLOW_DEFAULTS },
    winConditions: { mode: WIN_DEFAULTS.mode, endOn: [...WIN_DEFAULTS.endOn] },
  }
}
