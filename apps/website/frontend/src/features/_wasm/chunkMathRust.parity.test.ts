// T-151.3 W3 (P1, Class R) — the Rust chunk-id math must equal the JS `chunkIdsForViewport` for
// every viewport. Both are deterministic IEEE-754 f64 (`0.05*span`, floor/ceil/clamp) producing
// exact `"cx_cy"` strings, so equality is byte-exact ordered-array equality (not just a set).

import { describe, expect, it } from 'vitest'

import { chunkIdsForViewport, type Bbox } from '@/features/tactical-map/worldmap/chunkMath'
import { TERRAINS } from '@/features/tactical-map/coords/terrains'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

const EV = TERRAINS.everon
const { width: W, height: H } = EV
const CS = 512

function rustIds(bbox: Bbox, extraRing: number): string[] {
  return Array.from(
    wasm.world_chunk_ids_for_viewport(bbox[0], bbox[1], bbox[2], bbox[3], W, H, CS, extraRing),
  )
}

// Battery: interior, cell edges, past every edge, tiny, negative/SW straddle, full extent, a
// deterministic pan/zoom-shaped spread of off-grid float bboxes.
const BBOXES: Bbox[] = [
  [0, 0, 511.9, 511.9],
  [512, 512, 1024, 1024],
  [1024, 1024, 1536, 1536],
  [2000, 2000, 2200, 2200],
  [12799, 12799, 99999, 99999], // past the SE edge → clamps to the last cell
  [-500, -500, 100, 100], // straddles the SW corner (negative → clamps to 0)
  [5000, 5000, 5001, 5001], // tiny
  [0, 0, 12800, 12800], // full extent
  [6371.5, 4098.2, 6372.5, 4099.2], // sub-meter, off-grid
  [3333.33, 9999.99, 7777.77, 11111.11],
  [0, 6000, 12800, 6400], // wide, thin band
  [11000, 200, 12790, 900],
]

describe('chunkMathRust.parity — Rust chunk_ids_for_viewport == JS chunkIdsForViewport (Class R)', () => {
  for (const bbox of BBOXES) {
    for (const extraRing of [0, 1]) {
      it(`[${bbox.join(',')}] ring=${extraRing}`, () => {
        const js = chunkIdsForViewport(bbox, EV, { chunkSizeM: CS, extraRing })
        expect(rustIds(bbox, extraRing)).toEqual(js)
      })
    }
  }
})
