package contract

import (
	"bytes"
	_ "embed"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"sync"

	"github.com/santhosh-tekuri/jsonschema/v6"
)

// Canonical mod mission contract (string schemaVersion "1.0"/"1.1"/"1.2") — the document
// GET /missions/:id/compiled serves and TBD_MissionLoader consumes. Distinct from the
// editor-payload schema above. Embedded copy is written by `make schema-codegen`.
//
//go:embed schema/mission.schema.json
var missionSchema []byte

// kit-aliases.json — ResourceName -> kit: alias table + per-faction defaults shared with the
// frontend flatten (canonical copy: packages/tbd-schema/registry/kit-aliases.json; mirrored
// here by `make schema-codegen` because go:embed cannot leave the module).
//
//go:embed registry/kit-aliases.json
var kitAliasesRaw []byte

var (
	missionOnce      sync.Once
	missionSchemaC   *jsonschema.Schema
	missionSchemaErr error

	kitOnce    sync.Once
	kitAliases *KitAliases
	kitErr     error
)

// KitAliases is the parsed kit-aliases.json: the assetId (ResourceName) -> kit: alias
// mapping the mission compile flatten uses, plus per-faction fallback kit/preset.
type KitAliases struct {
	Kits []struct {
		Alias        string `json:"alias"`
		ResourceName string `json:"resourceName"`
	} `json:"kits"`
	FactionDefaults map[string]struct {
		Kit    string `json:"kit"`
		Preset string `json:"preset"`
	} `json:"factionDefaults"`
	FallbackFaction string `json:"fallbackFaction"`

	resourceToKit map[string]string
}

// KitForResource resolves a slot assetId (full Enfusion ResourceName) to its kit: alias;
// ok=false means the caller should fall back to the faction default kit.
func (k *KitAliases) KitForResource(resourceName string) (string, bool) {
	alias, ok := k.resourceToKit[resourceName]
	return alias, ok
}

// FactionDefault returns the fallback kit + preset aliases for a (lowercased) faction key,
// falling back to the table's fallbackFaction for unknown factions.
func (k *KitAliases) FactionDefault(factionKey string) (kit, preset string) {
	d, ok := k.FactionDefaults[factionKey]
	if !ok {
		d = k.FactionDefaults[k.FallbackFaction]
	}
	return d.Kit, d.Preset
}

// LoadKitAliases parses the embedded kit-aliases.json exactly once.
func LoadKitAliases() (*KitAliases, error) {
	kitOnce.Do(func() {
		var parsed KitAliases
		if err := json.Unmarshal(kitAliasesRaw, &parsed); err != nil {
			kitErr = fmt.Errorf("parse embedded kit-aliases.json: %w", err)
			return
		}
		parsed.resourceToKit = make(map[string]string, len(parsed.Kits))
		for _, e := range parsed.Kits {
			parsed.resourceToKit[e.ResourceName] = e.Alias
		}
		kitAliases = &parsed
	})
	return kitAliases, kitErr
}

// missionValidator compiles the embedded canonical mission schema exactly once.
func missionValidator() (*jsonschema.Schema, error) {
	missionOnce.Do(func() {
		doc, err := jsonschema.UnmarshalJSON(bytes.NewReader(missionSchema))
		if err != nil {
			missionSchemaErr = fmt.Errorf("parse embedded mission schema: %w", err)
			return
		}
		c := jsonschema.NewCompiler()
		const url = "mission.schema.json"
		if err := c.AddResource(url, doc); err != nil {
			missionSchemaErr = fmt.Errorf("add mission schema: %w", err)
			return
		}
		missionSchemaC, missionSchemaErr = c.Compile(url)
	})
	return missionSchemaC, missionSchemaErr
}

// ValidateMissionDocument validates a compiled mod mission document against
// mission.schema.json. Same contract as ValidateMissionEditorPayload: schema violations
// come back in details with nil err; only an internal compile failure sets err.
//
// @contract mission.schema.json#/
func ValidateMissionDocument(raw []byte) (details []string, err error) {
	sch, cerr := missionValidator()
	if cerr != nil {
		return nil, cerr
	}
	inst, jerr := jsonschema.UnmarshalJSON(bytes.NewReader(raw))
	if jerr != nil {
		return []string{"document is not valid JSON"}, nil
	}
	verr := sch.Validate(inst)
	if verr == nil {
		return nil, nil
	}
	var ve *jsonschema.ValidationError
	if errors.As(verr, &ve) {
		for _, u := range ve.BasicOutput().Errors {
			if u.Error == nil {
				continue
			}
			loc := u.InstanceLocation
			if loc == "" {
				loc = "/"
			}
			details = append(details, strings.TrimSpace(loc+": "+u.Error.String()))
		}
	}
	if len(details) == 0 {
		details = []string{verr.Error()}
	}
	return details, nil
}
