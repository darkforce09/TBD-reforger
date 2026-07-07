import { describe, it, expect } from 'vitest'
import { OrthographicView } from '@deck.gl/core'
import { OrthoCameraJs } from '@/wasm/pkg/map_engine_wasm'
import { ulpDistanceF64 } from './parity'

// T-151 live-oracle camera parity: the wasm OrthoCamera vs the in-process deck.gl 9.3.5
// viewport, over the same enumerated battery as the committed Rust golden fixture
// (scripts/gen-deckgl-ortho-goldens.mjs). Two oracles: the Rust tests pin against the frozen
// fixture; this suite tracks whatever deck.gl + V8 the runtime actually ships, so a silent
// dependency drift cannot rot the contract.
//
// ULP classes (mirroring the Rust T1/T2/T4 gates — T3 scale-injection lives Rust-side):
//   - integer zooms: ULP == 0 on everything (2^int is exact in both pow and exp2).
//   - fractional zooms: scale ≤ 1 ULP (pow vs exp2), matrices ≤ 2, projections/bounds ≤ 4.

const ZOOMS = [-6, -4, -2, 0, 2, 6, -5.5, -2.25, 0.5, 3.75]
const SIZES: Array<[number, number]> = [
  [1, 1],
  [800, 600],
  [1366, 768],
  [1920, 1080],
  [2560, 1440],
  [1237.33, 842.67],
]
const TARGETS: Array<[number, number]> = [
  [0, 0],
  [6400, 6400],
  [12800, 12800],
  [123.456, 9876.543],
  [-500, 13300],
]

const view = new OrthographicView({ flipY: false })

function assertUlp(got: ArrayLike<number>, expected: ArrayLike<number>, max: number, ctx: string) {
  expect(got.length, `${ctx} length`).toBe(expected.length)
  for (let i = 0; i < got.length; i++) {
    const d = ulpDistanceF64(got[i], expected[i])
    if (d > max) {
      expect.fail(`${ctx}[${i}] ULP ${d} > ${max} (got ${got[i]}, expected ${expected[i]})`)
    }
  }
}

interface Budgets {
  matrix: number
  projection: number
}

function checkCase(width: number, height: number, target: [number, number], zoom: number) {
  const viewport = view.makeViewport({ width, height, viewState: { target, zoom } })
  if (viewport === null) throw new Error(`makeViewport null at ${width}x${height}`)
  const cam = new OrthoCameraJs(width, height, target[0], target[1], zoom)
  const integer = Number.isInteger(zoom)
  const b: Budgets = integer ? { matrix: 0, projection: 0 } : { matrix: 2, projection: 4 }
  const id = `z${zoom}_w${width}x${height}_t${target[0]}_${target[1]}`

  try {
    // T2 analog: the sole non-mirrored op, measured alone.
    expect(ulpDistanceF64(cam.scale(), Math.pow(2, zoom))).toBeLessThanOrEqual(integer ? 0 : 1)

    // Matrices — live oracle is cheap, so all 300 cases get the full set.
    assertUlp(cam.view_matrix(), Array.from(viewport.viewMatrix), b.matrix, 'viewMatrix')
    assertUlp(
      cam.projection_matrix(),
      Array.from(viewport.projectionMatrix),
      b.matrix,
      'projectionMatrix',
    )
    assertUlp(
      cam.view_projection(),
      Array.from(viewport.viewProjectionMatrix),
      b.matrix,
      'viewProjectionMatrix',
    )
    assertUlp(
      cam.pixel_projection(),
      Array.from(viewport.pixelProjectionMatrix),
      b.matrix,
      'pixelProjectionMatrix',
    )
    assertUlp(
      cam.pixel_unprojection(),
      Array.from(viewport.pixelUnprojectionMatrix),
      b.matrix,
      'pixelUnprojectionMatrix',
    )

    // World→pixel probes (terrain corners, center, target, target+offset) + round trips.
    const probes: Array<[number, number, number]> = [
      [0, 0, 0],
      [12800, 12800, 0],
      [6400, 6400, 0],
      [target[0], target[1], 0],
      [target[0] + 137.037, target[1] + -73.31, 0],
    ]
    for (const world of probes) {
      const expectedPx = viewport.project(world)
      const gotPx = cam.project(world[0], world[1], world[2])
      assertUlp(gotPx, expectedPx, b.projection, `project(${world})`)
      const expectedRt = viewport.unproject([expectedPx[0], expectedPx[1]])
      const gotRt = cam.unproject_xy(gotPx[0], gotPx[1])
      assertUlp(gotRt, expectedRt, b.projection, `roundTrip(${world})`)
    }

    // Pixel→world unprojects over the coerced viewport dims (deck rounds + ||1 internally).
    const w = viewport.width
    const h = viewport.height
    const pixels: Array<[number, number]> = [
      [0, 0],
      [w / 2, h / 2],
      [w, h],
      [0.25 * w, 0.75 * h],
    ]
    for (const pixel of pixels) {
      assertUlp(
        cam.unproject_xy(pixel[0], pixel[1]),
        viewport.unproject(pixel),
        b.projection,
        `unproject(${pixel})`,
      )
    }

    // getBounds mirror — the culling/tile-selection primitive.
    assertUlp(cam.visible_world_rect(), viewport.getBounds(), b.projection, 'bounds')
  } catch (err) {
    throw new Error(`case ${id}: ${err instanceof Error ? err.message : String(err)}`, {
      cause: err,
    })
  } finally {
    cam.free()
  }
}

describe('map-engine-wasm OrthoCameraJs — live deck.gl oracle (T-151)', () => {
  it('matches deck.gl over the full 300-case battery (ULP 0 integer / ≤2/≤4 fractional)', () => {
    let count = 0
    for (const zoom of ZOOMS) {
      for (const [width, height] of SIZES) {
        for (const target of TARGETS) {
          checkCase(width, height, target, zoom)
          count++
        }
      }
    }
    expect(count).toBe(300)
  })

  it('pan/zoom_at interaction invariants hold through the wasm boundary', () => {
    const cam = new OrthoCameraJs(800, 600, 6400, 6400, -2)
    try {
      // Pan: a projected point shifts by exactly the screen delta (1e-9 abs).
      const before = cam.project(6000, 7000, 0)
      cam.pan(37.5, -12.25)
      const after = cam.project(6000, 7000, 0)
      expect(Math.abs(after[0] - (before[0] + 37.5))).toBeLessThanOrEqual(1e-9)
      expect(Math.abs(after[1] - (before[1] + -12.25))).toBeLessThanOrEqual(1e-9)

      // zoom_at: the world point under the cursor is fixed (1e-9 abs).
      const cursor: [number, number] = [123, 456]
      const worldBefore = cam.unproject_xy(cursor[0], cursor[1])
      cam.zoom_at(1.75, cursor[0], cursor[1])
      const worldAfter = cam.unproject_xy(cursor[0], cursor[1])
      expect(Math.abs(worldAfter[0] - worldBefore[0])).toBeLessThanOrEqual(1e-9)
      expect(Math.abs(worldAfter[1] - worldBefore[1])).toBeLessThanOrEqual(1e-9)
      expect(cam.zoom).toBe(-0.25)
    } finally {
      cam.free()
    }
  })
})
