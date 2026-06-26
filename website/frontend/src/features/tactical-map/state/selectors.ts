// Pure, memoized transforms: store slices -> per-layer data arrays (Ultra Plan
// §4.4). Memoize on input refs — bindings.ts replaces a dictionary object only when
// it actually changes, so ref-equality caching avoids rebuilding a layer's data
// (and thus the layer) when an unrelated slice mutates.

import type { Faction, ID, Selection, Slot, Squad } from './schema'

export interface SlotIcon {
  id: ID
  x: number
  y: number
  selected: boolean
}

function memo1<A, R>(fn: (a: A) => R): (a: A) => R {
  let lastA: A | undefined
  let lastR: R | undefined
  let primed = false
  return (a) => {
    if (primed && a === lastA) return lastR as R
    lastA = a
    lastR = fn(a)
    primed = true
    return lastR
  }
}

function memo2<A, B, R>(fn: (a: A, b: B) => R): (a: A, b: B) => R {
  let lastA: A | undefined
  let lastB: B | undefined
  let lastR: R | undefined
  let primed = false
  return (a, b) => {
    if (primed && a === lastA && b === lastB) return lastR as R
    lastA = a
    lastB = b
    lastR = fn(a, b)
    primed = true
    return lastR
  }
}

function memo3<A, B, C, R>(fn: (a: A, b: B, c: C) => R): (a: A, b: B, c: C) => R {
  let lastA: A | undefined
  let lastB: B | undefined
  let lastC: C | undefined
  let lastR: R | undefined
  let primed = false
  return (a, b, c) => {
    if (primed && a === lastA && b === lastB && c === lastC) return lastR as R
    lastA = a
    lastB = b
    lastC = c
    lastR = fn(a, b, c)
    primed = true
    return lastR
  }
}

/** O(n) slot count; memoized on slotsById ref (bindings replace the dict object per snapshot). */
export const selectSlotCount = memo1(
  (slotsById: Record<ID, Slot>): number => Object.keys(slotsById).length,
)

// Base map icons (T-061): every slot EXCEPT the ones currently being drag-previewed.
// `excludeIds` only changes at drag start/end, so this O(n) scan over ~360k slots does
// NOT re-run per frame during a move — the dragged icons render in the cheap overlay
// layer (selectDragOverlayIcons) instead. Selected ids drive the highlight.
export const selectSlotIconsBase = memo3(
  (slotsById: Record<ID, Slot>, selection: Selection, excludeIds: Set<ID> | null): SlotIcon[] => {
    const selected = selection.kind !== 'none' ? new Set(selection.ids) : null
    const out: SlotIcon[] = []
    for (const s of Object.values(slotsById)) {
      if (excludeIds?.has(s.id)) continue
      out.push({
        id: s.id,
        x: s.position.x,
        y: s.position.y,
        selected: selected?.has(s.id) ?? false,
      })
    }
    return out
  },
)

// Drag-preview overlay (T-061): only the dragged ids, offset by the live world delta.
// O(k) in the selection size — the per-frame path during a move — never O(total slots).
export const selectDragOverlayIcons = memo3(
  (
    slotsById: Record<ID, Slot>,
    ids: ID[] | null,
    delta: { dx: number; dy: number } | null,
  ): SlotIcon[] => {
    if (!ids?.length || !delta) return []
    const out: SlotIcon[] = []
    for (const id of ids) {
      const s = slotsById[id]
      if (!s) continue
      out.push({
        id: s.id,
        x: s.position.x + delta.dx,
        y: s.position.y + delta.dy,
        selected: true,
      })
    }
    return out
  },
)

// ── Outliner (Miller columns) ───────────────────────────────────────────────
// The Outliner shows one faction's squads and one squad's slots at a time, so a
// last-call memo (memo2) is sufficient even though the parent id varies.

export const selectFactionList = memo1(
  (factionsById: Record<ID, Faction>): Faction[] => Object.values(factionsById),
)

export const selectSquadsOf = memo2(
  (squadsById: Record<ID, Squad>, factionId: ID | null): Squad[] =>
    factionId
      ? Object.values(squadsById).filter((s) => s.factionId === factionId)
      : [],
)

export const selectSlotsOf = memo2(
  (slotsById: Record<ID, Slot>, squadId: ID | null): Slot[] =>
    squadId
      ? Object.values(slotsById)
          .filter((s) => s.squadId === squadId)
          .sort((a, b) => a.index - b.index)
      : [],
)
