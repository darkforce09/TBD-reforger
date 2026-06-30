// Cluster / LOD layer (T-065, pan-stabilized T-065.2). At extreme zoom-out on a large mission
// (clusterMode) the map draws cluster discs from the slotClusterIndex (supercluster) instead of all
// ~367k individual IconLayer markers — Eden's "group icon stacked when zoomed out". Disc size
// encodes the aggregated count (no per-frame TextLayer). pickable:false (T-063) — the cluster
// drill-in hit-tests slotClusterIndex.pickClusterAt, never Deck's GPU pick.
//
// Pan-stability contract (T-061/T-065.2): the cluster markers come from a module-level cache keyed
// on the FULL terrain + supercluster zoom bucket (slotClusterIndex.getClusterMarkers), recomputed
// ONLY when an edit dirties them or the zoom bucket changes. So a pan returns the SAME `markers`
// reference → the same IconLayer instance → Deck transforms the view only (no per-frame rebuild /
// re-upload). Mirrors useIconLayer reading slotIconCache.getBaseIcons + iconCacheVersion.

import { useMemo } from 'react'
import { IconLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { useMapStore } from '../state/useMapStore'
import {
  getClusterMarkers,
  getClusterMarkersVersion,
  type ClusterMarker,
} from '../state/slotClusterIndex'

const DISC_SIZE = 64
const PRIMARY: [number, number, number, number] = [173, 198, 255, 235] // Aegis primary
const EMPTY: ClusterMarker[] = []

let discUrl: string | null = null

/** One-time solid white disc (mask=true → tinted by getColor) on transparent. */
function getDiscIcon(): string {
  if (discUrl) return discUrl
  const canvas = document.createElement('canvas')
  canvas.width = DISC_SIZE
  canvas.height = DISC_SIZE
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('2d canvas context unavailable')
  const c = DISC_SIZE / 2
  ctx.fillStyle = '#ffffff'
  ctx.beginPath()
  ctx.arc(c, c, c - 6, 0, Math.PI * 2)
  ctx.fill()
  discUrl = canvas.toDataURL('image/png')
  return discUrl
}

const ICON_MAPPING = {
  disc: {
    x: 0,
    y: 0,
    width: DISC_SIZE,
    height: DISC_SIZE,
    anchorX: DISC_SIZE / 2,
    anchorY: DISC_SIZE / 2,
    mask: true,
  },
}

/** Disc grows with the aggregated count so dense clusters read as bigger (the count readout — no
 *  TextLayer). */
function discSize(count: number): number {
  return 22 + Math.min(26, Math.log10(Math.max(count, 1)) * 12)
}

interface UseClusterIconLayerArgs {
  clusterMode: boolean
  deckZoom: number
}

export function useClusterIconLayer({
  clusterMode,
  deckZoom,
}: UseClusterIconLayerArgs): Array<IconLayer<ClusterMarker>> {
  // Self-subscribe iconCacheVersion (mirror useIconLayer) — defense-in-depth so an edit re-runs the
  // hook even if the host's other store subscriptions didn't change.
  const iconCacheVersion = useMapStore((s) => s.iconCacheVersion)
  const markers = clusterMode ? getClusterMarkers(deckZoom) : EMPTY
  const version = getClusterMarkersVersion()

  return useMemo(() => {
    if (!clusterMode || !markers.length) return []
    return [
      new IconLayer<ClusterMarker>({
        id: 'cluster-icons',
        coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
        data: markers,
        getIcon: () => 'disc',
        iconAtlas: getDiscIcon(),
        iconMapping: ICON_MAPPING,
        getPosition: (d) => [d.x, d.y],
        getSize: (d) => discSize(d.count),
        getColor: PRIMARY,
        sizeUnits: 'pixels',
        pickable: false,
        updateTriggers: { getSize: version },
      }),
    ]
    // markers + version already capture every real change; iconCacheVersion + deckZoom are kept as
    // defense-in-depth invalidation guards (eslint sees them as unused in the body).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clusterMode, markers, version, iconCacheVersion, deckZoom])
}
