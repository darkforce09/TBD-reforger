import { describe, it, expect } from 'vitest'
import { deviceSize } from './wasmRender'

// Literal-expectation pins for the one CSS→device-pixel rounding rule (T-151 plan §S5) —
// the same formula lives in RenderEngine::resize (js_round = half toward +∞), so these
// literals are the contract that canvas backing store and surface config always agree.
describe('deviceSize — css→device pixel rounding', () => {
  it('matches the pinned literals across dpr 1 / 1.25 / 1.5 / 2 / 2.75', () => {
    expect(deviceSize(800, 600, 1)).toEqual([800, 600])
    expect(deviceSize(1237.33, 842.67, 1)).toEqual([1237, 843])
    expect(deviceSize(1237.33, 842.67, 1.25)).toEqual([1547, 1053]) // 1546.6625, 1053.3375
    expect(deviceSize(1023.5, 767.5, 1.5)).toEqual([1535, 1151]) // 1535.25, 1151.25
    expect(deviceSize(1023.5, 767.5, 2)).toEqual([2047, 1535])
    expect(deviceSize(333.4, 111.2, 2.75)).toEqual([917, 306]) // 916.85, 305.8
  })

  it('rounds .5 halves toward +∞ (JS Math.round, not half-away-from-zero)', () => {
    expect(deviceSize(100.5, 200.5, 1)).toEqual([101, 201])
  })

  it('clamps sub-pixel sizes to 1 (never a zero-sized surface)', () => {
    expect(deviceSize(0.2, 0.2, 1)).toEqual([1, 1])
  })
})
