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
	uL := fmt.Sprintf("itest-evL-%d", time.Now().UnixNano())
	allUsers := []string{adminID, u1, u2, u3, uL}

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
				gdb.Where("event_mission_id = ?", em.ID).Delete(&models.OrbatReservation{})
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
	gdb.Create(&models.User{DiscordID: uL, Username: "Lead Lucy", Role: models.RoleLeader})
	gdb.Create(&mission)

	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	t1, _, _ := h.JWT().IssueAccess(u1, "enlisted", false)
	t2, _, _ := h.JWT().IssueAccess(u2, "enlisted", false)
	t3, _, _ := h.JWT().IssueAccess(u3, "enlisted", false)
	tL, _, _ := h.JWT().IssueAccess(uL, "leader", false)

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
			{"faction":"US Army","callsign":"Platoon HQ","squad":"HQ","slots":[
				{"role":"Platoon Lead","loadout":"M4A1 + M203"}
			]},
			{"faction":"US Army","squad":"Alpha 1-1","slots":[
				{"role":"Combat Medic","loadout":"M4A1","tag":"MED"}
			]}
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
	var medic orbatSlotDTO
	for _, sq := range orbat.Data {
		totalSlots += sq.Total
		for _, s := range sq.Slots {
			if s.Role == "Combat Medic" {
				medic = s
			}
		}
	}
	if totalSlots != 2 || len(orbat.Data) != 2 || medic.ID == "" {
		t.Fatalf("unexpected ORBAT: squads=%d slots=%d medic=%q", len(orbat.Data), totalSlots, medic.ID)
	}
	// Per-slot detail parsed from the mission.json: number, loadout, tag.
	if medic.Number != 1 || medic.Loadout != "M4A1" || medic.Tag != "MED" {
		t.Fatalf("medic slot detail wrong: number=%d loadout=%q tag=%q", medic.Number, medic.Loadout, medic.Tag)
	}
	medicSlotID := medic.ID

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

	// --- squad reservation: a leader holds a squad in one click ---
	// Use a second operation with a 2-slot squad so the hold flow is isolated.
	w = do(r, "POST", "/api/v1/events", reqOpt{bearer: adminTok, body: fmt.Sprintf(`{"start_time":%q,"name_override":"Reserve Test Op"}`, start)})
	var event2 models.Event
	mustJSON(t, w, &event2)
	addBody2 := fmt.Sprintf(`{"mission_id":%q,"start_time":%q,"orbat":[
		{"faction":"BAF","callsign":"Bulldog","squad":"Bravo 2","slots":[
			{"role":"Squad Leader","loadout":"L85A3 + GL"},
			{"role":"Medic","loadout":"L85A3","tag":"MED"}
		]}]}`, mission.ID.String(), missionStart)
	w = do(r, "POST", "/api/v1/events/"+event2.ID.String()+"/missions", reqOpt{bearer: adminTok, body: addBody2})
	if w.Code != http.StatusCreated {
		t.Fatalf("attach reserve-test mission = %d, body=%s", w.Code, w.Body.String())
	}
	var em2 models.EventMission
	mustJSON(t, w, &em2)
	emid2 := em2.ID.String()

	// Slot ids for Bravo 2.
	w = do(r, "GET", "/api/v1/event-missions/"+emid2+"/orbat", reqOpt{bearer: tL})
	var orbat2 struct {
		Data []orbatSquadDTO `json:"data"`
	}
	mustJSON(t, w, &orbat2)
	if len(orbat2.Data) != 1 {
		t.Fatalf("reserve-test ORBAT squads = %d, want 1", len(orbat2.Data))
	}
	slIDs := []string{orbat2.Data[0].Slots[0].ID, orbat2.Data[0].Slots[1].ID}

	// An enlisted member cannot reserve (needs leader role).
	if w := do(r, "POST", "/api/v1/event-missions/"+emid2+"/squads/reserve", reqOpt{bearer: t2, body: `{"squad":"Bravo 2"}`}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted reserve = %d, want 403", w.Code)
	}
	// The leader reserves the whole squad.
	if w := do(r, "POST", "/api/v1/event-missions/"+emid2+"/squads/reserve", reqOpt{bearer: tL, body: `{"squad":"Bravo 2"}`}); w.Code != http.StatusCreated {
		t.Fatalf("leader reserve = %d, body=%s", w.Code, w.Body.String())
	}
	// Reservation is surfaced on the ORBAT.
	w = do(r, "GET", "/api/v1/event-missions/"+emid2+"/orbat", reqOpt{bearer: t2})
	mustJSON(t, w, &orbat2)
	if orbat2.Data[0].ReservedBy != uL {
		t.Fatalf("squad reserved_by = %q, want %q", orbat2.Data[0].ReservedBy, uL)
	}
	// A non-reserver can no longer claim a slot in the held squad.
	if w := do(r, "POST", "/api/v1/event-missions/"+emid2+"/register", reqOpt{bearer: t2, body: fmt.Sprintf(`{"slot_id":%q}`, slIDs[0])}); w.Code != http.StatusConflict {
		t.Fatalf("claim in reserved squad = %d, want 409", w.Code)
	}
	// The leader fills the squad by assigning a member.
	if w := do(r, "PUT", "/api/v1/event-missions/"+emid2+"/slots/"+slIDs[0]+"/assign", reqOpt{bearer: tL, body: fmt.Sprintf(`{"discord_id":%q}`, u2)}); w.Code != http.StatusOK {
		t.Fatalf("leader assign in reserved squad = %d, body=%s", w.Code, w.Body.String())
	}
	// Member search backs the leader's assignee picker.
	w = do(r, "GET", "/api/v1/members?q=Player+Two", reqOpt{bearer: tL})
	var members struct {
		Data []memberDTO `json:"data"`
	}
	mustJSON(t, w, &members)
	if len(members.Data) == 0 {
		t.Fatalf("member search returned no results")
	}
	// Release reopens the squad for self-registration.
	if w := do(r, "POST", "/api/v1/event-missions/"+emid2+"/squads/release", reqOpt{bearer: tL, body: `{"squad":"Bravo 2"}`}); w.Code != http.StatusOK {
		t.Fatalf("leader release = %d, body=%s", w.Code, w.Body.String())
	}
	if w := do(r, "POST", "/api/v1/event-missions/"+emid2+"/register", reqOpt{bearer: t3, body: fmt.Sprintf(`{"slot_id":%q}`, slIDs[1])}); w.Code != http.StatusOK {
		t.Fatalf("claim after release = %d, body=%s", w.Code, w.Body.String())
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
