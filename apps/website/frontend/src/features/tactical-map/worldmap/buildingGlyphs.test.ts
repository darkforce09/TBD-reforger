// T-152.3 — Class R oracle policy (Rust compose must match these keys).
import { describe, expect, it } from 'vitest'

import {
  BUILDING_CLASSES,
  badgeIconKey,
  buildingIconKey,
  landmarkGlyphIconKey,
} from './buildingGlyphs'
import { classVisible } from './lodGates'

describe('buildingGlyphs oracle (T-152.3 G1)', () => {
  it('buildingIconKey covers every BUILDING_CLASSES entry', () => {
    for (const cls of BUILDING_CLASSES) {
      expect(buildingIconKey(cls)).toBe(`building-${cls}`)
    }
    expect(buildingIconKey('pier')).toBeNull()
  })

  it('landmarkGlyphIconKey prefers badge overlay', () => {
    expect(landmarkGlyphIconKey('military')).toBe('building-badge-military')
    expect(landmarkGlyphIconKey('lighthouse')).toBe('building-lighthouse')
    expect(landmarkGlyphIconKey('castle')).toBe('building-castle')
    expect(badgeIconKey('residential')).toBeNull()
  })
})

describe('buildingBadge zoom gate (T-152.3 G3)', () => {
  it('badges off below +1, on at +1', () => {
    expect(classVisible('buildingBadge', 0.9)).toBe(false)
    expect(classVisible('buildingBadge', 1.0)).toBe(true)
  })
})
