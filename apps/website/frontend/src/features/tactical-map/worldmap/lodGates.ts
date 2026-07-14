// ORACLE-ONLY (T-151.11.3, audit B-01): the live path calls the wasm exports
// (`class_visible`, `contour_interval_for_zoom`); this file feeds parity/unit tests only.
// T-090.5.1 — World-object LOD gates. Data-form of the canonical N2/N3 tables in
// docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md (v2) — that doc is the
// authority; any threshold change lands there first. Pure decision module: no React/Deck,
// node-testable. A3 density-gate model: zoom is continuous, feature CLASSES step in and out
// at fixed deckZoom gates. NO world clustering exists in v2 (contract LOD5) — forest mass
// polygons carry readability below the tree-glyph band.
//
// Consumers: layer builders + worker pick (T-090.5.2+); pick gates MUST use the same
// classVisible table so picking never hits an invisible class (contract N4).

/** Glyph size anchor: displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM). */
export const REF_ZOOM = 3
/** deckZoom ≥ 0 → individual tree glyphs (below: hidden; forest mass only). */
export const TREE_GLYPH_MIN_ZOOM = 0
/** Historical N3 max fill zoom (+1). T-151.5.1: fill hides for zoom ≥ TREE_GLYPH_MIN_ZOOM;
 *  this constant is no longer used by `classVisible` (kept for contract grep). */
export const FOREST_FILL_MAX_ZOOM = 1
/** deckZoom ≥ −1.5 → forest outline, and only while below tree glyph band (T-151.5.1). */
export const FOREST_OUTLINE_MIN_ZOOM = -1.5
/** deckZoom ≥ −2.5 → building OBB rects (A3 ptsPerSquareObj≈9; visible at default −2). */
export const BUILDING_FOOTPRINT_MIN_ZOOM = -2.5
/** deckZoom ≥ +1 → military/tower/bunker badge. */
export const BUILDING_BADGE_MIN_ZOOM = 1
/** deckZoom ≥ +1.5 → vegetation glyphs. */
export const VEGETATION_MIN_ZOOM = 1.5
/** deckZoom ≥ +3 → prop/small-rock glyphs. */
export const PROP_MIN_ZOOM = 3
/** deckZoom ≥ +1 → large rock landmark glyphs. */
export const ROCK_LARGE_MIN_ZOOM = 1
/** deckZoom ≤ +3 → sea band fill visible (A3 DrawSea; fades out as detail takes over — the
 *  discrete α ladder is T-090.5.4 styling, seaBand.seaFillAlpha). */
export const SEA_FILL_MAX_ZOOM = 3
/** Screen pick radius in px (A3 2%-viewport analogue); world radius = PICK_RADIUS_PX · 2^-zoom. */
export const PICK_RADIUS_PX = 12
/** Max drawn world instances at any zoom (vitest vs census once streaming lands, T-090.5.3). */
export const INSTANCE_BUDGET = 150_000

/** Road classes (map-object-roads schema) — gates per the contract's road class table. */
export type RoadClass = 'highway_paved' | 'road_paved' | 'road_dirt' | 'track' | 'path' | 'runway'

/** Every world render class the gate table covers (contract N2/N3 rows). `sea` is a MAX gate
 *  (like forestFill); `contour` draws across the whole band (min −6). */
export type WorldRenderClass =
  | 'tree'
  | 'vegetation'
  | 'prop'
  | 'rockLarge'
  | 'building'
  | 'buildingBadge'
  | 'forestFill'
  | 'forestOutline'
  | 'sea'
  | 'contour'
  | RoadClass

/** Min-deckZoom gate per class. `forestFill`/`sea`/`forestOutline` have custom bands. */
const MIN_ZOOM_GATES: Record<
  Exclude<WorldRenderClass, 'forestFill' | 'sea' | 'forestOutline'>,
  number
> = {
  tree: TREE_GLYPH_MIN_ZOOM,
  vegetation: VEGETATION_MIN_ZOOM,
  prop: PROP_MIN_ZOOM,
  rockLarge: ROCK_LARGE_MIN_ZOOM,
  building: BUILDING_FOOTPRINT_MIN_ZOOM,
  buildingBadge: BUILDING_BADGE_MIN_ZOOM,
  contour: -6,
  highway_paved: -6,
  road_paved: -6,
  road_dirt: -2,
  track: -2,
  path: 4,
  runway: -6,
}

/** Is a class drawn (and pickable — N4) at this deckZoom? Class gates only; the per-prefab
 *  `render.importanceZoom` override (contract N2) lives in the Rust engine (`world/residency.rs`,
 *  T-152.21) — TypeScript is not on the world render path.
 *  T-151.5.1: forest fill + outline hide when tree glyphs are on (zoom ≥ TREE_GLYPH_MIN_ZOOM). */
export function classVisible(cls: WorldRenderClass, deckZoom: number): boolean {
  if (cls === 'forestFill') return deckZoom < TREE_GLYPH_MIN_ZOOM
  if (cls === 'forestOutline') {
    return deckZoom >= FOREST_OUTLINE_MIN_ZOOM && deckZoom < TREE_GLYPH_MIN_ZOOM
  }
  if (cls === 'sea') return deckZoom <= SEA_FILL_MAX_ZOOM
  return deckZoom >= MIN_ZOOM_GATES[cls]
}

/**
 * Contour interval (m) for a deckZoom, per the render contract §N3 ladder. Bands step to the
 * FINER interval at each edge (edge belongs to the finer band).
 *
 * NOTE — divergence: the T-090.5.4 ticket text reads `20 m @ 0…+3`, but §N3 (and the plan §5
 * master band table) read `20 m @ 0…+1, 10 m @ +1…+3`. §N3 is the cited authority and wins;
 * this implements N3. The ticket's vitest pin (−2 → 20 m) holds either way. Flagged for Cursor
 * doc sync to reconcile the ticket prose.
 */
export function contourIntervalForZoom(deckZoom: number): number {
  if (deckZoom < -4) return 100
  if (deckZoom < -2.5) return 50
  if (deckZoom < 1) return 20
  return 10
}

/** Budget gate: total would-be-drawn instances across visible classes fits INSTANCE_BUDGET.
 *  T-090.5.3 wires this to type-inventory census integers per band boundary. */
export function instanceBudgetCheck(visibleCounts: number[]): boolean {
  let sum = 0
  for (const n of visibleCounts) sum += n
  return sum <= INSTANCE_BUDGET
}
