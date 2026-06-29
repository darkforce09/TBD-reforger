// Package contract holds the generated cross-boundary type projections (in its subpackages)
// and the runtime JSON Schema validator for mission version payloads
// (docs/platform/DOCUMENTATION_STANDARDS.md §9.2). The schema files it validates against are
// copied into ./schema by `make schema-codegen`, because go:embed cannot reach the canonical
// copies under packages/tbd-schema/ (outside this Go module).
package contract

import (
	"bytes"
	_ "embed"
	"errors"
	"fmt"
	"strings"
	"sync"

	"github.com/santhosh-tekuri/jsonschema/v6"
)

//go:embed schema/mission-editor-payload.schema.json
var missionEditorPayloadSchema []byte

var (
	editorOnce   sync.Once
	editorSchema *jsonschema.Schema
	editorErr    error
)

// missionEditorValidator compiles the embedded editor-payload schema exactly once.
func missionEditorValidator() (*jsonschema.Schema, error) {
	editorOnce.Do(func() {
		doc, err := jsonschema.UnmarshalJSON(bytes.NewReader(missionEditorPayloadSchema))
		if err != nil {
			editorErr = fmt.Errorf("parse embedded mission-editor-payload schema: %w", err)
			return
		}
		c := jsonschema.NewCompiler()
		const url = "mission-editor-payload.schema.json"
		if err := c.AddResource(url, doc); err != nil {
			editorErr = fmt.Errorf("add mission-editor-payload schema: %w", err)
			return
		}
		editorSchema, editorErr = c.Compile(url)
	})
	return editorSchema, editorErr
}

// ValidateMissionEditorPayload validates a raw mission version payload against
// mission-editor-payload.schema.json (the editor superset contract — T-123.5). It returns
// (nil, nil) when the payload is valid; schema-invalid input returns its validation messages in
// details with a nil err. Only an internal schema-compile failure returns a non-nil err. The
// schema is intentionally shallow (top-level keys + types, large arrays unconstrained), so this
// stays cheap even on a 100k+ slot payload.
//
// @contract mission-editor-payload.schema.json#/
func ValidateMissionEditorPayload(raw []byte) (details []string, err error) {
	sch, cerr := missionEditorValidator()
	if cerr != nil {
		return nil, cerr
	}
	inst, jerr := jsonschema.UnmarshalJSON(bytes.NewReader(raw))
	if jerr != nil {
		return []string{"payload is not valid JSON"}, nil
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
