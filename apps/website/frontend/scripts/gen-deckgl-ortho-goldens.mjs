// Golden-fixture generator for the T-151 OrthoCamera parity corpus.
//
// Oracle = the installed @deck.gl/core, driven through the exact code path the app uses
// (`new OrthographicView({ flipY: false }).makeViewport({ width, height, viewState })` —
// see features/tactical-map/view/useOrthographicView.ts + TacticalMap.tsx). The battery is
// fully enumerated (no randomness); rerunning must produce a byte-identical file (sha256
// determinism gate). Output: crates/map-engine-core/tests/fixtures/deckgl_ortho_goldens.json.
//
// Numbers are serialized with JS shortest-round-trip formatting (JSON.stringify), which
// serde_json parses back to the identical f64 — lossless transport. (−0 serializes as "0";
// the Rust ULP metric treats ±0 as distance 0, so this cannot mask a real mismatch.)
//
// Run: npm run gen:ortho-goldens   (cwd: apps/website/frontend)

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { OrthographicView } from '@deck.gl/core'

// package.json is not in @deck.gl/core's export map — read it off disk.
const deckPkg = JSON.parse(
  readFileSync(
    fileURLToPath(new URL('../node_modules/@deck.gl/core/package.json', import.meta.url)),
    'utf8',
  ),
)

// ---------------------------------------------------------------------------------------
// The enumerated battery (plan §S3a). All inputs are exact f64 literals.
// ---------------------------------------------------------------------------------------

const ZOOMS = [-6, -4, -2, 0, 2, 6, -5.5, -2.25, 0.5, 3.75]
// 1237.33×842.67 is a deliberate fractional CSS size — the live app builds viewports from
// getBoundingClientRect(), which returns fractional dimensions (TacticalMap.tsx:210).
const SIZES = [
  [1, 1],
  [800, 600],
  [1366, 768],
  [1920, 1080],
  [2560, 1440],
  [1237.33, 842.67],
]
// (-500, 13300) is deliberately out of terrain bounds — camera math is clamp-free; clamps
// live in the view-state layer (useOrthographicView.onViewStateChange).
const TARGETS = [
  [0, 0],
  [6400, 6400],
  [12800, 12800],
  [123.456, 9876.543],
  [-500, 13300],
]

/** World probe points per case: terrain corners, terrain center, target, target+offset. */
function probePoints(target) {
  return [
    [0, 0, 0],
    [12800, 12800, 0],
    [6400, 6400, 0],
    [target[0], target[1], 0],
    [target[0] + 137.037, target[1] + -73.31, 0],
  ]
}

/** Pixel probe points per case (uses the ||1-coerced viewport dims, like deck internally). */
function pixelPoints(width, height) {
  const w = width || 1
  const h = height || 1
  return [
    [0, 0],
    [w / 2, h / 2],
    [w, h],
    [0.25 * w, 0.75 * h],
  ]
}

// ---------------------------------------------------------------------------------------

const view = new OrthographicView({ flipY: false })
const cases = []

for (const zoom of ZOOMS) {
  for (const [width, height] of SIZES) {
    for (const target of TARGETS) {
      const viewport = view.makeViewport({
        width,
        height,
        viewState: { target, zoom },
      })

      const c = {
        id: `z${zoom}_w${width}x${height}_t${target[0]}_${target[1]}`,
        width,
        height,
        zoom,
        target,
        scale: Math.pow(2, zoom),
        bounds: viewport.getBounds(),
        probes: probePoints(target).map((world) => {
          const projected = viewport.project(world)
          const roundTrip = viewport.unproject([projected[0], projected[1]])
          return { world, project: projected, roundTrip }
        }),
        unprojects: pixelPoints(width, height).map((pixel) => ({
          pixel,
          world: viewport.unproject(pixel),
        })),
      }

      // Full matrices only for the 800×600 subset (bounds fixture size — plan §S3a).
      if (width === 800 && height === 600) {
        c.viewMatrix = Array.from(viewport.viewMatrix)
        c.projectionMatrix = Array.from(viewport.projectionMatrix)
        c.viewProjectionMatrix = Array.from(viewport.viewProjectionMatrix)
        c.pixelProjectionMatrix = Array.from(viewport.pixelProjectionMatrix)
        c.pixelUnprojectionMatrix = Array.from(viewport.pixelUnprojectionMatrix)
      }

      cases.push(c)
    }
  }
}

const fixture = {
  meta: {
    generator: 'apps/website/frontend/scripts/gen-deckgl-ortho-goldens.mjs',
    deckglVersion: deckPkg.version,
    nodeVersion: process.version,
    flipY: false,
    near: 0.1,
    far: 1000,
    caseCount: cases.length,
    matrixSubset: '800x600',
  },
  cases,
}

// Pretty JSON, but collapse leaf arrays (numbers only) onto one line to bound file size.
const pretty = JSON.stringify(fixture, null, 2).replace(
  /\[[\s\d.,eE+-]+\]/g,
  (m) => `[${m.slice(1, -1).replace(/\s+/g, ' ').trim()}]`,
)

const outDir = fileURLToPath(
  new URL('../../../../crates/map-engine-core/tests/fixtures/', import.meta.url),
)
mkdirSync(outDir, { recursive: true })
const outPath = `${outDir}deckgl_ortho_goldens.json`
writeFileSync(outPath, `${pretty}\n`)

console.warn(`wrote ${cases.length} cases → ${outPath}`)
