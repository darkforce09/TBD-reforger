// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/loadout-export.schema.json — regenerate with: make schema-codegen
// To parse and unparse this JSON data, add this code to your project and do:
//
//    loadoutGen, err := UnmarshalLoadoutGen(bytes)
//    bytes, err = loadoutGen.Marshal()

package loadout

import "encoding/json"

func UnmarshalLoadoutGen(data []byte) (LoadoutGen, error) {
	var r LoadoutGen
	err := json.Unmarshal(data, &r)
	return r, err
}

func (r *LoadoutGen) Marshal() ([]byte, error) {
	return json.Marshal(r)
}

// Dumb loadout download: a fixed set of gear slots, each holding a resource_name (from
// registry-items) or null when empty. Consumed by the mod equip test and the web download.
type LoadoutGen struct {
	Gear           Gear           `json:"gear"`
	LoadoutVersion LoadoutVersion `json:"loadoutVersion"`
	ModpackID      string         `json:"modpackId"`
}

type Gear struct {
	Helmet  *string `json:"helmet"`
	Primary *string `json:"primary"`
	Uniform *string `json:"uniform"`
	Vest    *string `json:"vest"`
}

type LoadoutVersion string

const (
	The1 LoadoutVersion = "1"
)
