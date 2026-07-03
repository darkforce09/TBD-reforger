// T-090.1 — Basemap view preference (Satellite | Map), per-user + global (not per-mission).
// Persisted to localStorage `tbd-mc-basemap-view` (dual-view N8). Both views render since
// T-090.1.1 (the Map cartographic pyramid). Backed by useSyncExternalStore so the map
// re-renders on a switch and non-React code (layer builders) can read the current value
// synchronously.

import { useSyncExternalStore } from 'react'

export type BasemapView = 'satellite' | 'map'

const KEY = 'tbd-mc-basemap-view'
const DEFAULT: BasemapView = 'satellite'

function read(): BasemapView {
  try {
    const v = localStorage.getItem(KEY)
    return v === 'map' || v === 'satellite' ? v : DEFAULT
  } catch {
    return DEFAULT
  }
}

let current: BasemapView = read()
const listeners = new Set<() => void>()

export function getBasemapView(): BasemapView {
  return current
}

export function setBasemapView(v: BasemapView): void {
  if (v === current) return
  current = v
  try {
    localStorage.setItem(KEY, v)
  } catch {
    /* private mode / quota — keep the in-memory value */
  }
  listeners.forEach((l) => l())
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb)
  return () => listeners.delete(cb)
}

/** React hook: current basemap view, re-rendering on change. */
export function useBasemapView(): BasemapView {
  return useSyncExternalStore(subscribe, getBasemapView, getBasemapView)
}
