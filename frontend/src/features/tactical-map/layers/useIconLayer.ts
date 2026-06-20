// Entity icons (Ultra Plan §4.3 layer #5). Deck's IconLayer renders every slot as a
// pixel-sized, pickable marker — Deck (not the DOM) draws them all, which is the
// answer to the "200 Slot Problem". Phase 4 renders test slots; vehicles/waypoints
// join later. Positions are RAW world meters (same convention as the grid).

import { useMemo } from 'react'
import { IconLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { useMapStore } from '../state/useMapStore'
import { selectSlotIcons, type SlotIcon } from '../state/selectors'

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

export function useIconLayer(): IconLayer<SlotIcon> {
  const slotsById = useMapStore((s) => s.slotsById)
  const selection = useMapStore((s) => s.selection)
  const icons = selectSlotIcons(slotsById, selection)

  return useMemo(
    () =>
      new IconLayer<SlotIcon>({
        id: 'slot-icons',
        coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
        data: icons,
        getIcon: () => 'marker',
        iconAtlas: getMarkerIcon(),
        iconMapping: {
          marker: {
            x: 0,
            y: 0,
            width: ICON_SIZE,
            height: ICON_SIZE,
            anchorX: ICON_SIZE / 2,
            anchorY: ICON_SIZE / 2,
            mask: true,
          },
        },
        getPosition: (d) => [d.x, d.y],
        getSize: (d) => (d.selected ? 28 : 20),
        getColor: (d) => (d.selected ? SELECTED : PRIMARY),
        sizeUnits: 'pixels',
        pickable: true,
        updateTriggers: {
          getSize: selection.id,
          getColor: selection.id,
        },
      }),
    [icons, selection.id],
  )
}
