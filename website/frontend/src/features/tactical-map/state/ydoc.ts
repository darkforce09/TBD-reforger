// The Y.Doc — the editor's single source of truth (Ultra Plan §2.3). Top-level
// Y.Maps named meta + the eight entity maps; each entity map is Y.Map<ID, Y.Map>
// (an entity is a nested Y.Map). ID-keyed maps (never Y.Array of objects) make
// concurrent insert/delete/reparent commute — the basis for ADR-3 multiplayer.
//
// All writes go through transact(...) with LOCAL_ORIGIN so (a) one user gesture is
// one undo step and (b) Y.UndoManager can track only local edits (state/undo.ts).

import * as Y from 'yjs'
import { ENTITY_MAPS } from './schema'
import type { ClipboardSlot, EditorLayer, ID, MissionMeta, Slot } from './schema'
import { getTerrain } from '../coords/terrains'
import type { TerrainId } from '../coords/terrains'
import { yieldToUi } from './yieldToUi'

const VALID_TERRAINS: ReadonlySet<MissionMeta['terrain']> = new Set([
  'everon',
  'arland',
  'custom',
])

const clamp = (n: number, lo: number, hi: number): number => Math.min(Math.max(n, lo), hi)

/** Origin tag stamped on every local mutation; tracked by the UndoManager. */
export const LOCAL_ORIGIN = 'local-user'

/** Origin for non-user seeding (defaults) — deliberately NOT undo-tracked. */
export const INIT_ORIGIN = 'init'

export type EntityMapName = (typeof ENTITY_MAPS)[number]

export interface MissionDoc {
  doc: Y.Doc
  meta: Y.Map<unknown>
  /** The eight entity maps, each Y.Map<ID, Y.Map>. */
  entities: Record<EntityMapName, Y.Map<Y.Map<unknown>>>
}

export function createMissionDoc(): MissionDoc {
  const doc = new Y.Doc()
  const meta = doc.getMap('meta')
  const entities = {} as Record<EntityMapName, Y.Map<Y.Map<unknown>>>
  for (const name of ENTITY_MAPS) {
    entities[name] = doc.getMap(name) as Y.Map<Y.Map<unknown>>
  }
  return { doc, meta, entities }
}

/** Every shared type the UndoManager / observers should scope to. */
export function trackedTypes(md: MissionDoc): Y.AbstractType<unknown>[] {
  return [md.meta, ...ENTITY_MAPS.map((n) => md.entities[n])] as unknown as Y.AbstractType<unknown>[]
}

/** Run a mutation as a single local transaction (one undo step). */
export function transact(md: MissionDoc, fn: () => void): void {
  md.doc.transact(fn, LOCAL_ORIGIN)
}

/** Plain object -> nested Y.Map (complex fields stored as opaque JSON values).
 *  Exported so the v2 persistence layer (mission-creator/persistence) restores chunked
 *  slots with the exact same conversion the hydrate path uses (T-062.1). */
export function entityToYMap(entity: Record<string, unknown>): Y.Map<unknown> {
  const ym = new Y.Map<unknown>()
  for (const [k, v] of Object.entries(entity)) ym.set(k, v)
  return ym
}

const newId = (): ID =>
  typeof crypto !== 'undefined' && crypto.randomUUID
    ? crypto.randomUUID()
    : `id-${Math.random().toString(36).slice(2)}-${Date.now()}`

// ── Actions ─────────────────────────────────────────────────────────────────
// Each wraps its writes in one transact() so the gesture undoes atomically.

/** Ensure a default faction + squad exist; returns the squad id to attach to. */
export function ensureDefaultSquad(md: MissionDoc): ID {
  const { factions, squads } = md.entities
  let factionId = [...factions.keys()][0]
  if (!factionId) {
    factionId = newId()
    factions.set(
      factionId,
      entityToYMap({ id: factionId, key: 'BLUFOR', name: 'BLUFOR', squadIds: [] }),
    )
  }
  let squadId = [...squads.keys()][0]
  if (!squadId) {
    squadId = newId()
    squads.set(
      squadId,
      entityToYMap({
        id: squadId,
        factionId,
        callsign: 'Test',
        name: 'Test Squad',
        slotIds: [],
      }),
    )
    const faction = factions.get(factionId)!
    faction.set('squadIds', [...(faction.get('squadIds') as ID[]), squadId])
  }
  return squadId
}

/** Ensure at least one Outliner folder exists; returns the id to file entities into.
 *  No transact() of its own — call inside an existing transaction (e.g. addSlot). */
export function ensureDefaultLayer(md: MissionDoc): ID {
  const { editorLayers } = md.entities
  let layerId = [...editorLayers.keys()][0]
  if (!layerId) {
    layerId = newId()
    editorLayers.set(
      layerId,
      entityToYMap({ id: layerId, name: 'Default Layer', parentId: null, entityIds: [] }),
    )
  }
  return layerId
}

/** Add a slot at a world position. One transaction (one undo step) that creates the
 *  Slot with Arma defaults, attaches it to a squad (for the ORBAT export contract),
 *  and files its id under an EditorLayer (the Outliner folder it appears in — the
 *  active layer, or the default layer as a fallback). */
export function addSlot(
  md: MissionDoc,
  position: { x: number; y: number },
  opts?: { squadId?: ID; layerId?: ID; role?: string; tag?: string; assetId?: string },
): ID {
  let id = ''
  transact(md, () => {
    const targetSquad = opts?.squadId ?? ensureDefaultSquad(md)
    const targetLayer = opts?.layerId ?? ensureDefaultLayer(md)
    const { slots, squads, editorLayers } = md.entities
    const squad = squads.get(targetSquad)!
    const slotIds = squad.get('slotIds') as ID[]
    id = newId()
    const slot: Slot = {
      id,
      squadId: targetSquad,
      index: slotIds.length,
      role: opts?.role ?? 'Rifleman',
      ...(opts?.tag ? { tag: opts.tag } : {}),
      ...(opts?.assetId ? { assetId: opts.assetId } : {}),
      position: { x: position.x, y: position.y, z: 0, rotation: 0 },
      stance: 'stand',
      loadoutId: null,
    }
    slots.set(id, entityToYMap(slot as unknown as Record<string, unknown>))
    squad.set('slotIds', [...slotIds, id])
    const layer = editorLayers.get(targetLayer)
    if (layer) layer.set('entityIds', [...(layer.get('entityIds') as ID[]), id])
  })
  return id
}

/** Distance (m) a paste is offset from its originals when the cursor is off-map. */
const PASTE_NUDGE = 20

/** Paste copied slots (Ctrl+V, T-056) in ONE transaction (one undo step). Positions are
 *  translated so the clip's centroid lands at `anchorAt` (the map cursor); if `anchorAt` is
 *  null (mouse off-map) the clip is nudged +PASTE_NUDGE on x/y from its originals so copies
 *  don't perfectly overlap. Each new slot re-attaches to its source squad (or the default if
 *  it was deleted) and files into `opts.layerId` (or the default layer), then x/y are clamped
 *  to the terrain bounds. Returns the new slot ids (in clip order) for selection. */
export function pasteSlots(
  md: MissionDoc,
  clip: ClipboardSlot[],
  opts?: { anchorAt?: { x: number; y: number } | null; layerId?: ID },
): ID[] {
  if (!clip.length) return []
  const terrain = getTerrain(md.meta.get('terrain') as TerrainId | undefined)
  const cx = clip.reduce((a, s) => a + s.position.x, 0) / clip.length
  const cy = clip.reduce((a, s) => a + s.position.y, 0) / clip.length
  const anchor = opts?.anchorAt
  const dx = anchor ? anchor.x - cx : PASTE_NUDGE
  const dy = anchor ? anchor.y - cy : PASTE_NUDGE
  const newIds: ID[] = []
  transact(md, () => {
    const { slots, squads, editorLayers } = md.entities
    // Batch appends: accumulate each squad's new slotIds and each layer's new entityIds in
    // local arrays (seeded once from the live Y.Array) and write each map ONCE after the loop.
    // The per-slot `[...spread, id]` form was O(n²) and froze the tab on a 10k paste (T-059).
    const squadSlotIds = new Map<ID, ID[]>()
    const layerEntityIds = new Map<ID, ID[]>()
    const slotIdsFor = (sid: ID): ID[] => {
      let arr = squadSlotIds.get(sid)
      if (!arr) {
        arr = [...(squads.get(sid)!.get('slotIds') as ID[])]
        squadSlotIds.set(sid, arr)
      }
      return arr
    }
    const entityIdsFor = (lid: ID): ID[] => {
      let arr = layerEntityIds.get(lid)
      if (!arr) {
        arr = [...(editorLayers.get(lid)!.get('entityIds') as ID[])]
        layerEntityIds.set(lid, arr)
      }
      return arr
    }
    for (const c of clip) {
      const targetSquad = squads.get(c.squadId) ? c.squadId : ensureDefaultSquad(md)
      const targetLayer =
        opts?.layerId && editorLayers.get(opts.layerId) ? opts.layerId : ensureDefaultLayer(md)
      const ids = slotIdsFor(targetSquad)
      const id = newId()
      const slot: Slot = {
        id,
        squadId: targetSquad,
        index: ids.length,
        role: c.role,
        ...(c.tag ? { tag: c.tag } : {}),
        ...(c.assetId ? { assetId: c.assetId } : {}),
        position: {
          x: clamp(c.position.x + dx, 0, terrain.width),
          y: clamp(c.position.y + dy, 0, terrain.height),
          z: c.position.z,
          rotation: c.position.rotation,
        },
        stance: c.stance,
        loadoutId: null,
      }
      slots.set(id, entityToYMap(slot as unknown as Record<string, unknown>))
      ids.push(id)
      if (editorLayers.get(targetLayer)) entityIdsFor(targetLayer).push(id)
      newIds.push(id)
    }
    for (const [sid, ids] of squadSlotIds) squads.get(sid)!.set('slotIds', ids)
    for (const [lid, ids] of layerEntityIds) editorLayers.get(lid)!.set('entityIds', ids)
  })
  return newIds
}

/** Move any positioned entity (slot/vehicle/objective) to a new world x/y. */
export function moveEntity(
  md: MissionDoc,
  mapName: EntityMapName,
  id: ID,
  position: { x: number; y: number },
): void {
  const entity = md.entities[mapName].get(id)
  if (!entity) return
  transact(md, () => {
    const prev = (entity.get('position') as Record<string, number>) ?? {}
    entity.set('position', { ...prev, x: position.x, y: position.y })
  })
}

/** Move several positioned entities by a shared world delta in ONE transaction (one
 *  undo step) — the atomic group move behind Eden's drag-to-move (Phase 7b). */
export function moveEntities(
  md: MissionDoc,
  ids: ID[],
  delta: { x: number; y: number },
): void {
  if (!ids.length) return
  const map = md.entities.slots
  transact(md, () => {
    for (const id of ids) {
      const entity = map.get(id)
      if (!entity) continue
      const prev = (entity.get('position') as Record<string, number>) ?? {}
      entity.set('position', { ...prev, x: (prev.x ?? 0) + delta.x, y: (prev.y ?? 0) + delta.y })
    }
  })
}

/** Inner remove (no transaction) — cascades children and detaches refs. Call inside
 *  an existing transaction (removeEntity / removeEntities). */
function removeEntityInner(md: MissionDoc, mapName: EntityMapName, id: ID): void {
  const { factions, squads, slots, editorLayers } = md.entities
  if (mapName === 'slots') {
    const squadId = slots.get(id)?.get('squadId') as ID | undefined
    const squad = squadId ? squads.get(squadId) : undefined
    squad?.set('slotIds', (squad.get('slotIds') as ID[]).filter((s) => s !== id))
    // Detach from whichever Outliner folder held it.
    for (const layer of editorLayers.values()) {
      const ids = layer.get('entityIds') as ID[]
      if (ids.includes(id)) layer.set('entityIds', ids.filter((e) => e !== id))
    }
  } else if (mapName === 'squads') {
    const squad = squads.get(id)
    for (const slotId of (squad?.get('slotIds') as ID[]) ?? []) slots.delete(slotId)
    const factionId = squad?.get('factionId') as ID | undefined
    const faction = factionId ? factions.get(factionId) : undefined
    faction?.set('squadIds', (faction.get('squadIds') as ID[]).filter((s) => s !== id))
  } else if (mapName === 'factions') {
    for (const squadId of (factions.get(id)?.get('squadIds') as ID[]) ?? []) {
      const squad = squads.get(squadId)
      for (const slotId of (squad?.get('slotIds') as ID[]) ?? []) slots.delete(slotId)
      squads.delete(squadId)
    }
  }
  md.entities[mapName].delete(id)
}

/** Remove an entity, cascading children (faction→squads→slots) and detaching refs. */
export function removeEntity(md: MissionDoc, mapName: EntityMapName, id: ID): void {
  transact(md, () => removeEntityInner(md, mapName, id))
}

/** Remove several entities from one map in ONE transaction (one undo step) — Eden's
 *  Delete/Backspace on a multi-selection (Phase 7b). For a multi-slot delete this batches the
 *  ref-detach work (T-062.0.1): one `slotIds` filter per affected squad and one `entityIds`
 *  filter per Outliner folder, instead of `removeEntityInner`'s per-id squad filter + full
 *  editorLayers scan (O(n·k) → O(n+k) at scale). The resulting transaction shape (squad/layer
 *  child field updates + slots structural deletes) is unchanged, so the bindings classifier
 *  still sees a `slot-remove`. */
export function removeEntities(md: MissionDoc, mapName: EntityMapName, ids: ID[]): void {
  if (!ids.length) return
  if (mapName === 'slots' && ids.length > 1) {
    const { squads, slots, editorLayers } = md.entities
    const idSet = new Set(ids)
    transact(md, () => {
      // One filter per affected squad (not per slot).
      const affectedSquads = new Set<ID>()
      for (const id of ids) {
        const sid = slots.get(id)?.get('squadId') as ID | undefined
        if (sid) affectedSquads.add(sid)
      }
      for (const sid of affectedSquads) {
        const squad = squads.get(sid)
        if (!squad) continue
        squad.set('slotIds', (squad.get('slotIds') as ID[]).filter((s) => !idSet.has(s)))
      }
      // One pass over the folders; only rewrite a folder whose entityIds actually held a deleted id.
      for (const layer of editorLayers.values()) {
        const eids = layer.get('entityIds') as ID[]
        if (eids.some((e) => idSet.has(e))) {
          layer.set('entityIds', eids.filter((e) => !idSet.has(e)))
        }
      }
      for (const id of ids) slots.delete(id)
    })
    return
  }
  transact(md, () => {
    for (const id of ids) removeEntityInner(md, mapName, id)
  })
}

/** Wipe every entity map (keeps meta). */
export function clearAll(md: MissionDoc): void {
  transact(md, () => {
    for (const name of ENTITY_MAPS) md.entities[name].clear()
  })
}

// ── Meta + structural actions (Phase 3 shell) ───────────────────────────────

const DEFAULT_META = (id: ID, title: string): MissionMeta => ({
  id,
  title,
  terrain: 'everon',
  environment: { time: '06:00', weather: 'clear', viewDistance: 1600, thermals: false },
})

/** Apply mission row fields from GET /missions/:id. Uses INIT_ORIGIN — this is a load,
 *  not a user edit, so it is neither undo-tracked nor marks the doc dirty. Title hydrate
 *  only (T-049): the PostgreSQL row is the source for title/terrain/env on open; there is
 *  no PATCH-back. Invalid terrain values are ignored; env defaults (viewDistance/thermals)
 *  are preserved by merging onto the existing environment. */
export function applyMissionRowMeta(
  md: MissionDoc,
  row: { title: string; terrain: string; time_of_day?: string; weather?: string },
): void {
  md.doc.transact(() => {
    if (row.title) md.meta.set('title', row.title)
    if (VALID_TERRAINS.has(row.terrain as MissionMeta['terrain'])) {
      md.meta.set('terrain', row.terrain)
    }
    if (row.time_of_day != null || row.weather != null) {
      const env = (md.meta.get('environment') as MissionMeta['environment']) ?? {}
      md.meta.set('environment', {
        ...env,
        ...(row.time_of_day != null ? { time: row.time_of_day } : {}),
        ...(row.weather != null
          ? { weather: row.weather as MissionMeta['environment']['weather'] }
          : {}),
      })
    }
  }, INIT_ORIGIN)
}

/** Seed meta with defaults if empty. Uses INIT_ORIGIN so it is NOT an undo step. */
export function seedMeta(md: MissionDoc, opts: { id: ID; title: string }): void {
  if (md.meta.size > 0) return
  md.doc.transact(() => {
    const m = DEFAULT_META(opts.id, opts.title)
    for (const [k, v] of Object.entries(m)) md.meta.set(k, v)
  }, INIT_ORIGIN)
}

/** Seed a default Outliner folder if none exist. INIT_ORIGIN → NOT an undo step. */
export function seedDefaultLayer(md: MissionDoc): void {
  if (md.entities.editorLayers.size > 0) return
  md.doc.transact(() => ensureDefaultLayer(md), INIT_ORIGIN)
}

/** Create a new (root or nested) Outliner folder; returns its id. */
export function addEditorLayer(
  md: MissionDoc,
  opts?: { name?: string; parentId?: ID | null },
): ID {
  const id = newId()
  transact(md, () => {
    const n = md.entities.editorLayers.size + 1
    const layer: EditorLayer = {
      id,
      name: opts?.name ?? `New Folder ${n}`,
      parentId: opts?.parentId ?? null,
      entityIds: [],
    }
    md.entities.editorLayers.set(id, entityToYMap(layer as unknown as Record<string, unknown>))
  })
  return id
}

/** Rename an Outliner folder. */
export function renameEditorLayer(md: MissionDoc, id: ID, name: string): void {
  const layer = md.entities.editorLayers.get(id)
  if (!layer) return
  transact(md, () => layer.set('name', name))
}

/** Is `nodeId` inside `ancestorId`'s subtree (or equal to it)? Walks up via parentId. */
function isLayerDescendant(md: MissionDoc, ancestorId: ID, nodeId: ID): boolean {
  const { editorLayers } = md.entities
  let cur: ID | null = nodeId
  while (cur) {
    if (cur === ancestorId) return true
    cur = (editorLayers.get(cur)?.get('parentId') as ID | null) ?? null
  }
  return false
}

/** Reparent an Outliner folder. Rejects cycles (dropping a folder into its own subtree). */
export function reparentEditorLayer(md: MissionDoc, id: ID, newParentId: ID | null): void {
  const layer = md.entities.editorLayers.get(id)
  if (!layer) return
  if (newParentId === id) return
  if (newParentId && isLayerDescendant(md, id, newParentId)) return
  transact(md, () => layer.set('parentId', newParentId))
}

/** Refile a slot into a different Outliner folder (workflow-only; squad unchanged). */
export function moveSlotToLayer(md: MissionDoc, slotId: ID, targetLayerId: ID): void {
  const { editorLayers } = md.entities
  const target = editorLayers.get(targetLayerId)
  if (!target) return
  transact(md, () => {
    for (const layer of editorLayers.values()) {
      const ids = layer.get('entityIds') as ID[]
      if (ids.includes(slotId)) layer.set('entityIds', ids.filter((e) => e !== slotId))
    }
    const tIds = target.get('entityIds') as ID[]
    if (!tIds.includes(slotId)) target.set('entityIds', [...tIds, slotId])
  })
}

/** Delete an Outliner folder AND its whole subtree — every nested folder plus all units
 *  filed in any of them — in ONE transaction (Eden expectation). No-op if it is the only
 *  layer (keep ≥1); if the subtree was every layer, a fresh default layer is reseeded so
 *  the editor is never layer-less (one undo restores the entire subtree + units). */
export function removeEditorLayer(md: MissionDoc, id: ID): void {
  const { editorLayers } = md.entities
  if (!editorLayers.get(id) || editorLayers.size <= 1) return
  // Collect the subtree: `id` plus every layer whose parent chain reaches it.
  const subtree = new Set<ID>([id])
  for (let added = true; added; ) {
    added = false
    for (const l of editorLayers.values()) {
      const lid = l.get('id') as ID
      const pid = l.get('parentId') as ID | null
      if (pid && subtree.has(pid) && !subtree.has(lid)) {
        subtree.add(lid)
        added = true
      }
    }
  }
  transact(md, () => {
    for (const layerId of subtree) {
      const layer = editorLayers.get(layerId)
      if (!layer) continue
      for (const slotId of [...(layer.get('entityIds') as ID[])]) {
        removeEntityInner(md, 'slots', slotId)
      }
      editorLayers.delete(layerId)
    }
    // Never leave the editor without a folder to file new placements into.
    if (editorLayers.size === 0) ensureDefaultLayer(md)
  })
}

/** Repopulate the Y.Doc from a compiled json_payload (Phase 9 load/hydrate). Runs as a
 *  NON-undo transaction (INIT_ORIGIN) — loading a server version is not a user edit. Prefers
 *  the lossless `editor` block (full normalized entities incl. positions + folders) our own
 *  compiler emits; falls back to reconstructing from the backend `orbat[]` (lossy: default
 *  positions, one Default Layer) for missions authored elsewhere. Title/id in meta are kept. */
export function hydrateMissionDoc(md: MissionDoc, payload: Record<string, unknown>): void {
  const p = payload ?? {}
  const editor = p.editor as
    | { factions?: unknown[]; squads?: unknown[]; slots?: unknown[]; editorLayers?: unknown[] }
    | undefined
  const map = p.map as { terrain?: string } | undefined
  const setEach = (name: EntityMapName, rows: unknown[] | undefined) => {
    for (const row of rows ?? []) {
      const r = row as Record<string, unknown>
      if (r && typeof r.id === 'string') {
        md.entities[name].set(r.id, entityToYMap(r))
      }
    }
  }

  md.doc.transact(() => {
    for (const name of ENTITY_MAPS) md.entities[name].clear()
    if (p.environment) md.meta.set('environment', p.environment)
    if (map?.terrain) md.meta.set('terrain', map.terrain)

    // Top-level export entities (same on both paths).
    setEach('objectives', p.objectives as unknown[] | undefined)
    setEach('vehicles', p.vehicles as unknown[] | undefined)
    setEach('markers', p.markers as unknown[] | undefined)
    const loadouts = p.loadouts as Record<string, unknown> | undefined
    if (loadouts) for (const v of Object.values(loadouts)) setEach('loadouts', [v])

    if (editor) {
      // Lossless path — restore the exact normalized graph (positions + folders).
      setEach('factions', editor.factions)
      setEach('squads', editor.squads)
      setEach('slots', editor.slots)
      setEach('editorLayers', editor.editorLayers)
      if (md.entities.editorLayers.size === 0) ensureDefaultLayer(md)
      return
    }

    // Lossy path — rebuild factions/squads/slots from the ORBAT contract block.
    const layerId = ensureDefaultLayer(md)
    const layer = md.entities.editorLayers.get(layerId)!
    const filed: ID[] = []
    const byKey = new Map<string, ID>()
    const squads = (p.orbat as Record<string, unknown>[] | undefined) ?? []
    for (const sq of squads) {
      const key = String(sq.faction ?? 'BLUFOR')
      let factionId = byKey.get(key)
      if (!factionId) {
        factionId = newId()
        byKey.set(key, factionId)
        md.entities.factions.set(
          factionId,
          entityToYMap({ id: factionId, key, name: key, squadIds: [] }),
        )
      }
      const faction = md.entities.factions.get(factionId)!
      const squadId = newId()
      const slotIds: ID[] = []
      const slots = (sq.slots as Record<string, unknown>[] | undefined) ?? []
      slots.forEach((sl, i) => {
        const slotId = newId()
        md.entities.slots.set(
          slotId,
          entityToYMap({
            id: slotId,
            squadId,
            index: i,
            role: String(sl.role ?? 'Rifleman'),
            ...(sl.tag ? { tag: String(sl.tag) } : {}),
            position: { x: 0, y: 0, z: 0, rotation: 0 },
            stance: 'stand',
            loadoutId: null,
          }),
        )
        slotIds.push(slotId)
        filed.push(slotId)
      })
      md.entities.squads.set(
        squadId,
        entityToYMap({
          id: squadId,
          factionId,
          callsign: String(sq.callsign ?? ''),
          name: String(sq.squad ?? 'Squad'),
          slotIds,
        }),
      )
      faction.set('squadIds', [...(faction.get('squadIds') as ID[]), squadId])
    }
    layer.set('entityIds', filed)
  }, INIT_ORIGIN)
}

/** Async, chunked variant of hydrateMissionDoc (T-060.1). Writing 300k+ slots in one
 *  transaction blocks the main thread for minutes with no progress; here the small entities
 *  go in one transaction and `editor.slots` are written in `chunkSize` batches (each its own
 *  INIT_ORIGIN transaction), yielding + reporting progress between chunks so the load overlay
 *  advances. Only the lossless `editor` path is chunked; the small/lossy `orbat`-only path (and
 *  an empty editor block) delegate to the sync hydrateMissionDoc. */
export async function hydrateMissionDocWithProgress(
  md: MissionDoc,
  payload: Record<string, unknown>,
  onProgress?: (done: number, total: number) => void,
  chunkSize = 5000,
): Promise<void> {
  const p = payload ?? {}
  const editor = p.editor as
    | { factions?: unknown[]; squads?: unknown[]; slots?: unknown[]; editorLayers?: unknown[] }
    | undefined
  const slots = editor?.slots
  // No editor block, or small enough to not need chunking → the sync path is fine.
  if (!editor || !Array.isArray(slots) || slots.length <= chunkSize) {
    hydrateMissionDoc(md, p)
    onProgress?.(slots?.length ?? 0, slots?.length ?? 0)
    return
  }

  const map = p.map as { terrain?: string } | undefined
  const setEach = (name: EntityMapName, rows: unknown[] | undefined) => {
    for (const row of rows ?? []) {
      const r = row as Record<string, unknown>
      if (r && typeof r.id === 'string') md.entities[name].set(r.id, entityToYMap(r))
    }
  }

  // Transaction 1: clear + meta + the small maps + factions/squads/layers (everything but slots).
  md.doc.transact(() => {
    for (const name of ENTITY_MAPS) md.entities[name].clear()
    if (p.environment) md.meta.set('environment', p.environment)
    if (map?.terrain) md.meta.set('terrain', map.terrain)
    setEach('objectives', p.objectives as unknown[] | undefined)
    setEach('vehicles', p.vehicles as unknown[] | undefined)
    setEach('markers', p.markers as unknown[] | undefined)
    const loadouts = p.loadouts as Record<string, unknown> | undefined
    if (loadouts) for (const v of Object.values(loadouts)) setEach('loadouts', [v])
    setEach('factions', editor.factions)
    setEach('squads', editor.squads)
    setEach('editorLayers', editor.editorLayers)
  }, INIT_ORIGIN)

  // Slots in chunks: one INIT_ORIGIN transaction per batch, yielding between.
  const total = slots.length
  onProgress?.(0, total)
  for (let i = 0; i < total; i += chunkSize) {
    const end = Math.min(i + chunkSize, total)
    md.doc.transact(() => {
      for (let j = i; j < end; j++) {
        const r = slots[j] as Record<string, unknown>
        if (r && typeof r.id === 'string') md.entities.slots.set(r.id, entityToYMap(r))
      }
    }, INIT_ORIGIN)
    onProgress?.(end, total)
    await yieldToUi()
  }

  if (md.entities.editorLayers.size === 0) {
    md.doc.transact(() => ensureDefaultLayer(md), INIT_ORIGIN)
  }
}

export function setTitle(md: MissionDoc, title: string): void {
  transact(md, () => md.meta.set('title', title))
}

export function updateEnvironment(
  md: MissionDoc,
  patch: Partial<MissionMeta['environment']>,
): void {
  transact(md, () => {
    const env = (md.meta.get('environment') as MissionMeta['environment']) ?? {}
    md.meta.set('environment', { ...env, ...patch })
  })
}

/** Edit a slot's transform numerically (Attributes Transform tab, T-049). One transact()
 *  per call → one undo step (call on blur/Enter, not every keystroke). x/y are clamped to the
 *  active terrain's bounds; z is free (manual until DEM); rotation is normalized to [0,360).
 *  Untouched / non-finite axes are left as-is. */
export function updateSlotPosition(
  md: MissionDoc,
  id: ID,
  patch: Partial<{ x: number; y: number; z: number; rotation: number }>,
): void {
  const slot = md.entities.slots.get(id)
  if (!slot) return
  const terrain = getTerrain(md.meta.get('terrain') as TerrainId | undefined)
  transact(md, () => {
    const prev = (slot.get('position') as Slot['position']) ?? { x: 0, y: 0, z: 0, rotation: 0 }
    const next = { ...prev }
    if (patch.x != null && Number.isFinite(patch.x)) next.x = clamp(patch.x, 0, terrain.width)
    if (patch.y != null && Number.isFinite(patch.y)) next.y = clamp(patch.y, 0, terrain.height)
    if (patch.z != null && Number.isFinite(patch.z)) next.z = patch.z
    if (patch.rotation != null && Number.isFinite(patch.rotation)) {
      next.rotation = ((patch.rotation % 360) + 360) % 360
    }
    slot.set('position', next)
  })
}

/** Patch scalar slot fields (role / tag / stance). */
export function updateSlot(
  md: MissionDoc,
  id: ID,
  patch: Partial<Pick<Slot, 'role' | 'tag' | 'stance'>>,
): void {
  const slot = md.entities.slots.get(id)
  if (!slot) return
  transact(md, () => {
    for (const [k, v] of Object.entries(patch)) slot.set(k, v)
  })
}

/** Create a new faction; returns its id. */
export function addFaction(md: MissionDoc): ID {
  const id = newId()
  transact(md, () => {
    const n = md.entities.factions.size + 1
    md.entities.factions.set(
      id,
      entityToYMap({ id, key: 'BLUFOR', name: `Faction ${n}`, squadIds: [] }),
    )
  })
  return id
}

/** Create a squad under a faction; returns its id. */
export function addSquad(md: MissionDoc, factionId: ID): ID {
  const id = newId()
  transact(md, () => {
    const faction = md.entities.factions.get(factionId)
    if (!faction) return
    const n = (faction.get('squadIds') as ID[]).length + 1
    md.entities.squads.set(
      id,
      entityToYMap({ id, factionId, name: `Squad ${n}`, slotIds: [] }),
    )
    faction.set('squadIds', [...(faction.get('squadIds') as ID[]), id])
  })
  return id
}
