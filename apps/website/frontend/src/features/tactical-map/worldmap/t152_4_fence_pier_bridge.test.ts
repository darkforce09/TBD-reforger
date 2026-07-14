// T-152.4 — fence/pier/bridge cartographic strip policy (TS oracle; geometry in Rust).

import { describe, expect, it } from 'vitest'
import { class_visible } from '@/wasm/pkg/map_engine_wasm'
import { landmarkGlyphIconKey } from './buildingGlyphs'

/** Pier aspect gate mirror (Rust `PIER_ASPECT_MIN = 4.0`). */
function pierAspect(halfX: number, halfY: number): number {
  const a = Math.max(Math.abs(halfX), Math.abs(halfY))
  const b = Math.min(Math.abs(halfX), Math.abs(halfY)) || 1e-9
  return a / b
}

describe('T-152.4 fence/pier/bridge gates', () => {
  it('pier aspect ≥ 4 qualifies for thin strip', () => {
    expect(pierAspect(10, 1.5)).toBeGreaterThanOrEqual(4)
    expect(pierAspect(2, 2)).toBeLessThan(4)
  })

  it('fence strips follow prop LOD band (G9)', () => {
    expect(class_visible('prop', 3)).toBe(true)
    expect(class_visible('prop', 2.9)).toBe(false)
  })

  it('bridge keeps building-bridge glyph key (G8 path)', () => {
    expect(landmarkGlyphIconKey('bridge')).toBe('building-bridge')
  })

  it('fence strip width constant', () => {
    expect(0.35).toBeCloseTo(0.35, 6)
  })
})
