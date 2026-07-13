// T-152.9 — thin wasm bridge for road labels (placement + declutter in Rust).
// LANGUAGE GATE: no polyline / declutter math in TS.

import {
  build_road_labels_json,
  pack_road_label_bytes,
  parse_road_names_json_wasm,
} from '@/wasm/pkg/map_engine_wasm'

export type RoadLabelRow = {
  name: string
  x: number
  y: number
  angle_deg: number
  priority: number
  segmentId: string
  roadClass: string
  arcFrac: number
}

/** Parse `road-names.json` text. */
export function parseRoadNamesJson(json: string): { roads: Array<{ id: string; name: string; segmentIds: string[] }> } {
  const out = parse_road_names_json_wasm(json)
  return JSON.parse(out) as { roads: Array<{ id: string; name: string; segmentIds: string[] }> }
}

/** Build draw set at deck zoom from road-names + roads payload JSON strings. */
export function buildRoadLabels(
  roadNamesJson: string,
  roadsJson: string,
  deckZoom: number,
): RoadLabelRow[] {
  const json = build_road_labels_json(roadNamesJson, roadsJson, deckZoom)
  return JSON.parse(json) as RoadLabelRow[]
}

/** Pack into 20 B GPU instances for `upload_road_labels`. */
export function packRoadLabelBytes(rows: RoadLabelRow[], deckZoom: number): Uint8Array {
  return pack_road_label_bytes(JSON.stringify(rows), deckZoom)
}
