// Entity icons (Ultra Plan §4.3 layer #5). Deck's IconLayer renders every slot as a
// pixel-sized marker — Deck (not the DOM) draws them all, which is the answer to the
// "200 Slot Problem". Picking is off (T-063): clicks/marquees query the slotSpatialIndex
// R-tree instead. Phase 4 renders test slots; vehicles/waypoints join later. Positions are
// RAW world meters (same convention as the grid).

import { useMemo } from 'react'
import { IconLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { useMapStore } from '../state/useMapStore'
import { getBaseIcons, getSelectedIcons } from '../state/slotIconCache'
import { selectDragOverlayIcons, type SlotIcon } from '../state/selectors'
import type { Selection } from '../state/schema'

const ICON_SIZE = 64
const PRIMARY: [number, number, number, number] = [173, 198, 255, 255] // Aegis primary
const SELECTED: [number, number, number, number] = [250, 204, 21, 255] // tactical yellow

let markerUrl: string | null = null

/** One-time white disc (mask=true → tinted by getColor) on transparent. */
function getMarkerIcon(): string {
  if (markerUrl) return markerUrl
  const canvas = document.createElement('canvas')
  canvas.width = ICON_SIZE
  canvas.height = ICON_SIZE
  const ctx = canvas.getContext('2d')!
  const c = ICON_SIZE / 2
  ctx.fillStyle = '#ffffff'
  ctx.beginPath()
  ctx.arc(c, c, c - 8, 0, Math.PI * 2)
  ctx.fill()
  // Punch a hole so it reads as a ring marker.
  ctx.globalCompositeOperation = 'destination-out'
  ctx.beginPath()
  ctx.arc(c, c, c - 22, 0, Math.PI * 2)
  ctx.fill()
  markerUrl = canvas.toDataURL('image/png')
  return markerUrl
}

const ICON_MAPPING = {
  marker: {
    x: 0,
    y: 0,
    width: ICON_SIZE,
    height: ICON_SIZE,
    anchorX: ICON_SIZE / 2,
    anchorY: ICON_SIZE / 2,
    mask: true,
  },
}

// Base map icons (T-061.0.1): the persistent slotIconCache is the source of truth. The cache
// holds the dragged ids out (exclude) during a move — those render in useDragIconLayer — and
// patches positions / selection flags in place, so the boundaries are O(k), never an O(n)
// re-derive from slotsById. The layer subscribes only `iconCacheVersion`: it bumps at drag
// pickup/release, selection change and snapshot replace — NOT per drag frame (motion) and NOT
// on pan, so neither regresses. `getBaseIcons()` hands a fresh array identity per version so
// Deck re-packs, while reusing the SlotIcon objects.
//
// T-065: in cluster mode (`detail === false`) the cluster layers cover the bulk of the slots, so
// the base IconLayer renders ONLY the current selection (highlighted, over the bubbles). `detail`
// is a derived boolean — the layer rebuilds only when the mode flips, never per zoom tick.
//
// T-067.0.1: viewport CPU-cull reverted. `getBaseIcons()` returns a cached array whose identity is
// stable across pan frames (only the version moves it), so Deck uploads the icon attributes once and
// never re-packs while panning — the pre-T-067 ~160 fps contract @ ~367k. The earlier chunk-rect cull
// swapped the IconLayer `data` whenever the viewport crossed a 512m chunk; at zoom -2 a normal drag
// crosses several chunks PER FRAME (high m/px) and the populated area is fully in view, so it re-packed
// ~360k attributes every frame (165→21 fps) for zero benefit. True render scaling for the 1M+ north
// star wants GPU-side culling (DataFilterExtension: one stable buffer, viewport bounds as a uniform)
// or clustering/residency — a separate slice; the slotIconCache chunk machinery stays in place for it.
export function useIconLayer({
  detail,
  selection,
}: {
  detail: boolean
  selection: Selection
}): IconLayer<SlotIcon> {
  const iconCacheVersion = useMapStore((s) => s.iconCacheVersion)
  // Stable array identity across pan frames: getBaseIcons() returns a cached view per version, and the
  // selected-only branch only re-derives when the mode/selection/version changes.
  const icons = useMemo(
    () =>
      detail
        ? getBaseIcons()
        : getSelectedIcons(selection.kind !== 'none' ? new Set(selection.ids) : new Set()),
    // iconCacheVersion is the cache-mutation signal getBaseIcons/getSelectedIcons read from —
    // eslint can't see the dependency through the module-level cache.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [detail, selection, iconCacheVersion],
  )

  return useMemo(
    () =>
      new IconLayer<SlotIcon>({
        id: 'slot-icons',
        coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
        data: icons,
        getIcon: () => 'marker',
        iconAtlas: getMarkerIcon(),
        iconMapping: ICON_MAPPING,
        getPosition: (d) => [d.x, d.y],
        getSize: (d) => (d.selected ? 28 : 20),
        getColor: (d) => (d.selected ? SELECTED : PRIMARY),
        sizeUnits: 'pixels',
        // Not pickable (T-063): click/marquee/dbl-click pick via the slotSpatialIndex R-tree,
        // not Deck's GPU pick over all ~360k icons.
        pickable: false,
        updateTriggers: {
          getPosition: iconCacheVersion,
          getSize: iconCacheVersion,
          getColor: iconCacheVersion,
        },
      }),
    [icons, iconCacheVersion],
  )
}

// Drag-preview overlay (T-061): only the dragged ids, offset by the live world delta.
// Typically 1–500 icons, so rebuilding its positions every frame is cheap. Not pickable —
// the gesture is already captured (and base picking is the spatial index, not Deck; T-063).
// Returns null when no drag is active.
export function useDragIconLayer(): IconLayer<SlotIcon> | null {
  const slotsById = useMapStore((s) => s.slotsById)
  const dragPreviewIds = useMapStore((s) => s.dragPreviewIds)
  const dragPreviewDelta = useMapStore((s) => s.dragPreviewDelta)
  const icons = selectDragOverlayIcons(slotsById, dragPreviewIds, dragPreviewDelta)
  const posKey = dragPreviewDelta ? `${dragPreviewDelta.dx},${dragPreviewDelta.dy}` : ''

  return useMemo(() => {
    if (!icons.length) return null
    return new IconLayer<SlotIcon>({
      id: 'slot-icons-drag',
      coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
      data: icons,
      getIcon: () => 'marker',
      iconAtlas: getMarkerIcon(),
      iconMapping: ICON_MAPPING,
      getPosition: (d) => [d.x, d.y],
      getSize: 28,
      getColor: SELECTED,
      sizeUnits: 'pixels',
      pickable: false,
      updateTriggers: {
        getPosition: posKey,
      },
    })
  }, [icons, posKey])
}
