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
/** deckZoom ≤ +1 → forest polygon fill visible (α ladders are T-090.8.1 styling). */
export const FOREST_FILL_MAX_ZOOM = 1
/** deckZoom ≥ −1.5 → forest outline (A3 ptsPerSquareForEdge≈15). */
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
/** Screen pick radius in px (A3 2%-viewport analogue); world radius = PICK_RADIUS_PX · 2^-zoom. */
export const PICK_RADIUS_PX = 12
/** Max drawn world instances at any zoom (vitest vs census once streaming lands, T-090.5.3). */
export const INSTANCE_BUDGET = 150_000

/** Road classes (map-object-roads schema) — gates per the contract's road class table. */
export type RoadClass = 'highway_paved' | 'road_paved' | 'road_dirt' | 'track' | 'path' | 'runway'

/** Every world render class the gate table covers (contract N2/N3 rows). */
export type WorldRenderClass =
  | 'tree'
  | 'vegetation'
  | 'prop'
  | 'rockLarge'
  | 'building'
  | 'buildingBadge'
  | 'forestFill'
  | 'forestOutline'
  | RoadClass

/** Min-deckZoom gate per class. `forestFill` is the one MAX gate and lives outside this map. */
const MIN_ZOOM_GATES: Record<Exclude<WorldRenderClass, 'forestFill'>, number> = {
  tree: TREE_GLYPH_MIN_ZOOM,
  vegetation: VEGETATION_MIN_ZOOM,
  prop: PROP_MIN_ZOOM,
  rockLarge: ROCK_LARGE_MIN_ZOOM,
  building: BUILDING_FOOTPRINT_MIN_ZOOM,
  buildingBadge: BUILDING_BADGE_MIN_ZOOM,
  forestOutline: FOREST_OUTLINE_MIN_ZOOM,
  highway_paved: -6,
  road_paved: -6,
  road_dirt: -2,
  track: -2,
  path: 4,
  runway: -6,
}

/** Is a class drawn (and pickable — N4) at this deckZoom? Class gates only; per-prefab
 *  importance overrides go through visibleWithImportance. */
export function classVisible(cls: WorldRenderClass, deckZoom: number): boolean {
  if (cls === 'forestFill') return deckZoom <= FOREST_FILL_MAX_ZOOM
  return deckZoom >= MIN_ZOOM_GATES[cls]
}

/** Per-prefab `render.importanceZoom` override (contract N2): an instance is visible when
 *  deckZoom ≥ importanceZoom even if its class gate is higher — landmarks (lighthouse,
 *  transmitter, watertower, military) surface early. Undefined → class gate only. */
export function visibleWithImportance(
  cls: WorldRenderClass,
  deckZoom: number,
  importanceZoom?: number,
): boolean {
  if (classVisible(cls, deckZoom)) return true
  return importanceZoom !== undefined && deckZoom >= importanceZoom
}

/** Budget gate: total would-be-drawn instances across visible classes fits INSTANCE_BUDGET.
 *  T-090.5.3 wires this to type-inventory census integers per band boundary. */
export function instanceBudgetCheck(visibleCounts: number[]): boolean {
  let sum = 0
  for (const n of visibleCounts) sum += n
  return sum <= INSTANCE_BUDGET
}
