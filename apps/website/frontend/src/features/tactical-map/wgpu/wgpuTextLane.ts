// T-152.1 — thin wasm bridge for cartographic text labels (declutter lives in Rust).
// LANGUAGE GATE: no layout / declutter / zoom scaling policy here.

import { TextLabelStore } from '@/wasm/pkg/map_engine_wasm'

export type MapLabelInput = {
  id: number
  x: number
  y: number
  importance: number
  text: string
}

/** Create an empty label store (call `.free()` on unmount). */
export function createTextLabelStore(): TextLabelStore {
  return new TextLabelStore()
}

/** Push labels through Rust declutter at `deckZoom`. */
export function setMapLabels(
  store: TextLabelStore,
  labels: MapLabelInput[],
  deckZoom: number,
): void {
  store.set_labels_json(JSON.stringify(labels), deckZoom)
}

/** Surviving label count after declutter (0 when empty). */
export function textLabelCount(store: TextLabelStore): number {
  return store.text_label_count()
}

export type { TextLabelStore }
