import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the DEM leaf so ydoc's terrainZ() is deterministic (T-091.2). vi.hoisted lets the
// factory share mutable state with the tests.
const dem = vi.hoisted(() => ({ ready: true, elevation: 123.456 }))
vi.mock('../dem', () => ({
  sampleElevation: () => dem.elevation,
  isDemReady: () => dem.ready,
}))

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { createMissionDoc, addSlot, pasteSlots, moveEntities, updateSlotPosition } from './ydoc'
import { useMapStore } from './useMapStore'
import type { MissionDoc } from './ydoc'
import type { Slot } from './schema'

// Post-flip (T-145 F3) the wrappers write the wasm doc + resync the Zustand store; read positions from
// the store (the render/read mirror), not the doc. Each test gets a fresh wasm handle + a reset store
// (the store is a shared singleton).
function makeDoc(): MissionDoc {
  const md = createMissionDoc()
  md.attach(new wasm.MissionDoc())
  return md
}

function posOf(id: string): Slot['position'] {
  const slot = useMapStore.getState().slotsById[id]
  if (!slot) throw new Error(`slot ${id} not found`)
  return slot.position
}

beforeEach(() => {
  dem.ready = true
  dem.elevation = 123.456
  useMapStore.getState().reset()
})

describe('ydoc terrain z sampling (T-091.2)', () => {
  it('addSlot samples z when DEM ready', () => {
    const md = makeDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    expect(posOf(id).z).toBe(123.456)
    md.detach()
  })

  it('addSlot z = 0 when DEM not ready', () => {
    dem.ready = false
    const md = makeDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    expect(posOf(id).z).toBe(0)
    md.detach()
  })

  it('moveEntities re-samples z on commit', () => {
    const md = makeDoc()
    const id = addSlot(md, { x: 100, y: 200 })
    dem.elevation = 50.5
    moveEntities(md, [id], { x: 10, y: 20 })
    const p = posOf(id)
    expect(p.x).toBe(110)
    expect(p.y).toBe(220)
    expect(p.z).toBe(50.5)
    md.detach()
  })

  it('pasteSlots re-samples z at pasted x/y (not clipboard z)', () => {
    const md = makeDoc()
    dem.elevation = 77.7
    const ids = pasteSlots(md, [
      {
        squadId: 'nope',
        role: 'Rifleman',
        stance: 'stand',
        position: { x: 300, y: 400, z: 999, rotation: 0 },
      } as never,
    ])
    expect(posOf(ids[0]).z).toBe(77.7) // not 999
    md.detach()
  })

  it('updateSlotPosition: manual Z sticks; X/Y edit terrain-follows; rotation-only leaves z', () => {
    const md = makeDoc()
    const id = addSlot(md, { x: 100, y: 200 }) // z = 123.456
    dem.elevation = 88.8

    // Manual Z patch sticks.
    updateSlotPosition(md, id, { z: 5 })
    expect(posOf(id).z).toBe(5)

    // X-only edit re-samples.
    updateSlotPosition(md, id, { x: 150 })
    expect(posOf(id).z).toBe(88.8)

    // Rotation-only leaves z untouched.
    dem.elevation = 11.1
    updateSlotPosition(md, id, { rotation: 90 })
    expect(posOf(id).z).toBe(88.8)
    md.detach()
  })
})
