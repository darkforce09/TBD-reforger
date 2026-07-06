// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/registry-items.schema.json — regenerate with: make schema-codegen
// To parse and unparse this JSON data, add this code to your project and do:
//
//    registryitemsGen, err := UnmarshalRegistryitemsGen(bytes)
//    bytes, err = registryitemsGen.Marshal()

package registryitems

import "time"

import "encoding/json"

func UnmarshalRegistryitemsGen(data []byte) (RegistryitemsGen, error) {
	var r RegistryitemsGen
	err := json.Unmarshal(data, &r)
	return r, err
}

func (r *RegistryitemsGen) Marshal() ([]byte, error) {
	return json.Marshal(r)
}

// Flat catalog of placeable/equipable engine items exported from the TBD-Content Workbench.
// Items are identified by their full Enfusion ResourceName (resource_name). This is a
// separate layer from the alias spawn registry (registry.schema.json): the alias registry
// maps mission aliases to GUIDs for spawn, this catalog drives the web Virtual Arsenal
// (browse, seed/import, loadout build).
type RegistryitemsGen struct {
	GeneratedAt          *time.Time            `json:"generatedAt,omitempty"`
	Items                []RegistryItemsSchema `json:"items"`
	ModpackID            string                `json:"modpackId"`
	RegistryItemsVersion string                `json:"registryItemsVersion"`
}

type RegistryItemsSchema struct {
	// Slash-delimited browse path, e.g. NATO/Rifleman.                                
	Category                                                                   string  `json:"category"`
	DisplayName                                                                string  `json:"display_name"`
	IconURL                                                                    *string `json:"icon_url,omitempty"`
	Kind                                                                       Kind    `json:"kind"`
	// Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.        
	ResourceName                                                               string  `json:"resource_name"`
}

type Kind string

const (
	Character   Kind = "character"
	GearHelmet  Kind = "gear_helmet"
	GearPrimary Kind = "gear_primary"
	GearUniform Kind = "gear_uniform"
	GearVest    Kind = "gear_vest"
)
