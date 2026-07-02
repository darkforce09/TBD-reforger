package handlers

import (
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// TestExportMissionDanglingVersion500 proves T-130.1 F2B-08: a mission whose
// current_version_id points at a missing version row must export a 500, not a
// silent 200 with an empty payload and version 0.0.0.
func TestExportMissionDanglingVersion500(t *testing.T) {
	r, h, gdb := setupIT(t)
	makerID := fmt.Sprintf("itest-exp-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id = ?", makerID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: makerID, Username: "Export Erin", Role: models.RoleMissionMaker})
	makerTok, _, _ := h.JWT().IssueAccess(makerID, "mission_maker", false)

	dangling := uuid.New() // no such mission_versions row
	mission := models.Mission{
		Title:      "Broken Pointer",
		AuthorID:   makerID,
		Terrain:    models.TerrainEveron,
		GameMode:   models.GameModePvECoop,
		Weather:    models.WeatherClear,
		TimeOfDay:  "14:00",
		MaxPlayers: 8,
		Status:     models.MissionDraft,
	}
	if err := gdb.Create(&mission).Error; err != nil {
		t.Fatalf("seed mission: %v", err)
	}
	if err := gdb.Model(&mission).Update("current_version_id", dangling).Error; err != nil {
		t.Fatalf("set dangling version: %v", err)
	}

	w := do(r, "GET", "/api/v1/missions/"+mission.ID.String()+"/export", reqOpt{bearer: makerTok})
	if w.Code != http.StatusInternalServerError {
		t.Fatalf("export with dangling version = %d, want 500 (body=%s)", w.Code, w.Body.String())
	}

	// A version-less mission (nil pointer) still exports its defaults — the error
	// is reserved for a pointer that fails to load.
	if err := gdb.Model(&mission).Update("current_version_id", nil).Error; err != nil {
		t.Fatalf("clear version pointer: %v", err)
	}
	if w := do(r, "GET", "/api/v1/missions/"+mission.ID.String()+"/export", reqOpt{bearer: makerTok}); w.Code != http.StatusOK {
		t.Fatalf("export without version = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
}
