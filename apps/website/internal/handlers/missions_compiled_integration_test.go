package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/contract"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// compiledFixturePayload is a minimal editor superset (compile.ts shape) with two
// factions, callsigned squads, a duplicate role (TL x2) and one slot carrying a real
// elevation, so the flatten's id/kit/orbat/y rules are all exercised end-to-end.
const compiledFixturePayload = `{
  "schemaVersion": 1,
  "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
  "editor": {
    "factions": [
      {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
      {"id": "f2", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
    ],
    "squads": [
      {"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1", "s2", "s3"]},
      {"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]}
    ],
    "slots": [
      {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270}},
      {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
      {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0}},
      {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90}}
    ],
    "editorLayers": []
  }
}`

// TestGetCompiledMission proves the T-092.2 game-server route: service-token auth,
// the canonical mission.schema.json document body (validated against the embedded
// schema), the locked editor->mod coordinate mapping, deterministic slot ids, and
// orbat/slots count parity (TBD_MissionLoader hard-fails on mismatch).
func TestGetCompiledMission(t *testing.T) {
	r, _, gdb := setupIT(t)
	makerID := fmt.Sprintf("itest-cmp-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Unscoped().Where("created_by = ?", makerID).Delete(&models.MissionVersion{})
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id = ?", makerID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: makerID, Username: "Compile Cara", Role: models.RoleMissionMaker})
	mission := models.Mission{
		Title:      "Compiled Fixture",
		AuthorID:   makerID,
		Terrain:    models.TerrainEveron,
		GameMode:   models.GameModePvECoop,
		Weather:    models.WeatherClear,
		TimeOfDay:  "05:30",
		MaxPlayers: 64,
		Status:     models.MissionDraft,
	}
	if err := gdb.Create(&mission).Error; err != nil {
		t.Fatalf("seed mission: %v", err)
	}
	path := "/api/v1/missions/" + mission.ID.String() + "/compiled"

	// No saved version yet -> deliberate 409, not an invalid document.
	if w := do(r, "GET", path, reqOpt{service: "svc-token"}); w.Code != http.StatusConflict {
		t.Fatalf("compiled without version = %d, want 409 (body=%s)", w.Code, w.Body.String())
	}

	version := models.MissionVersion{
		MissionID:   mission.ID,
		Semver:      "0.1.0",
		JSONPayload: []byte(compiledFixturePayload),
		CreatedBy:   makerID,
	}
	if err := gdb.Create(&version).Error; err != nil {
		t.Fatalf("seed version: %v", err)
	}
	if err := gdb.Model(&mission).Update("current_version_id", version.ID).Error; err != nil {
		t.Fatalf("set current version: %v", err)
	}

	// Game-server tier: no token -> 401; user bearer is NOT accepted on this route.
	if w := do(r, "GET", path, reqOpt{}); w.Code != http.StatusUnauthorized {
		t.Fatalf("compiled without token = %d, want 401 (body=%s)", w.Code, w.Body.String())
	}

	w := do(r, "GET", path, reqOpt{service: "svc-token"})
	if w.Code != http.StatusOK {
		t.Fatalf("compiled = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}

	// Body must validate against the canonical mission.schema.json (S1/S4).
	if details, err := contract.ValidateMissionDocument(w.Body.Bytes()); err != nil {
		t.Fatalf("schema validator: %v", err)
	} else if len(details) > 0 {
		t.Fatalf("compiled document violates mission.schema.json:\n%s", details)
	}

	var doc services.ModMissionDocument
	if err := json.Unmarshal(w.Body.Bytes(), &doc); err != nil {
		t.Fatalf("decode document: %v", err)
	}

	if doc.SchemaVersion != "1.2" {
		t.Fatalf("schemaVersion = %q, want 1.2 (one slot carries y)", doc.SchemaVersion)
	}
	wantIDs := []string{"blufor:Alpha:SL:0", "blufor:Alpha:TL:0", "blufor:Alpha:TL:1", "opfor:Grom:RFL:0"}
	if len(doc.Slots) != len(wantIDs) {
		t.Fatalf("slots length = %d, want %d", len(doc.Slots), len(wantIDs))
	}
	for i, want := range wantIDs {
		if doc.Slots[i].ID != want {
			t.Fatalf("slot[%d].id = %q, want %q", i, doc.Slots[i].ID, want)
		}
	}

	// Locked mapping: editor position.x -> x, position.y -> z, position.z -> y, rotation -> headingDeg.
	sl := doc.Slots[0]
	if sl.X != 4839.2 || sl.Z != 6620.8 || sl.Y != nil || sl.HeadingDeg != 270 {
		t.Fatalf("slot[0] mapping wrong: %+v", sl)
	}
	if doc.Slots[1].Y == nil || *doc.Slots[1].Y != 142.5 || doc.Slots[1].HeadingDeg != 90 {
		t.Fatalf("slot[1] y/heading wrong: %+v", doc.Slots[1])
	}
	if sl.Kit != "kit:us_sl" || doc.Slots[1].Kit != "kit:us_rifleman" || doc.Slots[3].Kit != "kit:sov_rifleman" {
		t.Fatalf("kit aliases wrong: %s / %s / %s", sl.Kit, doc.Slots[1].Kit, doc.Slots[3].Kit)
	}

	// Orbat instance count must equal slots length (loader parity gate).
	orbatCount := 0
	for _, f := range doc.Orbat {
		for _, g := range f.Groups {
			for _, role := range g.Roles {
				orbatCount += role.Count
			}
		}
	}
	if orbatCount != len(doc.Slots) {
		t.Fatalf("orbat instances = %d, slots = %d — loader would reject", orbatCount, len(doc.Slots))
	}

	if doc.Meta.ID == "" || doc.Meta.PlayerRange != [2]int{1, 64} {
		t.Fatalf("meta synthesized wrong: %+v", doc.Meta)
	}

	// A version whose editor graph has no placed slots -> 409 (ErrNoSlots).
	empty := models.MissionVersion{
		MissionID:   mission.ID,
		Semver:      "0.1.1",
		JSONPayload: []byte(`{"schemaVersion":1,"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}`),
		CreatedBy:   makerID,
	}
	if err := gdb.Create(&empty).Error; err != nil {
		t.Fatalf("seed empty version: %v", err)
	}
	if err := gdb.Model(&mission).Update("current_version_id", empty.ID).Error; err != nil {
		t.Fatalf("point at empty version: %v", err)
	}
	if w := do(r, "GET", path, reqOpt{service: "svc-token"}); w.Code != http.StatusConflict {
		t.Fatalf("compiled with slotless version = %d, want 409 (body=%s)", w.Code, w.Body.String())
	}
}
