// Faction library (T-153) — operator-authored reusable factions consumed by the Mission
// Creator palette (side → faction → roles/vehicles) and authored in the Faction Manager.

import type { SlotLoadoutV2 } from '@/features/tactical-map'

/** Export-side key — mirrors the mission doc Faction.key vocabulary. */
export type FactionSide = 'BLUFOR' | 'OPFOR' | 'INDFOR' | 'CIV'

export const FACTION_SIDES: readonly FactionSide[] = ['BLUFOR', 'OPFOR', 'INDFOR', 'CIV']

/**
 * One ORBAT role template: a registry character (vanilla bodies are fine here — the
 * palette hides them, role templates wrap them) plus an optional SlotLoadout v2.
 *
 * @contract faction-library.schema.json#/$defs/role
 */
export interface FactionRole {
  role: string
  tag?: string
  character: string
  loadout?: SlotLoadoutV2
}

/** @contract faction-library.schema.json#/$defs/vehicle */
export interface FactionVehicle {
  vehicle: string
  label?: string
}

/**
 * The full faction document (the jsonb `doc` of a user_factions row).
 *
 * @contract faction-library.schema.json#/
 */
export interface FactionDoc {
  side: FactionSide
  name: string
  emblem?: string
  roles: FactionRole[]
  vehicles: FactionVehicle[]
}

/**
 * One faction library row as served by the API (side/name are projections of doc).
 *
 * @model models.UserFaction
 */
export interface UserFaction {
  id: string
  owner_id: string
  side: FactionSide
  name: string
  doc: FactionDoc
  created_at: string
  updated_at: string
}

/**
 * The list response (house list shape).
 *
 * @route GET /api/v1/factions
 */
export interface FactionListResponse {
  data: UserFaction[]
  total: number
  limit: number
  offset: number
}
