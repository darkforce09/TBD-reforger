package handlers

import (
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// TestEditorOnlyOrbatDerivationIntegration verifies the T-062.1.1 Save dedup end-to-end:
// a mission version saved with an editor-only payload (NO top-level "orbat") still produces
// a correct auto-ORBAT when attached to an event WITHOUT an explicit orbat — the server
// derives it from the editor graph (factions → squads → slots, sorted by slot.index).
func TestEditorOnlyOrbatDerivationIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	makerID := fmt.Sprintf("itest-orbmm-%d", time.Now().UnixNano())
	adminID := fmt.Sprintf("itest-orbadm-%d", time.Now().UnixNano())
	playerID := fmt.Sprintf("itest-orbpl-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		var ms []models.Mission
		gdb.Unscoped().Where("author_id = ?", makerID).Find(&ms)
		for _, m := range ms {
			gdb.Where("mission_id = ?", m.ID).Delete(&models.MissionVersion{})
		}
		var evs []models.Event
		gdb.Unscoped().Where("created_by = ?", adminID).Find(&evs)
		for _, e := range evs {
			var ems []models.EventMission
			gdb.Where("event_id = ?", e.ID).Find(&ems)
			for _, em := range ems {
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.OrbatSlot{})
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.EventRegistration{})
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.OrbatReservation{})
			}
			gdb.Where("event_id = ?", e.ID).Delete(&models.EventMission{})
		}
		gdb.Unscoped().Where("created_by = ?", adminID).Delete(&models.Event{})
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id IN ?", []string{makerID, adminID, playerID}).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: makerID, Username: "Maker Mike", Role: models.RoleMissionMaker})
	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Dave", Role: models.RoleAdmin})
	gdb.Create(&models.User{DiscordID: playerID, Username: "Player One", Role: models.RoleEnlisted})
	makerTok, _, _ := h.JWT().IssueAccess(makerID, "mission_maker", false)
	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	playerTok, _, _ := h.JWT().IssueAccess(playerID, "enlisted", false)

	// --- create the mission ---
	createBody := `{"title":"Editor-Only ORBAT","terrain":"everon","game_mode":"pve_coop","weather":"clear","time_of_day":"08:00","max_players":32}`
	w := do(r, "POST", "/api/v1/missions", reqOpt{bearer: makerTok, body: createBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("create mission = %d, body=%s", w.Code, w.Body.String())
	}
	var mission models.Mission
	mustJSON(t, w, &mission)
	mid := mission.ID.String()

	// --- save an editor-only version (NO top-level "orbat" key). Alpha's slotIds are listed
	//     out of index order to prove server-side sorting by slot.index. ---
	verBody := `{"semver":"1.0.0","editor_notes":"editor-only","payload":{
		"schemaVersion":1,
		"map":{"terrain":"everon","bounds":[0,0,12800,12800]},
		"environment":{},
		"editor":{
			"factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq-a","sq-b"]}],
			"squads":[
				{"id":"sq-a","factionId":"f1","callsign":"Alpha Actual","name":"Alpha 1-1","slotIds":["s2","s0","s1"]},
				{"id":"sq-b","factionId":"f1","name":"Bravo 1-1","slotIds":["b0"]}
			],
			"slots":[
				{"id":"s0","squadId":"sq-a","index":0,"role":"Squad Leader","tag":""},
				{"id":"s1","squadId":"sq-a","index":1,"role":"Combat Medic","tag":"MED"},
				{"id":"s2","squadId":"sq-a","index":2,"role":"Rifleman","tag":""},
				{"id":"b0","squadId":"sq-b","index":0,"role":"Team Leader","tag":""}
			],
			"editorLayers":[]
		}
	}}`
	w = do(r, "POST", "/api/v1/missions/"+mid+"/versions", reqOpt{bearer: makerTok, body: verBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("create version = %d, body=%s", w.Code, w.Body.String())
	}

	// --- create an event ---
	start := time.Now().Add(72 * time.Hour).UTC().Format(time.RFC3339)
	w = do(r, "POST", "/api/v1/events", reqOpt{bearer: adminTok, body: fmt.Sprintf(`{"start_time":%q,"name_override":"Auto ORBAT Op"}`, start)})
	if w.Code != http.StatusCreated {
		t.Fatalf("create event = %d, body=%s", w.Code, w.Body.String())
	}
	var event models.Event
	mustJSON(t, w, &event)
	eid := event.ID.String()

	// --- attach the mission with mission_id + start_time ONLY (no explicit orbat) ---
	missionStart := time.Now().Add(73 * time.Hour).UTC().Format(time.RFC3339)
	addBody := fmt.Sprintf(`{"mission_id":%q,"start_time":%q}`, mid, missionStart)
	w = do(r, "POST", "/api/v1/events/"+eid+"/missions", reqOpt{bearer: adminTok, body: addBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("add event mission = %d, body=%s", w.Code, w.Body.String())
	}
	var em models.EventMission
	mustJSON(t, w, &em)
	emid := em.ID.String()

	// --- ORBAT must be derived from the editor block: 2 squads, 4 slots total ---
	w = do(r, "GET", "/api/v1/event-missions/"+emid+"/orbat", reqOpt{bearer: playerTok})
	if w.Code != http.StatusOK {
		t.Fatalf("get orbat = %d, body=%s", w.Code, w.Body.String())
	}
	var orbat struct {
		Data []orbatSquadDTO `json:"data"`
	}
	mustJSON(t, w, &orbat)

	if len(orbat.Data) != 2 {
		t.Fatalf("expected 2 squads, got %d: %+v", len(orbat.Data), orbat.Data)
	}
	totalSlots := 0
	var alpha *orbatSquadDTO
	var medic orbatSlotDTO
	for i := range orbat.Data {
		totalSlots += orbat.Data[i].Total
		if orbat.Data[i].Squad == "Alpha 1-1" {
			alpha = &orbat.Data[i]
		}
		for _, s := range orbat.Data[i].Slots {
			if s.Role == "Combat Medic" {
				medic = s
			}
		}
	}
	if totalSlots != 4 {
		t.Fatalf("expected 4 derived slots, got %d", totalSlots)
	}
	if alpha == nil {
		t.Fatal("Alpha 1-1 squad missing from derived ORBAT")
	}
	if alpha.Faction != "BLUFOR" || alpha.Callsign != "Alpha Actual" || alpha.Total != 3 {
		t.Fatalf("alpha header/total wrong: %+v", alpha)
	}
	// Order must follow slot.index (s0,s1,s2) despite the scrambled slotIds.
	wantRoles := []string{"Squad Leader", "Combat Medic", "Rifleman"}
	for i, want := range wantRoles {
		if alpha.Slots[i].Role != want {
			t.Fatalf("alpha slot %d role = %q, want %q", i, alpha.Slots[i].Role, want)
		}
	}
	if medic.Tag != "MED" || medic.Number != 2 {
		t.Fatalf("medic slot wrong: number=%d tag=%q", medic.Number, medic.Tag)
	}
}
