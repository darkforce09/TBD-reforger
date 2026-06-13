package missionvalidate

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

// Validate checks raw Mission JSON against tbd-schema via the shared ajv script.
func Validate(schemaDir string, raw []byte) error {
	if !json.Valid(raw) {
		return fmt.Errorf("invalid JSON")
	}

	script := filepath.Join(schemaDir, "scripts", "validate-file.mjs")
	if _, err := os.Stat(script); err != nil {
		return fmt.Errorf("schema validator missing at %s: %w", script, err)
	}

	cmd := exec.Command("node", script, "-")
	cmd.Dir = schemaDir
	cmd.Stdin = bytes.NewReader(raw)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("mission validation failed: %s", bytes.TrimSpace(out))
	}
	return nil
}

// MetaID extracts meta.id from validated mission JSON.
func MetaID(raw []byte) (string, error) {
	var doc struct {
		Meta struct {
			ID string `json:"id"`
		} `json:"meta"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		return "", err
	}
	if doc.Meta.ID == "" {
		return "", fmt.Errorf("meta.id is required")
	}
	return doc.Meta.ID, nil
}

// MetaName extracts meta.name from mission JSON.
func MetaName(raw []byte) (string, error) {
	var doc struct {
		Meta struct {
			Name string `json:"name"`
		} `json:"meta"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		return "", err
	}
	if doc.Meta.Name == "" {
		return "", fmt.Errorf("meta.name is required")
	}
	return doc.Meta.Name, nil
}
