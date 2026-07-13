// T-154 doll parity (Class R, cross-language contract): the wasm doll's region list must
// equal the frontend RAIL_REGIONS exactly (order + names — set_states bytes and pick
// indexes are positions in this list), and the pure CPU pick must land the same goldens
// the cargo tests assert. No GPU — doll_pick_cpu is the same core math DollEngine runs.

import { describe, expect, it } from 'vitest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { RAIL_REGIONS } from '@/features/mission-creator/loadout/arsenalDollModel'

describe('doll region contract', () => {
  it('wasm REGION_KEYS === RAIL_REGIONS keys (order + names)', () => {
    const keys = JSON.parse(wasm.doll_region_keys()) as string[]
    expect(keys).toEqual(RAIL_REGIONS.map((r) => r.key))
  })
})

describe('doll_pick_cpu goldens (mirror of the cargo pick tests)', () => {
  const W = 800
  const H = 600
  const key = (idx: number) => (idx >= 0 ? RAIL_REGIONS[idx]?.key : null)

  it('front view: screen center hits the rifle receiver', () => {
    expect(key(wasm.doll_pick_cpu(0, W, H, 400, 300))).toBe('primary')
  })

  it('misses: sky above the soldier and far off to the side', () => {
    expect(wasm.doll_pick_cpu(0, W, H, 400, 10)).toBe(-1)
    expect(wasm.doll_pick_cpu(0, W, H, 20, 300)).toBe(-1)
  })

  it('back view (yaw = π): torso center hits back-mounted gear', () => {
    const hit = key(wasm.doll_pick_cpu(Math.PI, W, H, 400, 280))
    expect(['backpack', 'armoredVest', 'launcher']).toContain(hit)
  })
})
