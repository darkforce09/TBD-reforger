// T-090.8.1 — Forest layer-builder gates, incl. the slice's F3/LOD3 acceptance: at the
// default deckZoom −2 forests are POLYGONS (world-forest fill live, α 0.35) while trees are
// hidden and buildings stay OBB rects — and no tree layer exists in this module at all.
import { describe, it, expect } from 'vitest'
import { buildForestLayers, forestFillColor, FOREST_OUTLINE_RGBA } from './forestMassLayer'
import { EMPTY_FOREST_COMPOSITE, type ForestMassComposite } from './forestMassStore'
import { forestFillAlpha } from './forestMass'
import { classVisible } from './lodGates'

/** Two triangles + two contour segments — enough to exercise both layers' binary form. */
const MASS: ForestMassComposite = {
  fillPositions: Float32Array.from([
    0, 0, 24, 0, 0, 24, 0, 0, // ring 0 (closed)
    512, 512, 536, 512, 512, 536, 512, 512, // ring 1 (closed)
  ]),
  fillStartIndices: Uint32Array.from([0, 4]),
  outlineSegments: Float32Array.from([24, 0, 0, 24, 536, 512, 512, 536]),
  polygonCount: 2,
  segmentCount: 2,
  chunkCount: 2,
}

function build(deckZoom: number, mass: ForestMassComposite = MASS) {
  return buildForestLayers({
    mass,
    fillAlpha: forestFillAlpha(deckZoom),
    fillVisible: classVisible('forestFill', deckZoom),
    outlineVisible: classVisible('forestOutline', deckZoom),
  })
}

/** Untyped props view — the builder returns a Solid/Line union and these assertions probe
 *  per-layer accessors across it. */
const propsOf = (layer: { props: object } | undefined): Record<string, unknown> =>
  (layer?.props ?? {}) as Record<string, unknown>

describe('F3 / LOD3 — default zoom −2', () => {
  it('forest renders as filled polygons; trees hidden; buildings unaffected (gates)', () => {
    const layers = build(-2)
    const fill = layers.find((l) => l.id === 'world-forest')
    expect(fill?.props.visible).toBe(true)
    expect(propsOf(fill).getFillColor).toEqual([34, 120, 60, Math.round(255 * 0.35)])
    expect(classVisible('tree', -2)).toBe(false) // no per-tree icons inside the polygons
    expect(classVisible('building', -2)).toBe(true) // OBB rects unchanged (LOD3)
    // Outline opens at −1.5 — constructed but not visible at −2.
    expect(layers.find((l) => l.id === 'world-forest-outline')?.props.visible).toBe(false)
  })

  it('no tree glyph layer exists in the forest module (T-090.5.5 boundary)', () => {
    expect(build(-2).map((l) => l.id).sort()).toEqual(['world-forest', 'world-forest-outline'])
  })
})

describe('α ladder + gate bands', () => {
  it('island zoom −6: fill α 0.45, outline hidden', () => {
    const layers = build(-6)
    expect(propsOf(layers.find((l) => l.id === 'world-forest')).getFillColor).toEqual([
      34, 120, 60, Math.round(255 * 0.45),
    ])
    expect(layers.find((l) => l.id === 'world-forest-outline')?.props.visible).toBe(false)
  })

  it('outline visible from −1.5 and stays on above the fill max', () => {
    expect(build(-1.5).find((l) => l.id === 'world-forest-outline')?.props.visible).toBe(true)
    expect(build(4).find((l) => l.id === 'world-forest-outline')?.props.visible).toBe(true)
  })

  it('fill hides above FOREST_FILL_MAX_ZOOM (+1) — shipped gate wins over the 0.12 band', () => {
    expect(build(1).find((l) => l.id === 'world-forest')?.props.visible).toBe(true)
    expect(build(1.5).find((l) => l.id === 'world-forest')?.props.visible).toBe(false)
    // The N3 0.12 fade value stays encoded (latent) in the color fn.
    expect(forestFillColor(1.5)[3]).toBe(Math.round(255 * 0.12))
    expect(forestFillColor(3.5)[3]).toBe(0)
  })
})

describe('binary plumbing + mass-layer rules', () => {
  it('fill is the store composite in SolidPolygonLayer binary form (_normalize off)', () => {
    const fill = build(-2).find((l) => l.id === 'world-forest')
    const data = fill?.props.data as {
      length: number
      startIndices: Uint32Array
      attributes: { getPolygon: { value: Float32Array; size: number } }
    }
    expect(data.length).toBe(2)
    expect(data.startIndices).toBe(MASS.fillStartIndices)
    expect(data.attributes.getPolygon.value).toBe(MASS.fillPositions)
    expect((fill?.props as { _normalize?: boolean })._normalize).toBe(false)
  })

  it('outline views the one segment buffer via stride/offset (no repack)', () => {
    const outline = build(-1)?.find((l) => l.id === 'world-forest-outline')
    const data = outline?.props.data as {
      length: number
      attributes: {
        getSourcePosition: { value: Float32Array; stride: number; offset: number }
        getTargetPosition: { value: Float32Array; stride: number; offset: number }
      }
    }
    expect(data.length).toBe(2)
    expect(data.attributes.getSourcePosition.value).toBe(MASS.outlineSegments)
    expect(data.attributes.getTargetPosition.value).toBe(MASS.outlineSegments)
    expect(data.attributes.getSourcePosition.offset).toBe(0)
    expect(data.attributes.getTargetPosition.offset).toBe(8)
    expect(propsOf(outline).getColor).toEqual(FOREST_OUTLINE_RGBA)
  })

  it('never pickable; empty composite builds no layers', () => {
    for (const layer of build(-2)) expect(layer.props.pickable).toBe(false)
    expect(build(-2, EMPTY_FOREST_COMPOSITE)).toEqual([])
  })
})
