// Mission size measurement (T-060.1.3). The compiled version payload duplicates slot data
// across `orbat[].slots` and `editor.slots`, so a ~360k mission is hundreds of MB. We surface
// the size before/during save (toolbelt SZ, Save dialog, pre-upload gate) so a failed upload is
// diagnosable from measured bytes instead of a bare ERR_NETWORK. Estimation is sampled, not a
// full compile, so it's cheap enough to run on slot-count change.

import type { MapSnapshot, MissionDoc } from '@/features/tactical-map'

/** Server-side mission-version body cap (matches the Go route's BodyLimit / config default,
 *  256 MiB). Used by the pre-upload gate so we reject before POSTing an over-cap body. */
export const SERVER_VERSION_BODY_LIMIT = 256 << 20

/** macOS Finder-style decimal byte formatting (1 MB = 1,000,000 B): "187.4 MB", "1.2 GB". */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(Math.floor(Math.log10(n) / 3), units.length - 1)
  const scaled = n / 1000 ** i
  // Bytes/KB show as whole-ish; MB+ keep one decimal like Finder.
  const decimals = i === 0 ? 0 : scaled >= 100 || i <= 1 ? 0 : 1
  return `${scaled.toFixed(decimals)} ${units[i]}`
}

const utf8Bytes = (s: string): number => new TextEncoder().encode(s).length

/** Fast, sampled estimate of the compiled version payload size (bytes). Samples up to 20 slots,
 *  averages their JSON byte length, and extrapolates over the slot count, plus a small envelope
 *  constant. Save Version omits the top-level orbat (T-062.1.1), so the body is just editor.slots
 *  + envelope — no duplication factor. No full compile. */
export function estimateCompiledBytes(state: MapSnapshot): number {
  const ids = Object.keys(state.slotsById)
  const n = ids.length
  const ENVELOPE = 2048 // map/environment/loadouts/factions/squads/layers + JSON scaffolding
  if (n === 0) return ENVELOPE
  const sampleCount = Math.min(20, n)
  let sampleBytes = 0
  for (let i = 0; i < sampleCount; i++) {
    sampleBytes += utf8Bytes(JSON.stringify(state.slotsById[ids[i]]))
  }
  const avgSlotBytes = sampleBytes / sampleCount
  return Math.round(avgSlotBytes * n + ENVELOPE)
}

/** Exact local doc footprint in bytes (the yrs update-stream / IndexedDB size, not the compiled
 *  upload). */
export function getLocalDocBytes(md: MissionDoc): number {
  return md.encodeState().byteLength
}
