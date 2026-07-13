// T-152.8 — thin wasm bridge for town labels (A3 declutter + pack in Rust).
// LANGUAGE GATE: no layout / declutter math in TS.

import {
  declutter_town_labels_json,
  pack_town_label_bytes,
  parse_locations_json_wasm,
} from '@/wasm/pkg/map_engine_wasm'

export type TownLabelRow = {
  id: string
  name: string
  x: number
  y: number
  importance: number
  kind?: string
}

/** Parse `locations.json` text into rows. */
export function parseLocationsJson(json: string): TownLabelRow[] {
  const out = parse_locations_json_wasm(json)
  return JSON.parse(out) as TownLabelRow[]
}

/** Declutter at deck zoom; returns JSON rows. */
export function declutterTownLabels(rows: TownLabelRow[], deckZoom: number): TownLabelRow[] {
  const json = declutter_town_labels_json(JSON.stringify(rows), deckZoom)
  return JSON.parse(json) as TownLabelRow[]
}

/** Pack into 20 B GPU instances for `upload_town_labels`. */
export function packTownLabelBytes(rows: TownLabelRow[], deckZoom: number): Uint8Array {
  return pack_town_label_bytes(JSON.stringify(rows), deckZoom)
}
