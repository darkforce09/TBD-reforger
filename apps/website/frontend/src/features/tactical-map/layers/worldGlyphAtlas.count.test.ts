// Guard D (glyph-atlas fix) — the world glyph atlas must fit the engine's icon UV-table capacity.
// The wgpu loader (`wgpuWorldLoader.loadGlyphAtlas`) reads `atlas_glyph_count()` (the Rust
// `scene::ATLAS_GLYPH_COUNT`, wasm-exported) instead of a hardcoded literal; if the atlas ever holds
// more keys than the engine can upload, glyphs silently go dark (the T-152.10 `29 ≠ 28` regression).
// This asserts the invariant on the real atlas + proves the wasm count is reachable from TS.
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import * as wasm from '@/wasm/pkg/map_engine_wasm'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')

describe('world glyph atlas — count fits engine capacity', () => {
  it('atlas key count is within atlas_glyph_count() (Rust single source of truth)', () => {
    const atlas = JSON.parse(
      readFileSync(`${MAP_ASSETS}/glyphs/atlas/world-glyphs.json`, 'utf8'),
    ) as { icons: Record<string, unknown> }
    const keyCount = Object.keys(atlas.icons).length
    const capacity = wasm.atlas_glyph_count()

    expect(keyCount).toBeGreaterThan(0)
    expect(capacity).toBeGreaterThanOrEqual(keyCount)
  })
})
