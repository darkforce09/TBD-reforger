// T-151.6 — React glue for WgpuSlotsController (mission entities on wgpu).

import { useEffect, type RefObject } from 'react'
import type { WasmMissionDoc } from '../state/wasmDoc'
import type { WgpuSlotsController } from './wgpuSlots'

/** Init once when the engine is ready; re-bind missionDoc when the shell changes. */
export function useWgpuSlots(
  controllerRef: RefObject<WgpuSlotsController | null>,
  ready: boolean,
  missionDoc: WasmMissionDoc | null | undefined,
): void {
  useEffect(() => {
    if (!ready) return
    void controllerRef.current?.init()
  }, [ready, controllerRef])

  useEffect(() => {
    if (!ready) return
    const c = controllerRef.current
    if (!c) return
    c.setMissionDoc(missionDoc ?? null)
    return () => {
      c.setMissionDoc(null)
    }
  }, [ready, missionDoc, controllerRef])
}
