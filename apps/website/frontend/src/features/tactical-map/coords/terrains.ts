// Per-terrain world definition. Arma Reforger terrains are flat local grids in
// meters (no geographic projection) — see Ultra Plan §4.1. World origin is the
// bottom-left corner; +Y points to Arma north. `bounds` is [minX, minY, maxX, maxY]
// in meters. Elevation range from Bohemia Biki (Everon, Arland); DEM manifest may
// refine — see t090_091_map_terrain_program.md.

export type TerrainId = 'everon' | 'arland' | 'custom'

export interface TerrainDef {
  id: TerrainId
  name: string
  /** [minX, minY, maxX, maxY] in meters. */
  bounds: [number, number, number, number]
  /** Side length helpers (meters). */
  width: number
  height: number
  /** DEM / terrain minimum altitude (m ASL). Used from T-091 on. */
  heightRangeMinM: number
  /** DEM / terrain maximum altitude (m ASL). Used from T-091 on. */
  heightRangeMaxM: number
  /** @deprecated Use heightRangeMaxM — kept for callers until T-091.1 lands. */
  maxElevation: number
  /** Relative URL to terrain manifest (T-090). */
  manifestUrl?: string
}

function def(
  id: TerrainId,
  name: string,
  width: number,
  height: number,
  heightRangeMinM: number,
  heightRangeMaxM: number,
  manifestUrl?: string,
): TerrainDef {
  return {
    id,
    name,
    width,
    height,
    bounds: [0, 0, width, height],
    heightRangeMinM,
    heightRangeMaxM,
    maxElevation: heightRangeMaxM,
    manifestUrl,
  }
}

export const TERRAINS: Record<TerrainId, TerrainDef> = {
  everon: def(
    'everon',
    'Everon',
    12800,
    12800,
    -204.78,
    375.53,
    '/map-assets/everon/manifest.json',
  ),
  arland: def('arland', 'Arland', 4096, 4096, -163, 148.38, '/map-assets/arland/manifest.json'),
  custom: def('custom', 'Custom', 12800, 12800, -204.78, 375.53),
}

export const DEFAULT_TERRAIN: TerrainId = 'everon'

export function getTerrain(id?: TerrainId): TerrainDef {
  return (id && TERRAINS[id]) || TERRAINS[DEFAULT_TERRAIN]
}
