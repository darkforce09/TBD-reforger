// Mission compiler (Ultra Plan §8) — traverse the normalized state mirror into the
// `json_payload` SUPERSET saved to a MissionVersion. The `orbat[]` block matches exactly
// what internal/handlers/events.go `parseOrbatTemplate` reads (faction/callsign/squad +
// slots[{role,loadout,tag}]), so attaching the mission to an event still auto-builds ORBAT.
// Everything else is additive. An `editor` block carries the full normalized graph (positions,
// folders) the backend ignores but the editor reloads losslessly — orbat[] alone has no
// coordinates. Pure + synchronous (a few hundred entities → sub-ms; no worker needed).

// Worker-safe imports (T-066): import from the leaf modules, NOT the `@/features/tactical-map`
// barrel — the barrel re-exports <TacticalMap> (Deck.gl/React), which throws at module-eval in a
// Web Worker. `coords/terrains.ts` is pure; `MapSnapshot` is a type-only import (fully erased).
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { MapSnapshot } from '@/features/tactical-map/state/useMapStore'

export interface OrbatSlot {
  role: string
  loadout: string
  tag: string
}
export interface OrbatSquad {
  faction: string
  callsign: string
  squad: string
  slots: OrbatSlot[]
}

export interface MissionPayload {
  schemaVersion: 1
  map: { terrain: string; bounds: [number, number, number, number] }
  environment: Record<string, unknown>
  /** Backend ORBAT contract. Omitted on Save Version (T-062.1.1) to halve the payload —
   *  the server derives it from `editor` when absent (services.ParseOrbatTemplate). Export
   *  (compileMission) still includes it. */
  orbat?: OrbatSquad[]
  loadouts: Record<string, unknown>
  objectives: unknown[]
  vehicles: unknown[]
  markers: unknown[]
  /** Editor-only fidelity block — the lossless reload source and (when `orbat` is omitted)
   *  the server's ORBAT-derivation source. */
  editor: {
    factions: unknown[]
    squads: unknown[]
    slots: unknown[]
    editorLayers: unknown[]
  }
}

/** Compile knobs (T-062.1.1). `omitOrbat` drops the redundant top-level `orbat[]` from the
 *  payload (Save Version) — slots already live in `editor.slots`, and the server re-derives
 *  ORBAT from the editor graph. Export leaves it false so the file is self-contained. */
export interface CompileOptions {
  omitOrbat?: boolean
  chunkSize?: number
}

/** Compile the store snapshot (useMapStore.getState()) into the json_payload superset. */
export function compileMission(s: MapSnapshot): MissionPayload {
  const terrainId = s.meta?.terrain ?? 'everon'
  const terrain = getTerrain(terrainId)

  // ORBAT (backend contract): factions → squads → slots, in authored order.
  const orbat: OrbatSquad[] = Object.values(s.factionsById).flatMap((faction) =>
    faction.squadIds
      .map((sid) => s.squadsById[sid])
      .filter(Boolean)
      .map((squad) => ({
        faction: faction.key,
        callsign: squad.callsign ?? '',
        squad: squad.name,
        slots: squad.slotIds
          .map((slid) => s.slotsById[slid])
          .filter(Boolean)
          .sort((a, b) => a.index - b.index)
          .map((slot) => ({
            role: slot.role,
            loadout: '', // resolved loadout name lands with the Arsenal (Phase 6)
            tag: slot.tag ?? '',
          })),
      })),
  )

  return assemblePayload(s, terrainId, terrain, orbat)
}

/** Async, chunked compile for large missions (T-060): yields to the event loop every
 *  `chunkSize` slots so the tab stays responsive at 50k+, and reports compile progress
 *  (`done`/`total` slots) for the Save Version bar. With `omitOrbat` (T-062.1.1) the
 *  redundant top-level `orbat[]` is dropped — the squad/slot pass still runs to drive the
 *  progress bar, it just doesn't materialize the array. Same payload shape otherwise. */
export async function compileMissionWithProgress(
  s: MapSnapshot,
  onProgress?: (done: number, total: number) => void,
  options?: CompileOptions,
): Promise<MissionPayload> {
  const terrainId = s.meta?.terrain ?? 'everon'
  const terrain = getTerrain(terrainId)
  const chunkSize = options?.chunkSize ?? 5000
  const buildOrbat = !options?.omitOrbat

  const orbat: OrbatSquad[] | undefined = buildOrbat ? [] : undefined
  let processed = 0
  let sinceYield = 0
  const total = Object.keys(s.slotsById).length
  onProgress?.(0, total)

  for (const faction of Object.values(s.factionsById)) {
    for (const sid of faction.squadIds) {
      const squad = s.squadsById[sid]
      if (!squad) continue
      const slots = squad.slotIds
        .map((slid) => s.slotsById[slid])
        .filter(Boolean)
        .sort((a, b) => a.index - b.index)
      if (orbat) {
        orbat.push({
          faction: faction.key,
          callsign: squad.callsign ?? '',
          squad: squad.name,
          slots: slots.map((slot) => ({ role: slot.role, loadout: '', tag: slot.tag ?? '' })),
        })
      }
      processed += slots.length
      sinceYield += slots.length
      // Yield to the event loop every ~chunkSize slots so the tab stays responsive.
      if (sinceYield >= chunkSize) {
        sinceYield = 0
        onProgress?.(Math.min(processed, total), total)
        await new Promise((r) => setTimeout(r))
      }
    }
  }
  onProgress?.(total, total)

  return assemblePayloadWithProgress(s, terrainId, terrain, orbat, chunkSize)
}

/** Build the large `editor.slots` array in chunks, yielding so the final assembly doesn't
 *  block (the sync `Object.values(slotsById)` over 300k entries is otherwise a long stall
 *  right after compile % hits 100%). Other maps are small. T-060.1. */
async function chunkedValues<T>(byId: Record<string, T>, chunkSize: number): Promise<T[]> {
  const keys = Object.keys(byId)
  const out: T[] = new Array(keys.length)
  for (let i = 0; i < keys.length; i++) {
    out[i] = byId[keys[i]]
    if ((i + 1) % chunkSize === 0) await new Promise((r) => setTimeout(r))
  }
  return out
}

async function assemblePayloadWithProgress(
  s: MapSnapshot,
  terrainId: string,
  terrain: { width: number; height: number },
  orbat: OrbatSquad[] | undefined,
  chunkSize: number,
): Promise<MissionPayload> {
  const slots = await chunkedValues(s.slotsById, chunkSize)
  return {
    schemaVersion: 1,
    map: { terrain: terrainId, bounds: [0, 0, terrain.width, terrain.height] },
    environment: { ...(s.meta?.environment ?? {}) },
    ...(orbat ? { orbat } : {}),
    loadouts: { ...s.loadoutsById },
    objectives: Object.values(s.objectivesById),
    vehicles: Object.values(s.vehiclesById),
    markers: Object.values(s.markersById),
    editor: {
      factions: Object.values(s.factionsById),
      squads: Object.values(s.squadsById),
      slots,
      editorLayers: Object.values(s.editorLayersById),
    },
  }
}

/** Build the version-POST request body as a Blob, streaming the large `editor.slots` array
 *  (T-060.1.2). axios `JSON.stringify`-ing the whole `{ semver, payload, editor_notes }` graph in
 *  one synchronous call @ ~300k slots (100MB+) blocks/OOMs the tab → ERR_NETWORK. Here only the
 *  dominant `editor.slots` array is stringified in yielding chunks; everything else
 *  (orbat/loadouts/small maps/editor.factions·squads·editorLayers) stringifies normally. Parts are
 *  collected into a string[] and handed to the Blob constructor, so the bytes are concatenated in
 *  native code — we never hold one giant joined JS string. Posting the Blob means axios sends it
 *  as-is (no second graph stringify). The JSON shape matches createVersionInput exactly. */
export async function buildVersionBlob(
  semver: string,
  payload: MissionPayload,
  notes: string,
  onProgress?: (done: number, total: number) => void,
  chunkSize = 5000,
): Promise<Blob> {
  const { editor, ...rest } = payload
  const slots = editor.slots
  // restBody: payload minus `editor`, with its closing brace dropped so we can append `,"editor":…`.
  // rest always carries schemaVersion/map/… so it is never bare `{}`.
  const restJson = JSON.stringify(rest)
  const restBody = restJson.slice(0, -1)
  const restHasFields = restBody.length > 1 // more than just "{"
  // editorMeta: the small editor maps, inner content only (braces stripped).
  const editorMeta = JSON.stringify({
    factions: editor.factions,
    squads: editor.squads,
    editorLayers: editor.editorLayers,
  }).slice(1, -1)

  const parts: string[] = [
    `{"semver":${JSON.stringify(semver)},"editor_notes":${JSON.stringify(notes)},"payload":`,
    restBody,
    restHasFields ? ',' : '',
    '"editor":{',
    editorMeta,
    editorMeta ? ',' : '',
    '"slots":[',
  ]

  const total = slots.length
  onProgress?.(0, total)
  for (let i = 0; i < total; i += chunkSize) {
    const end = Math.min(i + chunkSize, total)
    let chunk = ''
    for (let j = i; j < end; j++) chunk += (j > 0 ? ',' : '') + JSON.stringify(slots[j])
    parts.push(chunk)
    onProgress?.(end, total)
    // Worker-safe yield (T-066): bare macrotask. yieldToUi's rAF paint-gate is meaningless in a
    // worker (nothing to paint), and `requestAnimationFrame`/`document` aren't defined there.
    await new Promise((r) => setTimeout(r, 0))
  }
  onProgress?.(total, total)

  parts.push(']}', '}', '}') // close slots array → editor obj → payload obj → body obj
  return new Blob(parts, { type: 'application/json' })
}

/** Shared payload assembly for the sync compiler (small missions / export). */
function assemblePayload(
  s: MapSnapshot,
  terrainId: string,
  terrain: { width: number; height: number },
  orbat: OrbatSquad[],
): MissionPayload {
  return {
    schemaVersion: 1,
    map: { terrain: terrainId, bounds: [0, 0, terrain.width, terrain.height] },
    environment: { ...(s.meta?.environment ?? {}) },
    orbat,
    loadouts: { ...s.loadoutsById },
    objectives: Object.values(s.objectivesById),
    vehicles: Object.values(s.vehiclesById),
    markers: Object.values(s.markersById),
    editor: {
      factions: Object.values(s.factionsById),
      squads: Object.values(s.squadsById),
      slots: Object.values(s.slotsById),
      editorLayers: Object.values(s.editorLayersById),
    },
  }
}
