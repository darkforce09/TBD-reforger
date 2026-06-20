// World (Arma meters) <-> Deck common space for the OrthographicView.
//
// The view is configured with `flipY: false` (see view/useOrthographicView.ts), so
// Deck's common space already matches Arma world space: origin bottom-left, +Y =
// north (up on screen). Under CARTESIAN there is therefore NO remapping — common
// space == world space — so these conversions are identity passthroughs. The base
// map grid, the icon layer and flyTo() all place entities at raw [x, y]; keeping a
// single shared convention here prevents the classic double-flip bug.

import type { TerrainDef } from './terrains'

/** World meters -> Deck common space (identity under flipY:false / CARTESIAN). */
export function worldToPixel(
  _terrain: TerrainDef,
  x: number,
  y: number,
): [number, number] {
  return [x, y]
}

/** Deck common space -> world meters (identity under flipY:false / CARTESIAN). */
export function pixelToWorld(
  _terrain: TerrainDef,
  px: number,
  py: number,
): [number, number] {
  return [px, py]
}

/** Center of the terrain in world/common space — the default camera target. */
export function terrainCenterPixel(terrain: TerrainDef): [number, number] {
  return [terrain.width / 2, terrain.height / 2]
}
