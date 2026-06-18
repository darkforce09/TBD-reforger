package handlers

import (
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestEventLifecycleIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	adminID := fmt.Sprintf("itest-evadm-%d", time.Now().UnixNano())
	u1 := fmt.Sprintf("itest-ev1-%d", time.Now().UnixNano())
	u2 := fmt.Sprintf("itest-ev2-%d", time.Now().UnixNano())
	u3 := fmt.Sprintf("itest-ev3-%d", time.Now().UnixNano())
	allUsers := []string{adminID, u1, u2, u3}

	// A live mission to schedule against.
	mission := models.Mission{
		Title: "Op Test Strike", AuthorID: adminID, Terrain: models.TerrainEveron,
		GameMode: models.GameModePvECoop, Weather: models.WeatherClear, TimeOfDay: "14:00",
		MaxPlayers: 64, Status: models.MissionLive,
	}

	t.Cleanup(func() {
		var evs []models.Event
		gdb.Unscoped().Where("created_by = ?", adminID).Find(&evs)
		for _, e := range evs {
			var ems []models.EventMission
			gdb.Where("event_id = ?", e.ID).Find(&ems)
			for _, em := range ems {
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.OrbatSlot{})
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.EventRegistration{})
			}
			gdb.Where("event_id = ?", e.ID).Delete(&models.EventMission{})
		}
		gdb.Unscoped().Where("created_by = ?", adminID).Delete(&models.Event{})
		gdb.Where("discord_id IN ?", allUsers).Delete(&models.LeaveRequest{})
		gdb.Unscoped().Where("id = ?", mission.ID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id IN ?", allUsers).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Dave", Role: models.RoleAdmin})
	gdb.Create(&models.User{DiscordID: u1, Username: "Player One", Role: models.RoleEnlisted})
	gdb.Create(&models.User{DiscordID: u2, Username: "Player Two", Role: models.RoleEnlisted})
	gdb.Create(&models.User{DiscordID: u3, Username: "Player Three", Role: models.RoleEnlisted})
	gdb.Create(&mission)

	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	t1, _, _ := h.JWT().IssueAccess(u1, "enlisted", false)
	t2, _, _ := h.JWT().IssueAccess(u2, "enlisted", false)
	t3, _, _ := h.JWT().IssueAccess(u3, "enlisted", false)

	// --- schedule an operation container ---
	start := time.Now().Add(72 * time.Hour).UTC().Format(time.RFC3339)
	createBody := fmt.Sprintf(`{"start_time":%q,"name_override":"Operation Enduring Freedom"}`, start)
	w := do(r, "POST", "/api/v1/events", reqOpt{bearer: adminTok, body: createBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("create event = %d, body=%s", w.Code, w.Body.String())
	}
	var event models.Event
	mustJSON(t, w, &event)
	eid := event.ID.String()

	// --- attach the mission with an explicit ORBAT (2 slots in 2 squads) ---
	missionStart := time.Now().Add(73 * time.Hour).UTC().Format(time.RFC3339)
	addBody := fmt.Sprintf(`{
		"mission_id":%q,"start_time":%q,
		"orbat":[
			{"faction":"US Army","callsign":"Platoon HQ","squad":"HQ","role":"Platoon Lead","count":1},
			{"faction":"US Army","squad":"Alpha 1-1","role":"Combat Medic","count":1}
		]}`, mission.ID.String(), missionStart)
	w = do(r, "POST", "/api/v1/events/"+eid+"/missions", reqOpt{bearer: adminTok, body: addBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("add event mission = %d, body=%s", w.Code, w.Body.String())
	}
	var em models.EventMission
	mustJSON(t, w, &em)
	emid := em.ID.String()

	// --- ORBAT materialized: HQ(1) + Alpha 1-1(1) = 2 slots in 2 squads ---
	w = do(r, "GET", "/api/v1/event-missions/"+emid+"/orbat", reqOpt{bearer: t1})
	var orbat struct {
		Data []orbatSquadDTO `json:"data"`
	}
	mustJSON(t, w, &orbat)
	totalSlots := 0
	var medicSlotID string
	for _, sq := range orbat.Data {
		totalSlots += sq.Total
		for _, s := range sq.Slots {
			if s.Role == "Combat Medic" {
				medicSlotID = s.ID
			}
		}
	}
	if totalSlots != 2 || len(orbat.Data) != 2 || medicSlotID == "" {
		t.Fatalf("unexpected ORBAT: squads=%d slots=%d medic=%q", len(orbat.Data), totalSlots, medicSlotID)
	}

	// --- u1 claims the medic slot -> registered with slot ---
	w = do(r, "POST", "/api/v1/event-missions/"+emid+"/register", reqOpt{bearer: t1, body: fmt.Sprintf(`{"slot_id":%q}`, medicSlotID)})
	if w.Code != http.StatusOK {
		t.Fatalf("u1 claim slot = %d, body=%s", w.Code, w.Body.String())
	}
	var reg1 struct {
		State string `json:"state"`
	}
	mustJSON(t, w, &reg1)
	if reg1.State != "registered" {
		t.Fatalf("u1 state = %q, want registered", reg1.State)
	}

	// --- u2 registers without a slot -> still within capacity (registered) ---
	if w := do(r, "POST", "/api/v1/event-missions/"+emid+"/register", reqOpt{bearer: t2}); w.Code != http.StatusOK {
		t.Fatalf("u2 register = %d", w.Code)
	}

	// --- u3 registers -> capacity (2 slots) reached -> waitlisted ---
	w = do(r, "POST", "/api/v1/event-missions/"+emid+"/register", reqOpt{bearer: t3})
	var reg3 struct {
		State string `json:"state"`
	}
	mustJSON(t, w, &reg3)
	if reg3.State != "waitlisted" {
		t.Fatalf("u3 state = %q, want waitlisted", reg3.State)
	}

	// --- u2 withdraws -> u3 promoted from waitlist to registered ---
	if w := do(r, "DELETE", "/api/v1/event-missions/"+emid+"/register", reqOpt{bearer: t2}); w.Code != http.StatusOK {
		t.Fatalf("u2 withdraw = %d", w.Code)
	}
	var promoted models.EventRegistration
	gdb.First(&promoted, "event_mission_id = ? AND discord_id = ?", em.ID, u3)
	if promoted.State != models.RegRegistered {
		t.Fatalf("u3 not promoted: state=%q", promoted.State)
	}

	// --- event hub shows the mission dossier with the caller's state + fill ---
	w = do(r, "GET", "/api/v1/events/"+eid, reqOpt{bearer: t1})
	var hub struct {
		Missions []struct {
			Filled  int    `json:"filled"`
			Total   int    `json:"total"`
			MyState string `json:"my_state"`
		} `json:"missions"`
	}
	mustJSON(t, w, &hub)
	if len(hub.Missions) != 1 || hub.Missions[0].Filled != 1 || hub.Missions[0].Total != 2 || hub.Missions[0].MyState != "registered" {
		t.Fatalf("hub dossier = %+v, want 1 mission filled=1 total=2 my_state=registered", hub.Missions)
	}

	// --- admin assigns u3 to the Platoon Lead slot ---
	var plSlot models.OrbatSlot
	gdb.First(&plSlot, "event_mission_id = ? AND role = ?", em.ID, "Platoon Lead")
	if w := do(r, "PUT", "/api/v1/event-missions/"+emid+"/slots/"+plSlot.ID.String()+"/assign", reqOpt{bearer: adminTok, body: fmt.Sprintf(`{"discord_id":%q}`, u3)}); w.Code != http.StatusOK {
		t.Fatalf("assign slot = %d, body=%s", w.Code, w.Body.String())
	}

	// --- My Deployments shows u1's assigned medic slot as a badge ---
	w = do(r, "GET", "/api/v1/me/deployments", reqOpt{bearer: t1})
	var dep struct {
		Upcoming []deploymentUpcoming `json:"upcoming"`
	}
	mustJSON(t, w, &dep)
	if len(dep.Upcoming) != 1 || dep.Upcoming[0].Role != "Combat Medic" || dep.Upcoming[0].Squad != "Alpha 1-1" {
		t.Fatalf("deployment badge wrong: %+v", dep.Upcoming)
	}

	// --- registration on a locked event is rejected for non-admins ---
	if w := do(r, "PATCH", "/api/v1/events/"+eid, reqOpt{bearer: adminTok, body: `{"registration_locked":true}`}); w.Code != http.StatusOK {
		t.Fatalf("lock event = %d", w.Code)
	}
	newUser := fmt.Sprintf("itest-ev4-%d", time.Now().UnixNano())
	gdb.Create(&models.User{DiscordID: newUser, Username: "Late Larry", Role: models.RoleEnlisted})
	defer gdb.Unscoped().Where("discord_id = ?", newUser).Delete(&models.User{})
	t4, _, _ := h.JWT().IssueAccess(newUser, "enlisted", false)
	if w := do(r, "POST", "/api/v1/event-missions/"+emid+"/register", reqOpt{bearer: t4}); w.Code != http.StatusForbidden {
		t.Fatalf("register on locked event = %d, want 403", w.Code)
	}

	// --- LOA: submit, list own, admin review ---
	loaBody := `{"starts_on":"2026-07-01","ends_on":"2026-07-14","reason":"Deployment IRL"}`
	w = do(r, "POST", "/api/v1/me/leave-requests", reqOpt{bearer: t1, body: loaBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("submit LOA = %d, body=%s", w.Code, w.Body.String())
	}
	var loa models.LeaveRequest
	mustJSON(t, w, &loa)

	w = do(r, "GET", "/api/v1/me/leave-requests", reqOpt{bearer: t1})
	var myLoa struct {
		Data []models.LeaveRequest `json:"data"`
	}
	mustJSON(t, w, &myLoa)
	if len(myLoa.Data) != 1 {
		t.Fatalf("expected 1 LOA, got %d", len(myLoa.Data))
	}

	if w := do(r, "PATCH", "/api/v1/admin/leave-requests/"+loa.ID.String(), reqOpt{bearer: adminTok, body: `{"status":"approved"}`}); w.Code != http.StatusOK {
		t.Fatalf("review LOA = %d, body=%s", w.Code, w.Body.String())
	}
	var reviewed models.LeaveRequest
	gdb.First(&reviewed, "id = ?", loa.ID)
	if reviewed.Status != models.LeaveApproved {
		t.Fatalf("LOA status = %q, want approved", reviewed.Status)
	}
}
