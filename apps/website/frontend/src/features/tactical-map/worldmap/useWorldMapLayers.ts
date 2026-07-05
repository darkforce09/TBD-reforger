// T-090.5.1 — Map Engine v2 layer assembly hook (scaffold). This is the single insertion
// point for world-object layers in TacticalMap's ordered array (plan §4.2 slots: sea →
// land-cover → contours → roads → buildings → forest → trees/props; hillshade + grid stay
// their own hooks). Scaffold returns [] ALWAYS — no world layers exist yet, and with
// WORLDMAP_ENABLED off nothing may ever mount (risk R3: first paint identical to today).
// T-090.5.2+ builds the per-class layer arrays here from chunkStore data + lodGates +
// worldLayerPrefs toggles.

import { useMemo } from 'react'
import type { Layer } from '@deck.gl/core'
import { WORLDMAP_ENABLED } from './config'

export function useWorldMapLayers(): Layer[] {
  return useMemo(() => {
    if (!WORLDMAP_ENABLED) return []
    // Flag on, nothing to draw yet: chunk streaming (T-090.5.3) + layer builders
    // (T-090.5.2/.8.1) land in later slices.
    return []
  }, [])
}
