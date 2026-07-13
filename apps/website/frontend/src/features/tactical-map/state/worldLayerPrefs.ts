// T-090.5.1 — World-layer preferences: the 3-way mapStyle + per-class layer toggles
// (implementation plan §4.3, N8). Per-user + global (localStorage `tbd-mc-world-layers`),
// NOT mission meta — like basemapView before it, the pref travels with the user. Same
// useSyncExternalStore module-singleton pattern as basemapView.ts (which is now a shim
// over this module until T-090.10.2 deletes it).
//
// Migration: mapStyle seeds from the legacy `tbd-mc-basemap-view` key when this store has
// no value yet ('satellite'→'satellite', 'map'→'map'; 'hybrid' is new). setMapStyle
// dual-writes the legacy key (hybrid maps to 'satellite') so a rollback to a pre-5.1
// build keeps the user's raster choice.

import { useSyncExternalStore } from 'react'
import { basemapViewForStyle, type MapStyle } from '../worldmap/styleModes'

const KEY = 'tbd-mc-world-layers'
const LEGACY_BASEMAP_KEY = 'tbd-mc-basemap-view'
const DEFAULT_STYLE: MapStyle = 'satellite'

/** Per-class world layer visibility (t090_5 toggles table; props default off = noise). */
export interface WorldClassToggles {
  roads: boolean
  buildings: boolean
  forest: boolean
  trees: boolean
  props: boolean
  /** T-152.4 cartographic fence/railing strips (default on). */
  fences: boolean
  /** T-152.5 airfield apron + runway polish + hangar/tower icons (default on). */
  airfield: boolean
  contours: boolean
  sea: boolean
  /** T-152.7 DEM peak ASL height labels (default on). */
  heights: boolean
  /** T-152.8 town name labels from locations.json (default on). */
  townLabels: boolean
}

const DEFAULT_TOGGLES: WorldClassToggles = {
  roads: true,
  buildings: true,
  forest: true,
  trees: true,
  props: false,
  fences: true,
  airfield: true,
  contours: true,
  sea: true,
  heights: true,
  townLabels: true,
}

interface WorldLayerPrefs {
  mapStyle: MapStyle
  classToggles: WorldClassToggles
  /** DEV world-object debug HUD (verbose stream counters in the FPS badge). Off by default;
   *  toggled with Ctrl+Alt+D. Persisted so it survives reloads while debugging. */
  worldmapDebug: boolean
}

const isMapStyle = (v: unknown): v is MapStyle => v === 'satellite' || v === 'hybrid' || v === 'map'

/** One-time module-load read: this store's key wins; else migrate the legacy basemap key. */
function read(): WorldLayerPrefs {
  const prefs: WorldLayerPrefs = {
    mapStyle: DEFAULT_STYLE,
    classToggles: { ...DEFAULT_TOGGLES },
    worldmapDebug: false,
  }
  try {
    const raw = localStorage.getItem(KEY)
    const parsed: unknown = raw ? JSON.parse(raw) : null
    if (parsed && typeof parsed === 'object') {
      const p = parsed as { mapStyle?: unknown; classToggles?: unknown; worldmapDebug?: unknown }
      if (isMapStyle(p.mapStyle)) prefs.mapStyle = p.mapStyle
      if (p.classToggles && typeof p.classToggles === 'object') {
        const stored = p.classToggles as Record<string, unknown>
        for (const k of Object.keys(DEFAULT_TOGGLES) as (keyof WorldClassToggles)[]) {
          if (typeof stored[k] === 'boolean') prefs.classToggles[k] = stored[k]
        }
      }
      if (typeof p.worldmapDebug === 'boolean') prefs.worldmapDebug = p.worldmapDebug
      if (isMapStyle(p.mapStyle)) return prefs
    }
    // No (valid) mapStyle stored yet → seed from the legacy Satellite|Map pref.
    const legacy = localStorage.getItem(LEGACY_BASEMAP_KEY)
    if (legacy === 'map' || legacy === 'satellite') prefs.mapStyle = legacy
  } catch {
    /* private mode / quota / garbage JSON — defaults */
  }
  return prefs
}

let current: WorldLayerPrefs = read()
const listeners = new Set<() => void>()

function persist(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(current))
    // Dual-write the legacy key until T-090.10.2 retires the shim (rollback safety).
    localStorage.setItem(LEGACY_BASEMAP_KEY, basemapViewForStyle(current.mapStyle))
  } catch {
    /* private mode / quota — keep the in-memory value */
  }
}

const notify = () => listeners.forEach((l) => l())

export function getMapStyle(): MapStyle {
  return current.mapStyle
}

export function setMapStyle(style: MapStyle): void {
  if (style === current.mapStyle) return
  current = { ...current, mapStyle: style }
  persist()
  notify()
}

export function getClassToggles(): WorldClassToggles {
  return current.classToggles
}

export function setClassToggle(cls: keyof WorldClassToggles, on: boolean): void {
  if (current.classToggles[cls] === on) return
  current = { ...current, classToggles: { ...current.classToggles, [cls]: on } }
  persist()
  notify()
}

/** Non-React subscription (the basemapView shim + future layer builders). */
export function subscribeWorldLayerPrefs(cb: () => void): () => void {
  listeners.add(cb)
  return () => listeners.delete(cb)
}

/** React hook: current map style, re-rendering on change. */
export function useMapStyle(): MapStyle {
  return useSyncExternalStore(subscribeWorldLayerPrefs, getMapStyle, getMapStyle)
}

/** React hook: current class toggles (stable reference between changes). */
export function useClassToggles(): WorldClassToggles {
  return useSyncExternalStore(subscribeWorldLayerPrefs, getClassToggles, getClassToggles)
}

export function getWorldmapDebug(): boolean {
  return current.worldmapDebug
}

/** Flip the DEV world-object debug HUD (bound to Ctrl+Alt+D in the FPS badge). */
export function toggleWorldmapDebug(): void {
  current = { ...current, worldmapDebug: !current.worldmapDebug }
  persist()
  notify()
}

/** React hook: whether the verbose world-object debug HUD is on. */
export function useWorldmapDebug(): boolean {
  return useSyncExternalStore(subscribeWorldLayerPrefs, getWorldmapDebug, getWorldmapDebug)
}
