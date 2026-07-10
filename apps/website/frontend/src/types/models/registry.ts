// Virtual Arsenal registry catalog (T-068). Types only — the palette wiring lands in T-068.3.

/**
 * The kind of a registry item (T-150 v2 vocabulary; the Phase 1 kinds are the
 * first five). Mirrors registry-items.schema.json#/$defs/item kind enum.
 *
 * @model models.RegistryItem
 */
export type RegistryItemKind =
  | 'character'
  | 'gear_primary'
  | 'gear_handgun'
  | 'gear_launcher'
  | 'gear_uniform'
  | 'gear_vest'
  | 'gear_helmet'
  | 'gear_backpack'
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
