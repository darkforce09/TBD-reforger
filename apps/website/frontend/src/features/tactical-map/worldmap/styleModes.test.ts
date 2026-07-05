// T-090.5.1 — style-mode mapping contract (implementation plan §4.3, locked): satellite 1.0 /
// hybrid 0.55 / map 0 (+ paper tint), and the interim raster routing (map keeps the legacy
// pyramid until T-090.10.2).
import { describe, it, expect } from 'vitest'
import { PAPER_TINT, basemapViewForStyle, styleForMode } from './styleModes'

describe('styleModes (plan §4.3)', () => {
  it('satellite → full-opacity sat, no paper, overlay emphasis', () => {
    expect(styleForMode('satellite')).toEqual({
      satOpacity: 1.0,
      paperTint: null,
      vectorEmphasis: 'overlay',
    })
  })

  it('hybrid → sat dimmed to 0.55, vectors full', () => {
    expect(styleForMode('hybrid')).toEqual({
      satOpacity: 0.55,
      paperTint: null,
      vectorEmphasis: 'full',
    })
  })

  it('map → sat hidden, paper tint on, cartographic emphasis', () => {
    expect(styleForMode('map')).toEqual({
      satOpacity: 0,
      paperTint: PAPER_TINT,
      vectorEmphasis: 'cartographic',
    })
    expect(PAPER_TINT).toEqual([0xcd, 0xc6, 0xa3])
  })

  it('raster routing: satellite + hybrid draw the satellite field; map keeps the legacy pyramid', () => {
    expect(basemapViewForStyle('satellite')).toBe('satellite')
    expect(basemapViewForStyle('hybrid')).toBe('satellite')
    expect(basemapViewForStyle('map')).toBe('map')
  })
})
