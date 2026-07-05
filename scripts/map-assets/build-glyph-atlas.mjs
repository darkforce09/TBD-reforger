#!/usr/bin/env node
// T-090.5.2 — World-glyph atlas builder (`make map-glyphs-build`).
// packages/map-assets/glyphs/manifest.json (svg sources) → atlas/world-glyphs.webp (one GPU
// texture) + atlas/world-glyphs.json (Deck-ready IconLayer mapping + meta). Deterministic:
// sorted glyph keys, fixed 128 px cells, row-major grid on a power-of-two canvas (GL-G4 caps
// the atlas at 4096²). Rasterization shells out to ImageMagick (RSVG delegate) — same system
// dependency as the cartographic pipeline; no npm deps (T-125 portability rule).
//
// Contract consumed by layers/worldGlyphAtlas.ts + verify-map-glyphs-manifest.mjs (G4):
//   { meta: { width, height, cellPx, refZoom, schemaVersion }, icons: { <iconKey>:
//     { x, y, width, height, anchorX, anchorY, mask } } }
// anchorX/anchorY are px inside the rect (manifest anchor fractions × cell); mask mirrors the
// manifest `tintable` flag (Deck tints mask icons via getColor).

import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')
const glyphDir = join(repoRoot, 'packages', 'map-assets', 'glyphs')
const atlasDir = join(glyphDir, 'atlas')

const CELL_PX = 128
const MAX_ATLAS_PX = 4096
// 24×24 design units @ 96 dpi natively — render via density so RSVG rasterizes at (beyond)
// target resolution instead of upscaling a 24 px bitmap.
const RASTER_DENSITY = 512

const fail = (msg) => {
  console.error(`build-glyph-atlas: FAIL — ${msg}`)
  process.exit(1)
}

try {
  execFileSync('magick', ['-version'], { stdio: 'pipe' })
} catch {
  fail('ImageMagick `magick` not found on PATH (needs RSVG + WebP delegates)')
}

const manifest = JSON.parse(readFileSync(join(glyphDir, 'manifest.json'), 'utf8'))
const keys = Object.keys(manifest.glyphs ?? {}).sort()
if (keys.length === 0) fail('glyph manifest has no glyphs')

const nextPow2 = (n) => 2 ** Math.ceil(Math.log2(Math.max(1, n)))
// Near-square start, then widen to the full power-of-two row so no texture space is wasted.
const width = nextPow2(Math.ceil(Math.sqrt(keys.length)) * CELL_PX)
const cols = Math.floor(width / CELL_PX)
const rows = Math.ceil(keys.length / cols)
const height = nextPow2(rows * CELL_PX)
if (width > MAX_ATLAS_PX || height > MAX_ATLAS_PX) {
  fail(`atlas ${width}×${height} exceeds ${MAX_ATLAS_PX}² (GL-G4) — shrink CELL_PX or split`)
}

const tmp = mkdtempSync(join(tmpdir(), 'tbd-glyph-atlas-'))
const icons = {}
try {
  // 1. Rasterize every SVG to a transparent CELL_PX² cell.
  for (const key of keys) {
    const g = manifest.glyphs[key]
    if (!g?.svg) fail(`glyph '${key}' has no svg path`)
    const svgPath = join(glyphDir, g.svg)
    const cellPng = join(tmp, `${key}.png`)
    try {
      execFileSync(
        'magick',
        [
          '-background', 'none',
          '-density', String(RASTER_DENSITY),
          svgPath,
          '-resize', `${CELL_PX}x${CELL_PX}`,
          '-gravity', 'center',
          '-extent', `${CELL_PX}x${CELL_PX}`,
          cellPng,
        ],
        { stdio: 'pipe' },
      )
    } catch (e) {
      fail(`rasterize '${key}' (${g.svg}): ${e.stderr?.toString().trim() || e.message}`)
    }
  }

  // 2. Composite all cells onto one canvas in a single magick invocation.
  const args = ['-size', `${width}x${height}`, 'xc:none']
  keys.forEach((key, i) => {
    const x = (i % cols) * CELL_PX
    const y = Math.floor(i / cols) * CELL_PX
    args.push(join(tmp, `${key}.png`), '-geometry', `+${x}+${y}`, '-composite')
    const g = manifest.glyphs[key]
    const anchor = Array.isArray(g.anchor) ? g.anchor : [0.5, 0.5]
    icons[key] = {
      x,
      y,
      width: CELL_PX,
      height: CELL_PX,
      anchorX: Math.round(anchor[0] * CELL_PX),
      anchorY: Math.round(anchor[1] * CELL_PX),
      mask: g.tintable === true,
    }
  })
  mkdirSync(atlasDir, { recursive: true })
  const webpPath = join(atlasDir, 'world-glyphs.webp')
  args.push('-define', 'webp:lossless=true', webpPath)
  try {
    execFileSync('magick', args, { stdio: 'pipe' })
  } catch (e) {
    fail(`composite atlas: ${e.stderr?.toString().trim() || e.message}`)
  }

  // 3. Deck-ready mapping + meta (verify gate G4 reads dims from here; webp header is
  // sanity-checked separately so the two can't drift silently).
  const mapping = {
    meta: {
      schemaVersion: manifest.schemaVersion ?? '1.0.0',
      refZoom: manifest.refZoom ?? 3,
      width,
      height,
      cellPx: CELL_PX,
    },
    icons,
  }
  writeFileSync(join(atlasDir, 'world-glyphs.json'), `${JSON.stringify(mapping, null, 2)}\n`)

  const webpBytes = readFileSync(webpPath)
  if (webpBytes.length < 12 || webpBytes.toString('ascii', 0, 4) !== 'RIFF' || webpBytes.toString('ascii', 8, 12) !== 'WEBP') {
    fail('emitted atlas is not a RIFF/WEBP file')
  }
  console.log(
    `build-glyph-atlas: OK — ${keys.length} glyphs → ${width}×${height} atlas (${(webpBytes.length / 1024).toFixed(1)} KB) @ ${atlasDir}`,
  )
} finally {
  rmSync(tmp, { recursive: true, force: true })
}
