// Dev-only shadow-doc parity (T-145 Phase 3.2 Stage 1) — a passive yrs `MissionDoc` kept in sync from
// the authoritative Y.Doc's update stream, so we can prove LIVE (across every real editor mutator)
// that the yrs SoA materializes identically to the Y.Doc before the cutover routes readers onto it.
// `docCore.parity.test.ts` proves this over synthetic ops; this proves it over real editing.
//
// Stage 1 gates the whole shadow behind `import.meta.env.DEV` (zero prod cost — nothing reads it yet);
// Stage 2 un-gates it and feeds the render + indices off its SoA.

import * as Y from 'yjs'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { docToSnapshot } from './bindings'
import type { MissionDoc } from './ydoc'
import type { EditorLayer, ID } from './schema'

const NONE = 0xffffffff // SlotSoa NONE_IDX (u32::MAX): no tag / unfiled layer.
const STANCE = ['stand', 'crouch', 'prone'] as const
// Above this the live gate checks slot COUNT only: a full `docToSnapshot` toJSON compare gets
// expensive, and the headless test already covers large-scale materialize. Mutator-shape bugs surface
// at any scale, so small missions during normal editing exercise every path.
const DEEP_CAP = 20_000

/** A fresh, empty shadow yrs doc. Free it (`.free()`) on teardown. */
export function createDocShadow(): wasm.MissionDoc {
  return new wasm.MissionDoc()
}

/** Seed the shadow with the Y.Doc's current state, so a shadow created mid-lifecycle (e.g. the
 *  StrictMode setup→cleanup→setup double, which does not re-run the doc `useMemo`) is instantly in
 *  sync; thereafter `md.doc.on('update')` keeps it live. */
export function seedDocShadow(md: MissionDoc, shadow: wasm.MissionDoc): void {
  shadow.apply_update(Y.encodeStateAsUpdate(md.doc))
}

/** Structural deep-equal: objects order-insensitive, arrays order-SENSITIVE (slotIds/entityIds are
 *  ordered lists). Numbers via `===` — the small maps store f64 (no SoA f32 truncation). */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false
    return a.every((v, i) => deepEqual(v, b[i]))
  }
  const oa = a as Record<string, unknown>
  const ob = b as Record<string, unknown>
  const ka = Object.keys(oa)
  if (ka.length !== Object.keys(ob).length) return false
  return ka.every((k) => k in ob && deepEqual(oa[k], ob[k]))
}

const SMALL_DICTS = [
  'factionsById',
  'squadsById',
  'loadoutsById',
  'itemsById',
  'objectivesById',
  'vehiclesById',
  'markersById',
  'editorLayersById',
] as const

/** Compare the shadow yrs SoA + small maps to the authoritative Y.Doc. Returns a mismatch
 *  description, else null. DEV diagnostic only — mirrors the `docCore.parity.test.ts` comparison,
 *  run live over the WHOLE model (Phase 3.2.2). */
// eslint-disable-next-line complexity -- DEV parity: one guarded compare per SoA column; flat by design
export function checkDocShadowParity(md: MissionDoc, shadow: wasm.MissionDoc): string | null {
  shadow.refresh()
  const yjsCount = md.entities.slots.size
  if (shadow.slot_len !== yjsCount) return `slot count yjs=${yjsCount} yrs=${shadow.slot_len}`
  if (yjsCount > DEEP_CAP) return null

  const snap = docToSnapshot(md)
  const slots = snap.slotsById
  const layerOf = new Map<ID, ID>()
  for (const [lid, layer] of Object.entries(snap.editorLayersById) as [ID, EditorLayer][]) {
    for (const sid of layer.entityIds ?? []) if (!layerOf.has(sid)) layerOf.set(sid, lid)
  }

  const ids = shadow.slot_ids()
  const xs = shadow.slot_xs()
  const ys = shadow.slot_ys()
  const zs = shadow.slot_zs()
  const rot = shadow.slot_rotations()
  const stance = shadow.slot_stance()
  const roleIdx = shadow.slot_role_idx()
  const tagIdx = shadow.slot_tag_idx()
  const squadIdx = shadow.slot_squad_idx()
  const layerIdx = shadow.slot_layer_idx()
  const roles = shadow.roles()
  const tags = shadow.tags()
  const squads = shadow.squads()
  const layers = shadow.layers()

  for (let i = 0; i < ids.length; i++) {
    const id = ids[i]
    const s = slots[id]
    if (!s) return `yrs has slot ${id} absent from Y.Doc`
    if (Math.fround(s.position.x) !== xs[i]) return `x ${id}`
    if (Math.fround(s.position.y) !== ys[i]) return `y ${id}`
    if (Math.fround(s.position.z) !== zs[i]) return `z ${id}`
    if (Math.fround(s.position.rotation) !== rot[i]) return `rotation ${id}`
    if (STANCE[stance[i]] !== s.stance) return `stance ${id}`
    if (roles[roleIdx[i]] !== s.role) return `role ${id}`
    const tag = tagIdx[i] === NONE ? undefined : tags[tagIdx[i]]
    if ((tag ?? undefined) !== (s.tag ?? undefined)) return `tag ${id}`
    if (squads[squadIdx[i]] !== s.squadId) return `squad ${id}`
    const layer = layerIdx[i] === NONE ? undefined : layers[layerIdx[i]]
    if ((layer ?? undefined) !== (layerOf.get(id) ?? undefined)) return `layer ${id}`
  }

  // Small maps (factions/squads/loadouts/items/objectives/vehicles/markers/editorLayers + meta):
  // the shadow's JSON must deep-equal docToSnapshot's dicts. These maps are small, so this stays cheap
  // regardless of slot count.
  const small = JSON.parse(shadow.small_maps_json()) as Record<string, unknown>
  if (!deepEqual(small.meta, snap.meta)) return 'meta'
  for (const key of SMALL_DICTS) {
    if (!deepEqual(small[key], snap[key])) return key
  }
  return null
}
