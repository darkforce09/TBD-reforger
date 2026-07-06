// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/mission-editor-payload.schema.json — regenerate with: make schema-codegen
// To parse and unparse this JSON data, add this code to your project and do:
//
//    missioneditorGen, err := UnmarshalMissioneditorGen(bytes)
//    bytes, err = missioneditorGen.Marshal()

package missioneditor

import "encoding/json"

func UnmarshalMissioneditorGen(data []byte) (MissioneditorGen, error) {
	var r MissioneditorGen
	err := json.Unmarshal(data, &r)
	return r, err
}

func (r *MissioneditorGen) Marshal() ([]byte, error) {
	return json.Marshal(r)
}

// The 2D-editor 'superset' stored verbatim as a MissionVersion.json_payload (the write side
// of POST /api/v1/missions/:id/versions; mirrors the frontend compile.ts MissionPayload).
// This is NOT the canonical mission.schema.json document — that is the game-server contract
// derived/exported separately. Its integer schemaVersion is the editor-payload format
// version, a DISTINCT namespace from the canonical mission contract's string schemaVersion.
// Validation is intentionally lenient on presence (minimal and partial saves are valid,
// including the empty {} a freshly created mission stores) but strict on type, to reject
// malformed payloads and the schemaVersion namespace confusion (a string here) before
// persist.
type MissioneditorGen struct {
	// Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so                       
	// validation stays O(1) on missions with hundreds of thousands of slots.                                          
	Editor                                                                                      *Editor                `json:"editor,omitempty"`
	Environment                                                                                 map[string]interface{} `json:"environment,omitempty"`
	Loadouts                                                                                    map[string]interface{} `json:"loadouts,omitempty"`
	Map                                                                                         *Map                   `json:"map,omitempty"`
	Markers                                                                                     []interface{}          `json:"markers,omitempty"`
	Objectives                                                                                  []interface{}          `json:"objectives,omitempty"`
	// Optional backend ORBAT contract (omitted on Save Version; the server derives it from                            
	// editor).                                                                                                        
	Orbat                                                                                       []interface{}          `json:"orbat,omitempty"`
	// Editor-payload format version (integer; do not confuse with the canonical mission                               
	// schemaVersion, which is a string).                                                                              
	SchemaVersion                                                                               *int64                 `json:"schemaVersion,omitempty"`
	Vehicles                                                                                    []interface{}          `json:"vehicles,omitempty"`
}

// Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so
// validation stays O(1) on missions with hundreds of thousands of slots.
type Editor struct {
	EditorLayers []interface{} `json:"editorLayers,omitempty"`
	Factions     []interface{} `json:"factions,omitempty"`
	Slots        []interface{} `json:"slots,omitempty"`
	Squads       []interface{} `json:"squads,omitempty"`
}

type Map struct {
	Bounds  []float64 `json:"bounds,omitempty"`
	Terrain *string   `json:"terrain,omitempty"`
}
