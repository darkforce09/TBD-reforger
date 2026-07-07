// snapshotFromShadow (T-145 Phase 3.2) — reconstruct the whole MapSnapshot from a wasm MissionDoc:
// the small maps + exact-f64 slots, both via JSON (not the f32 SoA, which is render-only/lossy). Proven
// byte-equal to the pre-flip docToSnapshot (Phase 3.2.3). Post-flip this is the only whole-doc read:
// the WasmMissionDoc shell's snapshot() and the boot / hydrate / undo-resync paths all go through it.
// O(n) — a one-shot, never the render hot path.

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import type { ID, Slot } from './schema'
import type { MapSnapshot } from './useMapStore'

/** Reconstruct the entire `MapSnapshot` from the wasm doc (small maps + exact-f64 slots via JSON). */
export function snapshotFromShadow(shadow: wasm.MissionDoc): MapSnapshot {
  const small = JSON.parse(shadow.small_maps_json()) as Omit<MapSnapshot, 'slotsById'>
  const slotsById = JSON.parse(shadow.slots_json()) as Record<ID, Slot>
  return { ...small, slotsById }
}
