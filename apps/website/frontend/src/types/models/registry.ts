// Virtual Arsenal registry catalog (T-068). Types only — the palette wiring lands in T-068.3.

/**
 * The kind of a registry item (character or one of the four gear slots).
 *
 * @model models.RegistryItem
 */
export type RegistryItemKind =
  'character' | 'gear_primary' | 'gear_uniform' | 'gear_vest' | 'gear_helmet'

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
