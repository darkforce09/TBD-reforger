// The mission document mutators (Ultra Plan §2.3), post-flip (T-145 Phase 3.2 F3): the authoritative
// document is the wasm `yrs` core behind the `WasmMissionDoc` shell (state/wasmDoc.ts) — there is no
// Y.Doc. Each mutator keeps its pre-flip signature so the ~31 consumer call sites are untouched; the
// body mints id(s) in JS (crypto stays JS), reads the Zustand store for context, samples terrain z
// JS-side, calls the Rust twin (`md.wasm.<op>`), then resyncs the store from the whole snapshot and
// fires a change notification.
//
// F3 is correctness-first: every mutator does a whole-snapshot resync (`_applySnapshot(md.snapshot())`,
// proven byte-equal to the old docToSnapshot). O(k) store fast-paths (the `_patch*` methods) are F3.1.
//
// Origins: a user gesture notifies `'local'` (undo-tracked in the Rust doc, drives dirty + persistence);
// a load/seed (boot, hydrate, conflict-adopt) is bracketed with `setOriginInit(true/false)` and notifies
// `'init'` (untracked, not dirty). This mirrors the pre-flip LOCAL_ORIGIN / INIT_ORIGIN split.

import { ENTITY_MAPS } from './schema'
import type { ClipboardSlot, ID, MissionMeta } from './schema'
import { getTerrain } from '../coords/terrains'
import type { TerrainId } from '../coords/terrains'
import { sampleElevation, isDemReady } from '../dem'
import { useMapStore } from './useMapStore'
import { createWasmMissionDoc, type WasmMissionDoc } from './wasmDoc'

/** The post-flip document handle: the wasm-owning shell. Consumers keep passing `md` opaquely. */
export type MissionDoc = WasmMissionDoc
export const createMissionDoc = createWasmMissionDoc

/** The eight entity-map names (kept for the `removeEntity`/`removeEntities` signature). */
export type EntityMapName = (typeof ENTITY_MAPS)[number]

/** Terrain elevation at (x,y) — 0 when the DEM is not ready or degraded (T-091.2). */
function terrainZ(x: number, y: number): number {
  return isDemReady() ? sampleElevation(x, y) : 0
}

const clamp = (n: number, lo: number, hi: number): number => Math.min(Math.max(n, lo), hi)

const newId = (): ID =>
  typeof crypto !== 'undefined' && crypto.randomUUID
    ? crypto.randomUUID()
    : `id-${Math.random().toString(36).slice(2)}-${Date.now()}`

/** Resync the store from the whole wasm snapshot + fire the change signal. `'local'` = an undoable
 *  user gesture (dirty + persist); `'init'` = a load/seed (neither). */
function commit(md: MissionDoc, origin: 'local' | 'init'): void {
  useMapStore.getState()._applySnapshot(md.snapshot())
  md.notifyChange(origin)
}

// ── Default-entity helpers ────────────────────────────────────────────────────
// Read the store (the committed model) to decide what already exists; mint + create via the Rust
// twin when it doesn't. The caller's init-mode bracket (if any) decides LOCAL vs INIT tracking.

/** Ensure a default faction + squad exist; returns the squad id to attach to. */
export function ensureDefaultSquad(md: MissionDoc): ID {
  const st = useMapStore.getState()
  let factionId = Object.keys(st.factionsById)[0]
  if (!factionId) {
    factionId = newId()
    md.wasm?.add_faction(factionId, 'BLUFOR', 'BLUFOR')
  }
  let squadId = Object.keys(st.squadsById)[0]
  if (!squadId) {
    squadId = newId()
    md.wasm?.add_squad(squadId, factionId, 'Test Squad', 'Test')
  }
  return squadId
}

/** Ensure at least one Outliner folder exists; returns the id to file entities into. */
export function ensureDefaultLayer(md: MissionDoc): ID {
  const st = useMapStore.getState()
  let layerId = Object.keys(st.editorLayersById)[0]
  if (!layerId) {
    layerId = newId()
    md.wasm?.add_editor_layer(layerId, 'Default Layer', undefined)
  }
  return layerId
}

// ── Actions ─────────────────────────────────────────────────────────────────

/** Add a slot at a world position: create the Slot with Arma defaults, attach it to a squad (the
 *  ORBAT export contract), and file it under an EditorLayer (the active layer, or a default). On an
 *  empty doc this fans out to add_faction + add_squad + add_editor_layer + add_slot — each its own
 *  Rust transaction, so the first placement is a multi-step undo (see the plan's undo-granularity
 *  note); later placements are a single add_slot. */
export function addSlot(
  md: MissionDoc,
  position: { x: number; y: number },
  opts?: { squadId?: ID; layerId?: ID; role?: string; tag?: string; assetId?: string },
): ID {
  if (!md.wasm) return ''
  const st = useMapStore.getState()
  const targetSquad = opts?.squadId ?? ensureDefaultSquad(md)
  const targetLayer = opts?.layerId ?? ensureDefaultLayer(md)
  const index = st.squadsById[targetSquad]?.slotIds.length ?? 0
  const id = newId()
  const z = terrainZ(position.x, position.y)
  md.wasm.add_slot(
    id,
    targetSquad,
    targetLayer,
    index,
    opts?.role ?? 'Rifleman',
    opts?.tag || undefined,
    opts?.assetId || undefined,
    position.x,
    position.y,
    z,
    0,
  )
  commit(md, 'local')
  return id
}

/** Distance (m) a paste is offset from its originals when the cursor is off-map (mirrors the Rust
 *  PASTE_NUDGE so the JS z-sampling lands at the same clamped position the Rust op computes). */
const PASTE_NUDGE = 20

/** Paste copied slots (Ctrl+V, T-056) in ONE Rust transaction. Positions translate so the clip's
 *  centroid lands at `anchorAt` (map cursor); off-map → +PASTE_NUDGE nudge. Each copy re-attaches to
 *  its source squad (or the default if it was deleted), files into `opts.layerId` (or the default),
 *  and x/y clamp to terrain bounds. JS mirrors that centroid/clamp math purely to sample terrain z at
 *  each final position; Rust re-derives the identical positions. Returns the new ids in clip order. */
export function pasteSlots(
  md: MissionDoc,
  clip: ClipboardSlot[],
  opts?: { anchorAt?: { x: number; y: number } | null; layerId?: ID },
): ID[] {
  if (!md.wasm || !clip.length) return []
  const st = useMapStore.getState()
  const terrain = getTerrain(st.meta?.terrain as TerrainId | undefined)
  const cx = clip.reduce((a, s) => a + s.position.x, 0) / clip.length
  const cy = clip.reduce((a, s) => a + s.position.y, 0) / clip.length
  const anchor = opts?.anchorAt
  const dx = anchor ? anchor.x - cx : PASTE_NUDGE
  const dy = anchor ? anchor.y - cy : PASTE_NUDGE

  // Resolve the default squad/layer AT MOST once per paste (the store isn't resynced until commit,
  // so re-reading it mid-loop would re-mint). Cache the minted default id.
  let defaultSquadId: ID | null = null
  const resolveSquad = (srcId: ID): ID => {
    if (st.squadsById[srcId]) return srcId
    if (!defaultSquadId) defaultSquadId = ensureDefaultSquad(md)
    return defaultSquadId
  }
  let defaultLayerId: ID | null = null
  const resolveLayer = (): ID => {
    if (opts?.layerId && st.editorLayersById[opts.layerId]) return opts.layerId
    if (!defaultLayerId) defaultLayerId = ensureDefaultLayer(md)
    return defaultLayerId
  }

  const ids: ID[] = []
  const squadIds: ID[] = []
  const layerIds: ID[] = []
  const srcX: number[] = []
  const srcY: number[] = []
  const srcRot: number[] = []
  const zs: number[] = []
  const roles: string[] = []
  const tags: string[] = []
  const assetIds: string[] = []
  const stances: string[] = []
  for (const c of clip) {
    ids.push(newId())
    squadIds.push(resolveSquad(c.squadId))
    layerIds.push(resolveLayer())
    srcX.push(c.position.x)
    srcY.push(c.position.y)
    srcRot.push(c.position.rotation)
    // Re-sample z at the clamped paste x/y (terrain-follow), not the clipboard z (T-091.2).
    const px = clamp(c.position.x + dx, 0, terrain.width)
    const py = clamp(c.position.y + dy, 0, terrain.height)
    zs.push(terrainZ(px, py))
    roles.push(c.role)
    tags.push(c.tag ?? '')
    assetIds.push(c.assetId ?? '')
    stances.push(c.stance)
  }
  md.wasm.paste_slots(
    ids,
    squadIds,
    layerIds,
    Float64Array.from(srcX),
    Float64Array.from(srcY),
    Float64Array.from(srcRot),
    Float64Array.from(zs),
    roles,
    tags,
    assetIds,
    stances,
    anchor ? anchor.x : undefined,
    anchor ? anchor.y : undefined,
    terrain.width,
    terrain.height,
  )
  commit(md, 'local')
  return ids
}

/** Move several positioned slots by a shared world delta in ONE transaction (Eden drag-to-move,
 *  Phase 7b). z is re-sampled JS-side at each moved x/y (drag preview stays xy-only) — T-091.2. */
export function moveEntities(md: MissionDoc, ids: ID[], delta: { x: number; y: number }): void {
  if (!md.wasm || !ids.length) return
  const st = useMapStore.getState()
  const zs = ids.map((id) => {
    const p = st.slotsById[id]?.position
    return terrainZ((p?.x ?? 0) + delta.x, (p?.y ?? 0) + delta.y)
  })
  md.wasm.move_entities(ids, delta.x, delta.y, Float64Array.from(zs))
  commit(md, 'local')
}

/** Remove an entity, cascading children + detaching refs. The app only ever removes slots; the
 *  Rust `remove_slots` owns the squad.slotIds + layer.entityIds detach. Non-slot maps are a dead
 *  path (no consumer) → no-op. */
export function removeEntity(md: MissionDoc, mapName: EntityMapName, id: ID): void {
  removeEntities(md, mapName, [id])
}

/** Remove several entities from one map in ONE transaction (Delete/Backspace, Phase 7b). */
export function removeEntities(md: MissionDoc, mapName: EntityMapName, ids: ID[]): void {
  if (!md.wasm || !ids.length) return
  if (mapName !== 'slots') return // dead path — the app only removes slots
  md.wasm.remove_slots(ids)
  commit(md, 'local')
}

// ── Meta + structural actions ────────────────────────────────────────────────

/** Apply mission row fields from GET /missions/:id (title hydrate, T-049). INIT origin — a load, not
 *  a user edit. Rust owns the empty-title / invalid-terrain skips + the env merge. */
export function applyMissionRowMeta(
  md: MissionDoc,
  row: { title: string; terrain: string; time_of_day?: string; weather?: string },
): void {
  if (!md.wasm) return
  md.setOriginInit(true)
  md.wasm.apply_row_meta(
    row.title,
    row.terrain,
    row.time_of_day ?? undefined,
    row.weather ?? undefined,
  )
  md.setOriginInit(false)
  commit(md, 'init')
}

/** Seed meta with defaults if empty. INIT origin → NOT an undo step. */
export function seedMeta(md: MissionDoc, opts: { id: ID; title: string }): void {
  if (!md.wasm || useMapStore.getState().meta) return
  md.setOriginInit(true)
  md.wasm.seed_meta(opts.id, opts.title)
  md.setOriginInit(false)
  commit(md, 'init')
}

/** Seed a default Outliner folder if none exist. INIT origin → NOT an undo step. */
export function seedDefaultLayer(md: MissionDoc): void {
  if (!md.wasm || Object.keys(useMapStore.getState().editorLayersById).length > 0) return
  md.setOriginInit(true)
  md.wasm.add_editor_layer(newId(), 'Default Layer', undefined)
  md.setOriginInit(false)
  commit(md, 'init')
}

/** Create a new (root or nested) Outliner folder; returns its id. */
export function addEditorLayer(md: MissionDoc, opts?: { name?: string; parentId?: ID | null }): ID {
  if (!md.wasm) return ''
  const n = Object.keys(useMapStore.getState().editorLayersById).length + 1
  const id = newId()
  md.wasm.add_editor_layer(id, opts?.name ?? `New Folder ${n}`, opts?.parentId ?? undefined)
  commit(md, 'local')
  return id
}

/** Rename an Outliner folder. */
export function renameEditorLayer(md: MissionDoc, id: ID, name: string): void {
  if (!md.wasm || !useMapStore.getState().editorLayersById[id]) return
  md.wasm.rename_editor_layer(id, name)
  commit(md, 'local')
}

/** Is `nodeId` inside `ancestorId`'s subtree (or equal to it)? Walks up via the store's parentId. */
function isLayerDescendant(ancestorId: ID, nodeId: ID): boolean {
  const layers = useMapStore.getState().editorLayersById
  let cur: ID | null = nodeId
  while (cur) {
    if (cur === ancestorId) return true
    cur = layers[cur]?.parentId ?? null
  }
  return false
}

/** Reparent an Outliner folder. Rejects cycles (dropping a folder into its own subtree) — the JS
 *  guard early-returns without notifying so a no-op cycle doesn't mark the doc dirty. */
export function reparentEditorLayer(md: MissionDoc, id: ID, newParentId: ID | null): void {
  if (!md.wasm || !useMapStore.getState().editorLayersById[id]) return
  if (newParentId === id) return
  if (newParentId && isLayerDescendant(id, newParentId)) return
  md.wasm.reparent_editor_layer(id, newParentId ?? undefined)
  commit(md, 'local')
}

/** Refile a slot into a different Outliner folder (workflow-only; squad unchanged). */
export function moveSlotToLayer(md: MissionDoc, slotId: ID, targetLayerId: ID): void {
  if (!md.wasm || !useMapStore.getState().editorLayersById[targetLayerId]) return
  md.wasm.move_slot_to_layer(slotId, targetLayerId)
  commit(md, 'local')
}

/** Delete an Outliner folder AND its whole subtree in ONE transaction. No-op if it is the only
 *  layer; if the subtree was every layer, Rust reseeds a fresh default with the JS-minted id. */
export function removeEditorLayer(md: MissionDoc, id: ID): void {
  const layers = useMapStore.getState().editorLayersById
  if (!md.wasm || !layers[id] || Object.keys(layers).length <= 1) return
  md.wasm.remove_editor_layer(id, newId())
  commit(md, 'local')
}

export function setTitle(md: MissionDoc, title: string): void {
  if (!md.wasm) return
  md.wasm.set_title(title)
  commit(md, 'local')
}

export function updateEnvironment(
  md: MissionDoc,
  patch: Partial<MissionMeta['environment']>,
): void {
  if (!md.wasm) return
  md.wasm.update_environment(JSON.stringify(patch))
  commit(md, 'local')
}

/** Edit a slot's transform numerically (Attributes Transform tab, T-049). x/y clamp to terrain
 *  bounds; rotation normalizes to [0,360); z policy (T-091.2): a manual z sticks, an x/y edit
 *  terrain-follows, a rotation-only edit leaves z. z is resolved JS-side and passed to Rust. */
export function updateSlotPosition(
  md: MissionDoc,
  id: ID,
  patch: Partial<{ x: number; y: number; z: number; rotation: number }>,
): void {
  if (!md.wasm) return
  const st = useMapStore.getState()
  const slot = st.slotsById[id]
  if (!slot) return
  const terrain = getTerrain(st.meta?.terrain as TerrainId | undefined)
  const prev = slot.position
  let z: number | undefined
  if (patch.z != null && Number.isFinite(patch.z)) {
    z = patch.z
  } else if (patch.x != null || patch.y != null) {
    const nx =
      patch.x != null && Number.isFinite(patch.x) ? clamp(patch.x, 0, terrain.width) : prev.x
    const ny =
      patch.y != null && Number.isFinite(patch.y) ? clamp(patch.y, 0, terrain.height) : prev.y
    z = terrainZ(nx, ny)
  }
  md.wasm.update_slot_position(
    id,
    patch.x,
    patch.y,
    z,
    patch.rotation,
    terrain.width,
    terrain.height,
  )
  commit(md, 'local')
}

/** Patch scalar slot fields (role / tag / stance). An undefined field is left unchanged. */
export function updateSlot(
  md: MissionDoc,
  id: ID,
  patch: Partial<{ role: string; tag: string; stance: string }>,
): void {
  if (!md.wasm) return
  md.wasm.update_slot(id, patch.role, patch.tag, patch.stance)
  commit(md, 'local')
}

/** Create a new faction; returns its id. */
export function addFaction(md: MissionDoc): ID {
  if (!md.wasm) return ''
  const n = Object.keys(useMapStore.getState().factionsById).length + 1
  const id = newId()
  md.wasm.add_faction(id, 'BLUFOR', `Faction ${n}`)
  commit(md, 'local')
  return id
}

/** Create a squad under a faction; returns its id (or '' if the faction is gone). */
export function addSquad(md: MissionDoc, factionId: ID): ID {
  if (!md.wasm) return ''
  const faction = useMapStore.getState().factionsById[factionId]
  if (!faction) return ''
  const n = faction.squadIds.length + 1
  const id = newId()
  md.wasm.add_squad(id, factionId, `Squad ${n}`, undefined)
  commit(md, 'local')
  return id
}

// ── Hydrate (Phase 9 load) ────────────────────────────────────────────────────

/** Rebuild the lossy backend `orbat[]` into an editor-shaped payload (JS mints the ids). The Rust
 *  hydrate is a verbatim loader; the lossy transform stays JS-side (batch 3d). Files every slot into
 *  a single Default Layer with default positions. */
function lossyOrbatToEditor(payload: Record<string, unknown>): Record<string, unknown> {
  const factions: Record<string, unknown>[] = []
  const squads: Record<string, unknown>[] = []
  const slots: Record<string, unknown>[] = []
  const filed: ID[] = []
  const byKey = new Map<string, ID>()
  const layerId = newId()
  const orbat = (payload.orbat as Record<string, unknown>[] | undefined) ?? []
  for (const sq of orbat) {
    const key = String(sq.faction ?? 'BLUFOR')
    let factionId = byKey.get(key)
    if (!factionId) {
      factionId = newId()
      byKey.set(key, factionId)
      factions.push({ id: factionId, key, name: key, squadIds: [] as ID[] })
    }
    const faction = factions.find((f) => f.id === factionId) as { squadIds: ID[] }
    const squadId = newId()
    const slotIds: ID[] = []
    const sqSlots = (sq.slots as Record<string, unknown>[] | undefined) ?? []
    sqSlots.forEach((sl, i) => {
      const slotId = newId()
      slots.push({
        id: slotId,
        squadId,
        index: i,
        role: String(sl.role ?? 'Rifleman'),
        ...(sl.tag ? { tag: String(sl.tag) } : {}),
        position: { x: 0, y: 0, z: 0, rotation: 0 },
        stance: 'stand',
        loadoutId: null,
      })
      slotIds.push(slotId)
      filed.push(slotId)
    })
    squads.push({
      id: squadId,
      factionId,
      callsign: String(sq.callsign ?? ''),
      name: String(sq.squad ?? 'Squad'),
      slotIds,
    })
    faction.squadIds.push(squadId)
  }
  const editorLayers = [{ id: layerId, name: 'Default Layer', parentId: null, entityIds: filed }]
  return { ...payload, editor: { factions, squads, slots, editorLayers } }
}

/** Repopulate the doc from a compiled json_payload (Phase 9 load/hydrate). INIT origin — loading a
 *  server version is not a user edit. Prefers the lossless `editor` block; falls back to the lossy
 *  `orbat[]` rebuild for missions authored elsewhere. The Rust loader clears + reloads verbatim. */
export function hydrateMissionDoc(md: MissionDoc, payload: Record<string, unknown>): void {
  if (!md.wasm) return
  const p = payload ?? {}
  const editorPayload = p.editor ? p : lossyOrbatToEditor(p)
  md.setOriginInit(true)
  md.wasm.hydrate(JSON.stringify(editorPayload), newId())
  md.setOriginInit(false)
  commit(md, 'init')
}

/** Async variant with progress (T-060.1). The Rust load is one fast call, so this is UI-only: it
 *  reports 0→total around the single hydrate. Signature kept for the load-overlay call sites. */
export async function hydrateMissionDocWithProgress(
  md: MissionDoc,
  payload: Record<string, unknown>,
  onProgress?: (done: number, total: number) => void,
): Promise<void> {
  const p = payload ?? {}
  const editor = p.editor as { slots?: unknown[] } | undefined
  const total = Array.isArray(editor?.slots)
    ? editor.slots.length
    : ((p.orbat as Record<string, unknown>[] | undefined) ?? []).reduce(
        (a, sq) => a + ((sq.slots as unknown[] | undefined)?.length ?? 0),
        0,
      )
  onProgress?.(0, total)
  hydrateMissionDoc(md, p)
  onProgress?.(total, total)
}
