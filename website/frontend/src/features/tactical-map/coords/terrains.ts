// Per-terrain world definition. Arma Reforger terrains are flat local grids in
// meters (no geographic projection) — see Ultra Plan §4.1. World origin is the
// bottom-left corner; +Y points to Arma north. `bounds` is [minX, minY, maxX, maxY]
// in meters; `maxElevation` is the DEM white-point used in later phases.

export type TerrainId = 'everon' | 'arland' | 'custom'

export interface TerrainDef {
  id: TerrainId
  name: string
  /** [minX, minY, maxX, maxY] in meters. */
  bounds: [number, number, number, number]
  /** Side length helpers (meters). */
  width: number
  height: number
  /** DEM white-point in meters (max terrain elevation). Used from Phase 2 on. */
  maxElevation: number
}

function def(
  id: TerrainId,
  name: string,
  width: number,
  height: number,
  maxElevation: number,
): TerrainDef {
  return { id, name, width, height, bounds: [0, 0, width, height], maxElevation }
}

export const TERRAINS: Record<TerrainId, TerrainDef> = {
  everon: def('everon', 'Everon', 12800, 12800, 1024),
  arland: def('arland', 'Arland', 10240, 10240, 512),
  custom: def('custom', 'Custom', 12800, 12800, 1024),
}

export const DEFAULT_TERRAIN: TerrainId = 'everon'

export function getTerrain(id?: TerrainId): TerrainDef {
  return (id && TERRAINS[id]) || TERRAINS[DEFAULT_TERRAIN]
}
