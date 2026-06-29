// Reactive DEM state version (T-091.2 follow-up). The DEM loads asynchronously, so any layer
// or read-out that depends on it must re-render when it becomes ready/degraded. This subscribes
// to the DemController external store; the returned counter changes on every state transition.
// Calling it inside a React.memo'd component still re-renders that component (internal
// subscriptions are not gated by memo).

import { useSyncExternalStore } from 'react'
import { subscribeDem, getDemVersion } from './DemController'

export function useDemVersion(): number {
  return useSyncExternalStore(subscribeDem, getDemVersion, getDemVersion)
}
