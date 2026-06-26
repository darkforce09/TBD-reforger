// Virtual Arsenal registry catalog (T-068). Mirrors internal/models/registry.go
// (RegistryItem) and the GET /api/v1/registry response. Types only — the palette
// wiring lands in T-068.3.

export type RegistryItemKind =
  | 'character'
  | 'gear_primary'
  | 'gear_uniform'
  | 'gear_vest'
  | 'gear_helmet'

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

export interface RegistryResponse {
  data: RegistryItem[]
  etag: string
  modpack_id: string
  modpack_version: string
}
