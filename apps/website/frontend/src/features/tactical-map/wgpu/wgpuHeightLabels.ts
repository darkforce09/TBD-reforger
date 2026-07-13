// T-152.7 — thin wasm bridge for DEM height labels (peak detect + declutter in Rust).
// LANGUAGE GATE: no layout / peak math in TS.

import {
  declutter_height_labels_json,
  find_peaks_from_meters,
  pack_height_label_bytes,
} from '@/wasm/pkg/map_engine_wasm'

export type HeightLabelRow = {
  x: number
  y: number
  value_m: number
  kind: 'peak' | 'contour'
  // T-152.16: optional toponym for named peaks/hills. The label text ("{name} · {N} m") is
  // composed in Rust; TS only carries the field through declutter/pack round-trips.
  name?: string
}

/** Detect peaks from a DEM meters cache + manifest bounds. */
export function findPeaksFromMeters(
  meters: Float32Array,
  width: number,
  height: number,
  bounds: { minX: number; minY: number; maxX: number; maxY: number; flipX: boolean; flipZ: boolean },
): HeightLabelRow[] {
  const json = find_peaks_from_meters(
    meters,
    width,
    height,
    bounds.minX,
    bounds.minY,
    bounds.maxX,
    bounds.maxY,
    bounds.flipX,
    bounds.flipZ,
  )
  return JSON.parse(json) as HeightLabelRow[]
}

/** Declutter at deck zoom; returns JSON rows. */
export function declutterHeightLabels(labels: HeightLabelRow[], deckZoom: number): HeightLabelRow[] {
  const json = declutter_height_labels_json(JSON.stringify(labels), deckZoom)
  return JSON.parse(json) as HeightLabelRow[]
}

/** Pack into 20 B GPU instances for `upload_text_labels`. */
export function packHeightLabelBytes(labels: HeightLabelRow[], deckZoom: number): Uint8Array {
  return pack_height_label_bytes(JSON.stringify(labels), deckZoom)
}
