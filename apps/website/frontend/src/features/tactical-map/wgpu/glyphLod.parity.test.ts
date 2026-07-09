// T-151.5 L6 — exhaustive LOD scan: Rust `class_visible` == TS `classVisible` for every
// world render class × zoom ∈ {−6.0 … +6.0} step 0.1.
import { describe, it, expect } from 'vitest'
import { class_visible } from '@/wasm/pkg/map_engine_wasm'
import { classVisible, type WorldRenderClass } from '../worldmap/lodGates'

const CLASSES: WorldRenderClass[] = [
  'tree',
  'vegetation',
  'prop',
  'rockLarge',
  'building',
  'buildingBadge',
  'forestFill',
  'forestOutline',
  'sea',
  'contour',
  'highway_paved',
  'road_paved',
  'road_dirt',
  'track',
  'path',
  'runway',
]

describe('T-151.5 class_visible Rust↔TS exhaustive scan', () => {
  it('matches lodGates.classVisible for all classes × zooms −6…+6 @ 0.1', () => {
    let checked = 0
    for (const cls of CLASSES) {
      for (let i = 0; i <= 120; i++) {
        const z = Math.round((-6 + i * 0.1) * 10) / 10
        const rust = class_visible(cls, z)
        const ts = classVisible(cls, z)
        expect(rust, `${cls} @ ${z}`).toBe(ts)
        checked++
      }
    }
    // 16 classes × 121 zooms
    expect(checked).toBe(16 * 121)
  })
})
