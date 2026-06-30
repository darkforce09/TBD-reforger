import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the DEM leaf so ydoc's terrainZ() is deterministic (T-091.2). vi.hoisted lets the
// factory share mutable state with the tests.
const dem = vi.hoisted(() => ({ ready: true, elevation: 123.456 }))
vi.mock('../dem', () => ({
  sampleElevation: () => dem.elevation,
  isDemReady: () => dem.ready,
}))

import {
  createMissionDoc,
  addSlot,
  pasteSlots,
  moveEntities,
  updateSlotPosition,
} from './ydoc'
import type { Slot } from './schema'

function posOf(md: ReturnType<typeof createMissionDoc>, id: string): Slot['position'] {
  const slot = md.entities.slots.get(id)
  if (!slot) throw new Error(`slot ${id} not found`)
  return slot.get('position') as Slot['position']
}

beforeEach(() => {
  dem.ready = true
  dem.elevation = 123.456
})

describe('ydoc terrain z sampling (T-091.2)', () => {
  it('addSlot samples z when DEM ready', () => {
    const md = createMissionDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    expect(posOf(md, id).z).toBe(123.456)
  })

  it('addSlot z = 0 when DEM not ready', () => {
    dem.ready = false
    const md = createMissionDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    expect(posOf(md, id).z).toBe(0)
  })

  it('moveEntities re-samples z on commit', () => {
    const md = createMissionDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    dem.elevation = 50.5
    moveEntities(md, [id], { x: 10, y: 20 })
    const p = posOf(md, id)
    expect(p.x).toBe(110)
    expect(p.y).toBe(220)
    expect(p.z).toBe(50.5)
  })

  it('pasteSlots re-samples z at pasted x/y (not clipboard z)', () => {
    const md = createMissionDoc()
    dem.elevation = 77.7
    const ids = pasteSlots(md, [
      {
        squadId: 'nope',
        role: 'Rifleman',
        stance: 'stand',
        position: { x: 300, y: 400, z: 999, rotation: 0 },
      } as never,
    ])
    expect(posOf(md, ids[0]).z).toBe(77.7) // not 999
  })

  it('updateSlotPosition: manual Z sticks; X/Y edit terrain-follows; rotation-only leaves z', () => {
    const md = createMissionDoc()
    const id = addSlot(md, { x: 100, y: 200 }) // z = 123.456
    dem.elevation = 88.8

    // Manual Z patch sticks.
    updateSlotPosition(md, id, { z: 5 })
    expect(posOf(md, id).z).toBe(5)

    // X-only edit re-samples.
    updateSlotPosition(md, id, { x: 150 })
    expect(posOf(md, id).z).toBe(88.8)

    // Rotation-only leaves z untouched.
    dem.elevation = 11.1
    updateSlotPosition(md, id, { rotation: 90 })
    expect(posOf(md, id).z).toBe(88.8)
  })
})
