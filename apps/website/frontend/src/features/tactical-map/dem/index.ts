// Public DEM API (T-091.1). T-091.2 depends on these exact names. _resetForTest is
// intentionally NOT re-exported here (test-only — import it from DemController directly).
export { loadDemForTerrain, isDemReady, isDemDegraded, sampleElevation } from './DemController'
export type { TerrainManifest } from './terrainManifest'
