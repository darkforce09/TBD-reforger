package models

import (
	"time"

	"github.com/google/uuid"
)

// RegistryItem is one placeable/equipable engine item in a modpack's flat
// catalog (the web Virtual Arsenal source). Items are identified by their full
// Enfusion ResourceName (resource_name, e.g. "{GUID}Prefabs/.../File.et") — the
// same string stored on Slot.assetId and emitted in the loadout export. This is
// a separate layer from the alias spawn registry; see the T-068 program spec.
//
// Unique per (modpack_id, resource_name); kind is one of character,
// gear_primary, gear_uniform, gear_vest, gear_helmet.
//
// @contract registry-items.schema.json#/$defs/item
type RegistryItem struct {
	ID           uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	ModpackID    uuid.UUID `gorm:"type:uuid;column:modpack_id;not null;uniqueIndex:idx_registry_items_modpack_resource,priority:1" json:"modpack_id"`
	ResourceName string    `gorm:"column:resource_name;not null;uniqueIndex:idx_registry_items_modpack_resource,priority:2" json:"resource_name"`
	DisplayName  string    `gorm:"column:display_name;not null" json:"display_name"`
	Category     string    `gorm:"not null" json:"category"` // slash-delimited browse path, e.g. "NATO/US_Army/Rifleman"
	IconURL      string    `gorm:"column:icon_url" json:"icon_url,omitempty"`
	Kind         string    `gorm:"not null" json:"kind"`
	SortOrder    int       `gorm:"column:sort_order;not null;default:0" json:"sort_order"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"` // feeds the weak ETag (max updated_at)
}
