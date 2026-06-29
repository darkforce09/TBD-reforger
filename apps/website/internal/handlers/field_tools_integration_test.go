package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestFieldToolsIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	mmID := fmt.Sprintf("itest-ft-%d", time.Now().UnixNano())
	adminID := fmt.Sprintf("itest-ft-adm-%d", time.Now().UnixNano())
	mission := models.Mission{
		Title: "Op Fire Test", AuthorID: mmID, Terrain: models.TerrainEveron,
		GameMode: models.GameModePvECoop, Weather: models.WeatherClear, TimeOfDay: "14:00",
		MaxPlayers: 32, Status: models.MissionLive,
	}
	event := models.Event{StartTime: time.Now().Add(48 * time.Hour), MaxSlots: 10, CreatedBy: mmID}

	t.Cleanup(func() {
		gdb.Where("created_by = ?", mmID).Delete(&models.FireMission{})
		gdb.Unscoped().Where("created_by = ?", mmID).Delete(&models.Event{})
		gdb.Where("actor_id IN ?", []string{mmID, adminID}).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("id = ?", mission.ID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id IN ?", []string{mmID, adminID}).Delete(&models.User{})
		_ = os.Remove("missions/" + mission.ID.String() + ".mission.json")
	})

	gdb.Create(&models.User{DiscordID: mmID, Username: "Mortar Mike", Role: models.RoleMissionMaker})
	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Ann", Role: models.RoleAdmin})
	gdb.Create(&mission)
	gdb.Create(&event)
	mmTok, _, _ := h.JWT().IssueAccess(mmID, "mission_maker", false)
	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)

	// --- live solve (no save) ---
	w := do(r, "POST", "/api/v1/fire-missions/solve", reqOpt{bearer: mmTok, body: `{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":1240}`})
	if w.Code != http.StatusOK {
		t.Fatalf("solve = %d, body=%s", w.Code, w.Body.String())
	}
	var sol struct {
		DistanceM     int     `json:"distance_m"`
		AzimuthDeg    float64 `json:"azimuth_deg"`
		ElevationMils int     `json:"elevation_mils"`
	}
	mustJSON(t, w, &sol)
	if sol.DistanceM != 1240 || sol.AzimuthDeg != 0 || sol.ElevationMils < 800 {
		t.Fatalf("unexpected solution: %+v", sol)
	}

	// --- out of range -> 422 ---
	if w := do(r, "POST", "/api/v1/fire-missions/solve", reqOpt{bearer: mmTok, body: `{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":100000}`}); w.Code != http.StatusUnprocessableEntity {
		t.Fatalf("out of range solve = %d, want 422", w.Code)
	}

	// --- save a fire mission on the event, then list it ---
	saveBody := fmt.Sprintf(`{"event_id":%q,"weapon_system":"M252 81mm","fp_grid":"FP Alpha","target_grid":"TGT 001","fp_x":0,"fp_y":0,"tgt_x":860,"tgt_y":890}`, event.ID.String())
	if w := do(r, "POST", "/api/v1/fire-missions", reqOpt{bearer: mmTok, body: saveBody}); w.Code != http.StatusCreated {
		t.Fatalf("save fire mission = %d, body=%s", w.Code, w.Body.String())
	}
	w = do(r, "GET", "/api/v1/events/"+event.ID.String()+"/fire-missions", reqOpt{bearer: mmTok})
	var list struct {
		Data []models.FireMission `json:"data"`
	}
	mustJSON(t, w, &list)
	if len(list.Data) != 1 || list.Data[0].FPGrid != "FP Alpha" || list.Data[0].DistanceM == 0 {
		t.Fatalf("fire mission list wrong: %+v", list.Data)
	}

	// --- inject is admin-only (T-122 T4): a mission_maker is forbidden ---
	if w := do(r, "POST", "/api/v1/missions/"+mission.ID.String()+"/inject", reqOpt{bearer: mmTok}); w.Code != http.StatusForbidden {
		t.Fatalf("mission_maker inject = %d, want 403", w.Code)
	}
	// --- admin injects mission.json: stages a file + audit row ---
	w = do(r, "POST", "/api/v1/missions/"+mission.ID.String()+"/inject", reqOpt{bearer: adminTok})
	if w.Code != http.StatusAccepted {
		t.Fatalf("inject = %d, body=%s", w.Code, w.Body.String())
	}
	var inj struct {
		StagedPath string `json:"staged_path"`
		Version    string `json:"version"`
	}
	mustJSON(t, w, &inj)
	data, err := os.ReadFile(inj.StagedPath)
	if err != nil {
		t.Fatalf("staged file not written: %v", err)
	}
	var doc map[string]any
	if err := json.Unmarshal(data, &doc); err != nil {
		t.Fatalf("staged file not valid json: %v", err)
	}
	if doc["title"] != "Op Fire Test" || doc["exportFormatVersion"] == nil {
		t.Fatalf("staged mission.json wrong: %v", doc)
	}
	var injAudit int64
	gdb.Model(&models.AuditLog{}).Where("action = ? AND target_id = ?", "mission.inject", mission.ID.String()).Count(&injAudit)
	if injAudit < 1 {
		t.Fatal("expected mission.inject audit row")
	}
}
