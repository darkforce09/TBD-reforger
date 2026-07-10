// Registry compat graph index (T-068.9) — pure, worker-safe module (no DOM/React).
// Builds O(1)-query adjacency maps over the T-150 edge graph so canEquip/canAttach
// never scan the full edge list per call. Edge types are handled as PLAIN STRINGS
// throughout: a new edge family shipped by a future export flows through
// buildIndex/hasEdge/itemsFor/hostsFor with zero changes here — only the
// convenience family unions below name concrete types.

import type { RegistryCompatEdgeType } from '@/types/models/registry'

/**
 * Minimal edge tuple the index consumes (what the worker caches in IDB —
 * API row ids/timestamps are stripped). `edge_type` is a string on purpose:
 * unknown future families must survive a round-trip.
 *
 * @contract registry-compat.schema.json#/$defs/edge
 */
export interface CompatEdgeTuple {
  from_node: string
  to_node: string
  edge_type: string
  evidence?: string
}

/** Adjacency index: node -> edge_type -> peer set, both directions. */
export interface CompatIndex {
  /** from_node -> edge_type -> set of to_node (hosts this item goes in/on). */
  byFrom: Map<string, Map<string, Set<string>>>
  /** to_node -> edge_type -> set of from_node (items this host accepts). */
  byTo: Map<string, Map<string, Set<string>>>
  total: number
  byType: Record<string, number>
}

/**
 * Edge families where `from_node` physically attaches to / loads into the host
 * (`canAttach`). The equip family is separate — see EQUIP_EDGE_TYPES.
 */
export const ATTACH_EDGE_TYPES: readonly RegistryCompatEdgeType[] = [
  'mag_in_weapon',
  'ammo_in_mag',
  'optic_on_weapon',
  'attachment_on_weapon',
  'mag_in_vehicle_weapon',
  'ammo_in_vehicle_weapon',
]

/**
 * Edge families where `from_node` is worn/carried by a character (`canEquip`).
 * T-150 derives these from default-loadout slots (LoadoutSlotInfo evidence), so
 * a true answer means "this gear appears in that character's default loadout".
 */
export const EQUIP_EDGE_TYPES: readonly RegistryCompatEdgeType[] = ['character_default_loadout']

function addEdge(
  side: Map<string, Map<string, Set<string>>>,
  key: string,
  type: string,
  peer: string,
): void {
  let byType = side.get(key)
  if (!byType) {
    byType = new Map()
    side.set(key, byType)
  }
  let peers = byType.get(type)
  if (!peers) {
    peers = new Set()
    byType.set(type, peers)
  }
  peers.add(peer)
}

/** Build the adjacency index in one O(E) pass. Duplicate tuples collapse (sets). */
export function buildIndex(edges: readonly CompatEdgeTuple[]): CompatIndex {
  const ix: CompatIndex = { byFrom: new Map(), byTo: new Map(), total: 0, byType: {} }
  for (const e of edges) {
    addEdge(ix.byFrom, e.from_node, e.edge_type, e.to_node)
    addEdge(ix.byTo, e.to_node, e.edge_type, e.from_node)
    ix.byType[e.edge_type] = (ix.byType[e.edge_type] ?? 0) + 1
    ix.total += 1
  }
  return ix
}

/** True if a `from -> to` edge exists (any family, or exactly `type` when given). */
export function hasEdge(ix: CompatIndex, from: string, to: string, type?: string): boolean {
  const byType = ix.byFrom.get(from)
  if (!byType) return false
  if (type !== undefined) return byType.get(type)?.has(to) ?? false
  for (const peers of byType.values()) if (peers.has(to)) return true
  return false
}

/** True if `item` attaches to / loads into `host` via any attach family. */
export function canAttach(ix: CompatIndex, item: string, host: string): boolean {
  return ATTACH_EDGE_TYPES.some((t) => hasEdge(ix, item, host, t))
}

/** True if `item` is in `character`'s default loadout (see EQUIP_EDGE_TYPES). */
export function canEquip(ix: CompatIndex, item: string, character: string): boolean {
  return EQUIP_EDGE_TYPES.some((t) => hasEdge(ix, item, character, t))
}

function collect(
  side: Map<string, Map<string, Set<string>>>,
  key: string,
  type?: string,
): string[] {
  const byType = side.get(key)
  if (!byType) return []
  const out = new Set<string>()
  if (type !== undefined) {
    for (const p of byType.get(type) ?? []) out.add(p)
  } else {
    for (const peers of byType.values()) for (const p of peers) out.add(p)
  }
  return [...out].sort()
}

/** Items a host accepts (sorted; optionally one family) — Forge dropdown feed. */
export function itemsFor(ix: CompatIndex, host: string, type?: string): string[] {
  return collect(ix.byTo, host, type)
}

/** Hosts an item goes in/on (sorted; optionally one family). */
export function hostsFor(ix: CompatIndex, item: string, type?: string): string[] {
  return collect(ix.byFrom, item, type)
}

/**
 * Flatten the index back to its edge tuple set (evidence is not indexed —
 * tuples project to from/to/type). Feeds the G6 losslessness gate: for any
 * input set S, edgesOf(buildIndex(S)) = project(S), from either side.
 */
export function edgesOf(
  ix: CompatIndex,
  side: 'from' | 'to' = 'from',
): Array<Omit<CompatEdgeTuple, 'evidence'>> {
  const out: Array<Omit<CompatEdgeTuple, 'evidence'>> = []
  if (side === 'from') {
    for (const [from, byType] of ix.byFrom)
      for (const [type, peers] of byType)
        for (const to of peers) out.push({ from_node: from, to_node: to, edge_type: type })
  } else {
    for (const [to, byType] of ix.byTo)
      for (const [type, peers] of byType)
        for (const from of peers) out.push({ from_node: from, to_node: to, edge_type: type })
  }
  return out
}

/** Cheap telemetry for HUD/debug surfaces. */
export function stats(ix: CompatIndex): { total: number; byType: Record<string, number> } {
  return { total: ix.total, byType: { ...ix.byType } }
}
