// Virtual Arsenal registry catalog (T-068). Types only — the palette wiring lands in T-068.3.

/**
 * The kind of a registry item (T-068.10.2 v3 vocabulary; Phase 1 kinds remain
 * valid and gear_uniform is retired to 0 rows but still accepted). Mirrors
 * registry-items.schema.json#/$defs/item kind enum; taxonomy map:
 * .ai/artifacts/ace_arsenal_taxonomy_map.md.
 *
 * @model models.RegistryItem
 */
export type RegistryItemKind =
  | 'character'
  | 'gear_primary'
  | 'gear_handgun'
  | 'gear_launcher'
  | 'gear_throwable'
  | 'gear_explosive'
  | 'gear_uniform'
  | 'gear_jacket'
  | 'gear_pants'
  | 'gear_boots'
  | 'gear_vest'
  | 'gear_armored_vest'
  | 'gear_helmet'
  | 'gear_backpack'
  | 'gear_glasses'
  | 'gear_gloves'
  | 'gear_binoculars'
  | 'gear_item'
  | 'magazine'
  | 'ammo'
  | 'optic'
  | 'attachment'
  | 'vehicle'
  | 'vehicle_weapon'
  | 'crate'
  | 'other'

/**
 * One Virtual Arsenal catalog item, identified by its full Enfusion resource_name.
 *
 * @model models.RegistryItem
 * @contract registry-items.schema.json#/$defs/item
 */
export interface RegistryItem {
  id: string
  modpack_id: string
  resource_name: string
  display_name: string
  category: string
  icon_url?: string | null
  kind: RegistryItemKind
  /** Non-placeable template prefab (`*_base.et` / `* Base`) — hidden from pickers. */
  abstract?: boolean | null
  /** SCR_EArsenalItemType flag name when a faction EntityCatalog entry exists. */
  arsenal_type?: string | null
  /** ItemPhysicalAttributes.Weight in kg; null = engine class default. */
  weight_kg?: number | null
  /** ItemPhysicalAttributes.ItemVolume in cm³; null = engine class default. */
  volume_cm3?: number | null
  /** Container carry capacity (kg) when the item is itself a container. */
  max_weight_kg?: number | null
  /** Container volume capacity (cm³) when the item is itself a container. */
  max_volume_cm3?: number | null
  /** Addon ID the prefab was scanned from (joins the envelope addons[] scan set). */
  addon?: string | null
  sort_order: number
  created_at: string
  updated_at: string
}

/**
 * The registry list response: catalog rows plus the modpack id/version and a weak ETag.
 *
 * @contract registry-items.schema.json#/
 * @route GET /api/v1/registry
 */
export interface RegistryResponse {
  data: RegistryItem[]
  etag: string
  modpack_id: string
  modpack_version: string
}

/**
 * A compat edge family (T-150 v1 vocabulary). The graph code treats edge types
 * as plain strings so future families flow through untouched; this union is
 * the current schema enum for call-site safety.
 *
 * @contract registry-compat.schema.json#/$defs/edge/properties/edge_type
 */
export type RegistryCompatEdgeType =
  | 'mag_in_weapon'
  | 'ammo_in_mag'
  | 'optic_on_weapon'
  | 'attachment_on_weapon'
  | 'mag_in_vehicle_weapon'
  | 'ammo_in_vehicle_weapon'
  | 'character_default_loadout'
  | 'character_default_weapon'

/**
 * One directed compatibility edge: `from_node` goes in/on `to_node`.
 * `evidence` is omitted by the API when empty (NULL ≡ '' ≡ absent).
 *
 * @model models.RegistryCompatEdge
 * @contract registry-compat.schema.json#/$defs/edge
 */
export interface RegistryCompatEdge {
  id: string
  modpack_id: string
  from_node: string
  to_node: string
  edge_type: RegistryCompatEdgeType
  evidence?: string | null
  created_at: string
  updated_at: string
}

/**
 * The compat graph response: edge rows plus the modpack id/version and a weak
 * ETag (`?edge_type=` filtered responses carry a distinct ETag).
 *
 * @contract registry-compat.schema.json#/
 * @route GET /api/v1/registry/compat
 */
export interface RegistryCompatResponse {
  data: RegistryCompatEdge[]
  etag: string
  modpack_id: string
  modpack_version: string
}
