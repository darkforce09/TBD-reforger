// T-090.5.5 — Main-thread tree/prop glyph store: viewport-driven streaming of individual
// tree/vegetation/prop instances through the world-objects worker's visibleInstances API (the
// budget-capped SoA the worker returns after self-hydrating the covering chunks). Sibling to
// forestMassStore/chunkStore, but REPLACE-not-accumulate: each viewport commit yields the exact
// visible set for the current zoom band, resolved once into two TreeGlyphSets (tree group + prop
// group) with per-instance glyph key / size / color / angle — nothing runs per frame (T-057),
// deduped on the covering chunk set + zoom band so pan within a set is a no-op.
//
// Below TREE_GLYPH_MIN_ZOOM (0) no tree class is visible → the store clears to empty and the
// forest-mass polygons carry readability. NO world supercluster (contract LOD5).
//
// Factory + module default instance (same test shape as forestMassStore): tests inject a fake
// client; the app uses the singleton wired to worldObjectsClient.

import {
  chunkIdsForRect,
  chunkRectForBbox,
  expandBbox,
  preloadMarginM,
  type Bbox,
} from './chunkMath'
import { classVisible } from './lodGates'
import {
  EMPTY_TREE_GLYPHS,
  deckAngleForRotationDeg,
  glyphSizeMeters,
  hexToRgba,
  type TreeGlyphInstance,
  type TreeGlyphSet,
} from './treePropLayer'
import { loadWorldManifest, worldVisibleInstances } from '../workers/worldObjectsClient'
import {
  RENDER_CLASS_CODES,
  renderClassForPrefab,
  type VisibleSet,
  type WorldManifestLite,
  type WorldPrefabRow,
} from '../workers/worldObjectsCore'
import type { TerrainDef } from '../coords/terrains'

/** Which toggles are on this commit (per-user `tbd-mc-world-layers` prefs). */
export interface TreeToggles {
  trees: boolean
  props: boolean
}

/** Per-prefab glyph render, resolved once from the manifest render block. */
interface GlyphInfo {
  iconKey: string
  /** Size in meters for the `sizeUnits:'meters'` layer (baseSizePx·mult / 2^REF_ZOOM). */
  sizeMeters: number
  colorRgba: [number, number, number, number]
}

/** Fallback glyph size (px @ REF_ZOOM) when a prefab omits render.baseSizePx. */
const DEFAULT_BASE_SIZE_PX = 16

/** RENDER_CLASS_CODES indices, resolved once (the worker ships classes as these codes). */
const TREE_CODE = RENDER_CLASS_CODES.indexOf('tree')
const VEGETATION_CODE = RENDER_CLASS_CODES.indexOf('vegetation')
const PROP_CODE = RENDER_CLASS_CODES.indexOf('prop')
const ROCK_LARGE_CODE = RENDER_CLASS_CODES.indexOf('rockLarge')

/** The worker-client surface the store consumes (injectable for tests). */
export interface TreeStreamClient {
  loadManifest(terrainId: string): Promise<WorldManifestLite | null>
  visibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet>
}

export interface TreeStore {
  ensureTreeStream(terrain: TerrainDef): void
  setTreeViewport(bbox: Bbox | null, deckZoom: number, toggles: TreeToggles): void
  getTreeGlyphs(): TreeGlyphSet
  getPropGlyphs(): TreeGlyphSet
  getTreeRevision(): number
  subscribeTreeStream(cb: () => void): () => void
  resetTreeStream(): void
}

/** prefabId → glyph render for the tree/prop kinds only (building/road/water excluded — those
 *  render as polygons/paths). A prefab without an iconKey can't draw a glyph → skipped. */
function buildGlyphLookup(rows: WorldPrefabRow[]): Map<number, GlyphInfo> {
  const lookup = new Map<number, GlyphInfo>()
  for (const row of rows) {
    const iconKey = row.render?.iconKey
    if (!iconKey) continue
    const rc = renderClassForPrefab(row.kind, row.class)
    if (rc !== 'tree' && rc !== 'vegetation' && rc !== 'prop' && rc !== 'rockLarge') continue
    lookup.set(row.prefabId, {
      iconKey,
      sizeMeters: glyphSizeMeters(row.render?.baseSizePx ?? DEFAULT_BASE_SIZE_PX, row.spatial?.heightM),
      colorRgba: hexToRgba(row.render?.defaultColor),
    })
  }
  return lookup
}

/** Which glyph layer an instance belongs to — from its class code, only if a glyph is known.
 *  Building/unclassified codes (visibleInstances also returns buildings at the tree band) and
 *  glyph-less prefabs → null (dropped; buildings draw via chunkStore/buildingLayer). */
function groupForInstance(
  classCode: number,
  prefabId: number,
  glyphs: Map<number, GlyphInfo>,
): 'tree' | 'prop' | null {
  if (!glyphs.has(prefabId)) return null
  if (classCode === TREE_CODE || classCode === VEGETATION_CODE) return 'tree'
  if (classCode === PROP_CODE || classCode === ROCK_LARGE_CODE) return 'prop'
  return null
}

/** Partition a VisibleSet into tree + prop glyph object arrays (the IconLayer accessor form —
 *  matches layers/useIconLayer.ts). colorRgba is shared per prefab (read-only in getColor);
 *  position is fresh per instance. */
function partition(
  set: VisibleSet,
  glyphs: Map<number, GlyphInfo>,
): { tree: TreeGlyphSet; prop: TreeGlyphSet } {
  const tree: TreeGlyphSet = []
  const prop: TreeGlyphSet = []
  for (let i = 0; i < set.count; i++) {
    const prefabId = set.prefabIdx[i]
    const g = groupForInstance(set.classes[i], prefabId, glyphs)
    if (!g) continue
    const info = glyphs.get(prefabId) as GlyphInfo
    const glyph: TreeGlyphInstance = {
      position: [set.positions[2 * i], set.positions[2 * i + 1]],
      angle: deckAngleForRotationDeg(set.rotations[i]),
      size: info.sizeMeters,
      color: info.colorRgba,
      iconKey: info.iconKey,
    }
    ;(g === 'tree' ? tree : prop).push(glyph)
  }
  return { tree, prop }
}

/** Zoom-band signature: which tree/prop classes are visible (the composite content changes at
 *  each of these gates, so the dedupe key must include them alongside the chunk set). */
function bandKey(deckZoom: number): string {
  return (
    (classVisible('tree', deckZoom) ? 'T' : '-') +
    (classVisible('vegetation', deckZoom) ? 'V' : '-') +
    (classVisible('prop', deckZoom) ? 'P' : '-') +
    (classVisible('rockLarge', deckZoom) ? 'R' : '-')
  )
}

export function createTreeStore(deps: { client: TreeStreamClient }): TreeStore {
  const { client } = deps

  let terrain: TerrainDef | null = null
  let manifest: WorldManifestLite | null = null
  let started = false
  let glyphs = new Map<number, GlyphInfo>()

  let treeSet: TreeGlyphSet = EMPTY_TREE_GLYPHS
  let propSet: TreeGlyphSet = EMPTY_TREE_GLYPHS
  let lastKey = ''
  /** Supersede token: a stale in-flight visibleInstances reply (older viewport) is discarded. */
  let requestSeq = 0
  let lastViewport: { bbox: Bbox; deckZoom: number; toggles: TreeToggles } | null = null

  let revision = 0
  const listeners = new Set<() => void>()

  const notify = (): void => {
    revision++
    listeners.forEach((l) => l())
  }

  /** Drop to empty (below band / toggles off / terrain switch); notify only on a real change. */
  function clearGlyphs(): void {
    if (treeSet.length === 0 && propSet.length === 0) return
    treeSet = EMPTY_TREE_GLYPHS
    propSet = EMPTY_TREE_GLYPHS
    notify()
  }

  function runViewport(bbox: Bbox, deckZoom: number, toggles: TreeToggles): void {
    if (!terrain || !manifest) return
    const treeWanted = toggles.trees && (classVisible('tree', deckZoom) || classVisible('vegetation', deckZoom))
    const propWanted = toggles.props && (classVisible('prop', deckZoom) || classVisible('rockLarge', deckZoom))
    if (!treeWanted && !propWanted) {
      // Below the glyph band or both toggles off → nothing streams (forest mass carries it).
      lastKey = ''
      requestSeq++ // cancel any in-flight reply
      clearGlyphs()
      return
    }
    const chunkSizeM = manifest.chunkSizeM
    const rect = chunkRectForBbox(expandBbox(bbox, preloadMarginM(bbox, chunkSizeM)), terrain, chunkSizeM)
    const ids = chunkIdsForRect(rect)
    const key = ids.join(',') + '|' + bandKey(deckZoom)
    if (key === lastKey) return // pan within the same chunk set + band — no refetch (T-057)
    lastKey = key
    // Query over a chunk-aligned bbox so sub-chunk pans return a stable set (Deck clips the
    // offscreen remainder). The worker self-hydrates + budget-caps (INSTANCE_BUDGET).
    const alignedBbox: Bbox = [
      rect.cx0 * chunkSizeM,
      rect.cy0 * chunkSizeM,
      (rect.cx1 + 1) * chunkSizeM,
      (rect.cy1 + 1) * chunkSizeM,
    ]
    const seq = ++requestSeq
    client
      .visibleInstances(alignedBbox, deckZoom)
      .then((set) => {
        if (seq !== requestSeq || terrain?.id == null) return // superseded by a newer viewport
        const { tree, prop } = partition(set, glyphs)
        treeSet = tree
        propSet = prop
        notify()
      })
      .catch((e: unknown) => {
        if (seq !== requestSeq) return
        lastKey = '' // allow a retry on the next viewport change
        console.warn('[worldmap] tree glyph stream failed — will retry on next viewport change', e)
      })
  }

  return {
    ensureTreeStream(t: TerrainDef): void {
      if (terrain?.id === t.id && started) return
      if (terrain && terrain.id !== t.id) {
        // Terrain switch: drop local state only — the shared worker core is unloaded by
        // chunkStore's switch path (all three stores talk to the same worker session).
        glyphs = new Map()
        treeSet = EMPTY_TREE_GLYPHS
        propSet = EMPTY_TREE_GLYPHS
        lastKey = ''
        requestSeq++
        lastViewport = null
        manifest = null
      }
      terrain = t
      started = true
      client
        .loadManifest(t.id)
        .then((m) => {
          if (terrain?.id !== t.id) return // switched away while loading
          manifest = m
          glyphs = m ? buildGlyphLookup(m.prefabRows) : new Map()
          if (manifest && lastViewport) {
            runViewport(lastViewport.bbox, lastViewport.deckZoom, lastViewport.toggles)
          }
        })
        .catch((e: unknown) => {
          if (terrain?.id !== t.id) return
          console.warn(`[worldmap] tree glyph manifest load failed for ${t.id} — glyphs off`, e)
        })
    },

    setTreeViewport(bbox: Bbox | null, deckZoom: number, toggles: TreeToggles): void {
      if (!bbox) return
      lastViewport = { bbox, deckZoom, toggles }
      runViewport(bbox, deckZoom, toggles)
    },

    getTreeGlyphs(): TreeGlyphSet {
      return treeSet
    },

    getPropGlyphs(): TreeGlyphSet {
      return propSet
    },

    getTreeRevision(): number {
      return revision
    },

    subscribeTreeStream(cb: () => void): () => void {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },

    resetTreeStream(): void {
      terrain = null
      manifest = null
      started = false
      glyphs = new Map()
      treeSet = EMPTY_TREE_GLYPHS
      propSet = EMPTY_TREE_GLYPHS
      lastKey = ''
      requestSeq++
      lastViewport = null
      notify()
    },
  }
}

const defaultStore = createTreeStore({
  client: {
    loadManifest: loadWorldManifest,
    visibleInstances: worldVisibleInstances,
  },
})

export const {
  ensureTreeStream,
  setTreeViewport,
  getTreeGlyphs,
  getPropGlyphs,
  getTreeRevision,
  subscribeTreeStream,
  resetTreeStream,
} = defaultStore
