// T-090.5.5 — Tree / vegetation / prop glyph layers (Map Engine v2 slots 9–10, ids
// `world-trees` + `world-props`, drawn after forest mass per the t090_10 layer stack). The
// last render lane: individual IconLayer glyphs from the world-glyph atlas, keyed per instance
// by prefab `render.iconKey`, rotated by export yaw, sized `baseSizePx·2^(zoom−REF_ZOOM)` with
// an optional heightM cap. Data is the budget-capped SoA the worker's visibleInstances returns,
// pre-resolved into a TreeGlyphComposite by treeStore (glyph key/size/color/angle computed once
// per stream commit — nothing per frame, T-057 rule). Below TREE_GLYPH_MIN_ZOOM (0) trees are
// hidden and the forest-mass polygons carry readability — NO world supercluster (contract LOD5).
//
// Pure decision exports (deckAngleForRotationDeg, treeSizeMultiplier, hexToRgba) are
// node-testable; the two builders stay thin (spine rule). Visibility booleans come from
// lodGates.classVisible — sole authority (LOD5). Mass layers — never pickable (N4: pick is
// worker-owned, T-090.9).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { IconLayer } from '@deck.gl/layers'
import { REF_ZOOM } from './lodGates'
import type { WorldGlyphAtlas } from '../layers/worldGlyphAtlas'

/** Per-instance glyph data for one class group (tree or prop), SoA + parallel string keys.
 *  `sizes` is METERS (baseSizePx·mult / 2^REF_ZOOM) so the IconLayer's `sizeUnits:'meters'`
 *  scales it with zoom for free — exactly `baseSizePx·2^(zoom−REF_ZOOM)` on screen, no
 *  per-frame updateTriggers (same trick as the T-090.5.2 building badge layer). */
export interface TreeGlyphComposite {
  count: number
  /** [x0,y0,x1,y1,…] world meters. */
  positions: Float32Array
  /** Deck `getAngle` degrees (handedness already converted from export yaw). */
  anglesDeg: Float32Array
  /** Per-instance glyph size in meters. */
  sizes: Float32Array
  /** rgba × count, 0–255 (Deck normalizes the uint8 color attribute). */
  colors: Uint8Array
  /** Atlas key per instance (getIcon accessor — the one non-binary attribute). */
  iconKeys: string[]
}

export const EMPTY_TREE_GLYPHS: TreeGlyphComposite = {
  count: 0,
  positions: new Float32Array(0),
  anglesDeg: new Float32Array(0),
  sizes: new Float32Array(0),
  colors: new Uint8Array(0),
  iconKeys: [],
}

/** Readability floor: never shrink a glyph below this on screen (plan §4.4 min-px clamps). */
export const GLYPH_SIZE_MIN_PX = 4

/** Reference tree height (m) at which the size multiplier is 1.0. Taller trees scale up to the
 *  1.5× cap (glyphs spec: "tall trees slightly larger icon, cap 1.5×"); shorter trees clamp to
 *  1.0 so undergrowth never renders smaller than a normal glyph. */
export const REF_TREE_HEIGHT_M = 10

/** Fallback glyph tint when a prefab has no (valid) render.defaultColor — a neutral forest
 *  green, never black (Deck's default getColor is black, which would blank tintable glyphs). */
export const DEFAULT_GLYPH_RGBA: [number, number, number, number] = [74, 122, 50, 255]

/** Export yaw (L2: 0° = map north +y, clockwise-positive) → Deck IconLayer `getAngle` (degrees,
 *  counter-clockwise-positive). Handedness flip = negate. On the north-up OrthographicView
 *  screen +y = map north, so this renders the glyph at the instance's true map yaw. */
export function deckAngleForRotationDeg(rotationDeg: number): number {
  const deg = Number.isFinite(rotationDeg) ? rotationDeg : 0
  return deg === 0 ? 0 : -deg // avoid -0 (Object.is(-0,0) is false — test-friendly)
}

/** Glyph size multiplier from tree height (glyphs spec 1.5× cap). Clamped to [1.0, 1.5];
 *  undefined/non-finite height → 1.0. */
export function treeSizeMultiplier(heightM: number | undefined): number {
  if (heightM === undefined || !Number.isFinite(heightM) || heightM <= 0) return 1
  const mult = heightM / REF_TREE_HEIGHT_M
  if (mult < 1) return 1
  if (mult > 1.5) return 1.5
  return mult
}

/** Glyph size in meters for the `sizeUnits:'meters'` layer: baseSizePx·mult / 2^REF_ZOOM
 *  (→ displayPx = baseSizePx·mult·2^(zoom−REF_ZOOM)). */
export function glyphSizeMeters(baseSizePx: number, heightM: number | undefined): number {
  return (baseSizePx * treeSizeMultiplier(heightM)) / 2 ** REF_ZOOM
}

/** `#rgb` / `#rrggbb` (with or without `#`) → [r,g,b,255]; invalid → DEFAULT_GLYPH_RGBA. */
export function hexToRgba(hex: string | undefined): [number, number, number, number] {
  if (!hex) return [...DEFAULT_GLYPH_RGBA]
  const h = hex.trim().replace(/^#/, '')
  const expand = h.length === 3 ? h.replace(/(.)/g, '$1$1') : h
  if (expand.length !== 6 || !/^[0-9a-fA-F]{6}$/.test(expand)) return [...DEFAULT_GLYPH_RGBA]
  return [
    parseInt(expand.slice(0, 2), 16),
    parseInt(expand.slice(2, 4), 16),
    parseInt(expand.slice(4, 6), 16),
    255,
  ]
}

/** Shared builder for the two glyph IconLayers (world-trees / world-props). Returns null when
 *  the atlas isn't loaded (per-layer degrade, plan risk R5 — never a crash) or the composite is
 *  empty. `visible` gates via Deck so buffers stay on the GPU across band crossings. Binary
 *  attributes for position/angle/size/color; getIcon is the one per-row accessor (string key). */
function buildGlyphLayer(
  id: 'world-trees' | 'world-props',
  opts: { composite: TreeGlyphComposite; atlas: WorldGlyphAtlas | null; visible: boolean },
): IconLayer | null {
  const { composite, atlas } = opts
  if (!atlas || composite.count === 0) return null
  const { iconKeys } = composite
  return new IconLayer({
    id,
    data: {
      length: composite.count,
      attributes: {
        getPosition: { value: composite.positions, size: 2 },
        getAngle: { value: composite.anglesDeg, size: 1 },
        getSize: { value: composite.sizes, size: 1 },
        getColor: { value: composite.colors, size: 4 },
      },
    },
    visible: opts.visible,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    iconAtlas: atlas.atlasUrl,
    iconMapping: atlas.iconMapping,
    // Binary data → object is undefined; resolve the atlas key by row index.
    getIcon: (_: unknown, info: { index: number }) => iconKeys[info.index],
    sizeUnits: 'meters',
    sizeMinPixels: GLYPH_SIZE_MIN_PX,
    pickable: false,
  })
}

/** Build the slot-9 `world-trees` IconLayer (tree + vegetation glyph group). */
export function buildTreeGlyphLayer(opts: {
  composite: TreeGlyphComposite
  atlas: WorldGlyphAtlas | null
  visible: boolean
}): IconLayer | null {
  return buildGlyphLayer('world-trees', opts)
}

/** Build the slot-10 `world-props` IconLayer (prop + large-rock glyph group). */
export function buildPropGlyphLayer(opts: {
  composite: TreeGlyphComposite
  atlas: WorldGlyphAtlas | null
  visible: boolean
}): IconLayer | null {
  return buildGlyphLayer('world-props', opts)
}
