// T-145 Phase 3.2 flip F2 — the post-flip document handle. A stable shell that owns a wasm
// `MissionDoc` (the yrs doc-core). The wasm handle is attached/detached by the lifecycle effect
// (created on setup, freed on cleanup) so React 19 StrictMode's setup→cleanup→setup double-invoke
// never shares — and then double-frees — one handle: wasm-bindgen `.free()` is NOT idempotent, unlike
// `Y.Doc.destroy()` (see [[wasm-react-lifecycle]]). Mutators no-op while the shell is detached.
//
// Isolated in F2: nothing imports this yet. F3 swaps `MissionDoc` = this shell + rewrites the
// `ydoc.ts` mutators to call `md.wasm.<op>` and `md.notifyChange('local')`.

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { snapshotFromShadow } from './docShadow'
import type { MapSnapshot } from './useMapStore'

/** `'local'` = an undoable user gesture (drives dirty + persistence); `'init'` = a load / seed
 *  (boot, hydrate, conflict-adopt) — NOT dirty, NOT persisted by the autosave gate. Mirrors
 *  `ydoc.ts`'s `LOCAL_ORIGIN` / `INIT_ORIGIN` and the Rust doc's undo scoping. */
export type ChangeOrigin = 'local' | 'init'
export type ChangeListener = (origin: ChangeOrigin) => void

const EMPTY_SNAPSHOT: MapSnapshot = {
  meta: null,
  factionsById: {},
  squadsById: {},
  slotsById: {},
  loadoutsById: {},
  itemsById: {},
  objectivesById: {},
  vehiclesById: {},
  markersById: {},
  editorLayersById: {},
}

/** Owns a wasm `MissionDoc` behind a stable identity. The `ydoc.ts` mutators read `.wasm` to write;
 *  reads go through `snapshot()`; undo/persistence/dirty listen on `subscribe` + `changeVersion`. */
export class WasmMissionDoc {
  private handle: wasm.MissionDoc | null = null
  private listeners = new Set<ChangeListener>()
  // Monotonic; bumped on every mutation + undo/redo. The undo buttons + the persistence poll read it
  // to know "something changed" without threading the specific change through.
  private version = 0

  /** True while a wasm handle is attached (between `attach` and `detach`). */
  get alive(): boolean {
    return this.handle !== null
  }

  /** The raw wasm handle for the `ydoc.ts` mutators; `null` while detached (mutators no-op). */
  get wasm(): wasm.MissionDoc | null {
    return this.handle
  }

  /** Monotonic change counter — the undo-button re-render signal. */
  get changeVersion(): number {
    return this.version
  }

  /** Attach a fresh wasm handle (lifecycle effect setup). Replaces any prior handle (frees it first
   *  — a well-behaved caller detaches before re-attaching, but guard anyway). */
  attach(handle: wasm.MissionDoc): void {
    if (this.handle) this.handle.free()
    this.handle = handle
  }

  /** Free + drop the wasm handle (lifecycle effect cleanup). Shell-level idempotent (guards a
   *  double-detach) even though wasm `.free()` is not. */
  detach(): void {
    if (this.handle) {
      this.handle.free()
      this.handle = null
    }
  }

  // ── change signal ──────────────────────────────────────────────────────────
  /** Fire after a mutation (the `ydoc.ts` wrappers call this). `origin` gates dirty + persistence. */
  notifyChange(origin: ChangeOrigin): void {
    this.version++
    for (const cb of this.listeners) cb(origin)
  }

  /** Subscribe to change notifications; returns an unsubscribe fn. */
  subscribe(cb: ChangeListener): () => void {
    this.listeners.add(cb)
    return () => {
      this.listeners.delete(cb)
    }
  }

  // ── reads ──────────────────────────────────────────────────────────────────
  /** The whole `MapSnapshot` (exact-f64 via JSON) — boot / hydrate / undo-resync. Empty if detached. */
  snapshot(): MapSnapshot {
    return this.handle ? snapshotFromShadow(this.handle) : { ...EMPTY_SNAPSHOT }
  }

  /** The yrs update-stream persistence blob. Empty if detached. */
  encodeState(): Uint8Array {
    return this.handle ? this.handle.encode_state() : new Uint8Array()
  }

  /** True if the doc holds authored content (any faction/slot/objective/vehicle/marker). */
  hasContent(): boolean {
    return this.handle ? this.handle.has_content() : false
  }

  // ── boot / load ──────────────────────────────────────────────────────────────
  /** Apply a persistence / peer update blob (always INIT / untracked in the Rust doc). */
  applyUpdate(bytes: Uint8Array): void {
    this.handle?.apply_update(bytes)
  }

  /** Bracket boot / hydrate / default-seeding so those mutations are INIT (untracked). */
  setOriginInit(on: boolean): void {
    this.handle?.set_origin_init(on)
  }

  // ── undo ──────────────────────────────────────────────────────────────────────
  /** Undo the last local gesture; `true` if anything was undone. Notifies `'local'` so the store
   *  resyncs + dirty flips (F3's `undo.ts` resyncs via `snapshot()`). */
  undo(): boolean {
    const did = this.handle?.undo() ?? false
    if (did) this.notifyChange('local')
    return did
  }

  redo(): boolean {
    const did = this.handle?.redo() ?? false
    if (did) this.notifyChange('local')
    return did
  }

  canUndo(): boolean {
    return this.handle?.can_undo() ?? false
  }

  canRedo(): boolean {
    return this.handle?.can_redo() ?? false
  }
}

/** A fresh, detached shell (stable identity for `useMemo`; the effect attaches a wasm handle). */
export function createWasmMissionDoc(): WasmMissionDoc {
  return new WasmMissionDoc()
}
