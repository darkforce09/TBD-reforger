// T-151.3 W3 — React glue for the wgpu world-object residency controller. The controller lifecycle
// (create in the mount effect, `dispose()` in cleanup) + camera wiring live in `WgpuTacticalMap`;
// this hook only kicks the manifest/prefab/index load once the engine is ready. The component is
// keyed on terrain in `MissionCreatorPage`, so a terrain switch remounts everything (fresh engine +
// controller) — `terrainId` in the dep array is belt-and-suspenders.

import { useEffect } from 'react'
import type { RefObject } from 'react'
import type { TerrainId } from '../coords/terrains'
import type { WgpuWorldController } from './wgpuWorldLoader'

export function useWgpuWorldResidency(
  controllerRef: RefObject<WgpuWorldController | null>,
  ready: boolean,
  opts: { terrainId: TerrainId },
): void {
  const { terrainId } = opts
  useEffect(() => {
    if (!ready) return
    void controllerRef.current?.init()
  }, [ready, terrainId, controllerRef])
}
