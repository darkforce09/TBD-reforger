// T-090.5.1 — lodGates must mirror the canonical N2 constants + N3 band behavior in
// t090_render_lod_contract.md (v2) verbatim (contract LOD4), with no world-cluster remnant
// (LOD5). Band spot-checks pin the default-zoom (−2) experience: buildings as OBB rects,
// forests as polygons, trees hidden.
import { describe, it, expect } from 'vitest'
import * as gates from './lodGates'
import {
  classVisible,
  contourIntervalForZoom,
  instanceBudgetCheck,
  visibleWithImportance,
} from './lodGates'

describe('lodGates N2 constants (LOD4)', () => {
  it('exports the v2 contract values verbatim', () => {
    expect(gates.REF_ZOOM).toBe(3)
    expect(gates.TREE_GLYPH_MIN_ZOOM).toBe(0)
    expect(gates.FOREST_FILL_MAX_ZOOM).toBe(1)
    expect(gates.FOREST_OUTLINE_MIN_ZOOM).toBe(-1.5)
    expect(gates.BUILDING_FOOTPRINT_MIN_ZOOM).toBe(-2.5)
    expect(gates.BUILDING_BADGE_MIN_ZOOM).toBe(1)
    expect(gates.VEGETATION_MIN_ZOOM).toBe(1.5)
    expect(gates.PROP_MIN_ZOOM).toBe(3)
    expect(gates.ROCK_LARGE_MIN_ZOOM).toBe(1)
    expect(gates.PICK_RADIUS_PX).toBe(12)
    expect(gates.INSTANCE_BUDGET).toBe(150_000)
  })

  it('has no world-cluster export (LOD5 — v1 constants deleted)', () => {
    expect(Object.keys(gates).filter((k) => /CLUSTER/i.test(k))).toEqual([])
  })
})

describe('classVisible bands (N3)', () => {
  it('default zoom −2: buildings = rects, forests = fill (no outline yet), trees hidden', () => {
    expect(classVisible('building', -2)).toBe(true) // −2 ≥ −2.5
    expect(classVisible('forestFill', -2)).toBe(true)
    expect(classVisible('forestOutline', -2)).toBe(false) // outline from −1.5
    expect(classVisible('tree', -2)).toBe(false)
    expect(classVisible('vegetation', -2)).toBe(false)
    expect(classVisible('prop', -2)).toBe(false)
    expect(classVisible('buildingBadge', -2)).toBe(false)
  })

  it('whole-island −6: only always-on road classes', () => {
    expect(classVisible('highway_paved', -6)).toBe(true)
    expect(classVisible('road_paved', -6)).toBe(true)
    expect(classVisible('runway', -6)).toBe(true)
    expect(classVisible('road_dirt', -6)).toBe(false)
    expect(classVisible('track', -6)).toBe(false)
    expect(classVisible('building', -6)).toBe(false)
    expect(classVisible('forestFill', -6)).toBe(true)
  })

  it('road ladder: dirt/track from −2, path only from +4', () => {
    expect(classVisible('road_dirt', -2)).toBe(true)
    expect(classVisible('track', -2)).toBe(true)
    expect(classVisible('path', 3)).toBe(false)
    expect(classVisible('path', 4)).toBe(true)
  })

  it('glyph bands: trees from 0, rocks/badges from +1, vegetation from +1.5, props from +3', () => {
    expect(classVisible('tree', 0)).toBe(true)
    expect(classVisible('rockLarge', 1)).toBe(true)
    expect(classVisible('rockLarge', 0.5)).toBe(false)
    expect(classVisible('buildingBadge', 1)).toBe(true)
    expect(classVisible('vegetation', 1.5)).toBe(true)
    expect(classVisible('vegetation', 1)).toBe(false)
    expect(classVisible('prop', 3)).toBe(true)
    expect(classVisible('prop', 2.5)).toBe(false)
  })

  it('forest fill is a MAX gate: on at ≤ +1, off past it (glyphs take over)', () => {
    expect(classVisible('forestFill', 1)).toBe(true)
    expect(classVisible('forestFill', 1.5)).toBe(false)
    expect(classVisible('forestFill', 6)).toBe(false)
  })

  it('sea is a MAX gate (T-090.5.4): on through +3, off past it; visible at default −2', () => {
    expect(gates.SEA_FILL_MAX_ZOOM).toBe(3)
    expect(classVisible('sea', -2)).toBe(true)
    expect(classVisible('sea', -6)).toBe(true)
    expect(classVisible('sea', 3)).toBe(true)
    expect(classVisible('sea', 4)).toBe(false)
  })

  it('contours draw across the whole band (min −6, T-090.5.4)', () => {
    expect(classVisible('contour', -6)).toBe(true)
    expect(classVisible('contour', -2)).toBe(true)
    expect(classVisible('contour', 6)).toBe(true)
  })
})

describe('contourIntervalForZoom ladder (N3, T-090.5.4)', () => {
  it('steps 100 → 50 → 20 → 10 m, edges to the finer band', () => {
    expect(contourIntervalForZoom(-6)).toBe(100)
    expect(contourIntervalForZoom(-5)).toBe(100)
    expect(contourIntervalForZoom(-4)).toBe(50) // edge to finer
    expect(contourIntervalForZoom(-3)).toBe(50)
    expect(contourIntervalForZoom(-2.5)).toBe(20) // edge to finer
    expect(contourIntervalForZoom(-2)).toBe(20) // ticket vitest pin
    expect(contourIntervalForZoom(0)).toBe(20)
    expect(contourIntervalForZoom(1)).toBe(10) // edge to finer
    expect(contourIntervalForZoom(3)).toBe(10)
    expect(contourIntervalForZoom(6)).toBe(10)
  })
})

describe('visibleWithImportance (N2 per-prefab override)', () => {
  it('landmark importanceZoom −4 surfaces a prop-class prefab at whole-island zoom', () => {
    expect(visibleWithImportance('prop', -4, -4)).toBe(true)
    expect(visibleWithImportance('prop', -4.5, -4)).toBe(false)
    expect(visibleWithImportance('prop', -4, undefined)).toBe(false)
  })

  it('never hides below the class gate', () => {
    expect(visibleWithImportance('building', -2, 5)).toBe(true) // class gate already open
  })
})

describe('instanceBudgetCheck', () => {
  it('sums visible-class counts against INSTANCE_BUDGET', () => {
    expect(instanceBudgetCheck([100_000, 50_000])).toBe(true)
    expect(instanceBudgetCheck([100_000, 50_001])).toBe(false)
    expect(instanceBudgetCheck([])).toBe(true)
  })
})
