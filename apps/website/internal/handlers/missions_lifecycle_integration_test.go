package handlers

import (
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// seedLifecycleMission creates a maker user + draft mission directly and returns
// (missionID, makerToken, makerID).
func seedLifecycleMission(t *testing.T, h *Handler) (string, string, string) {
	t.Helper()
	makerID := fmt.Sprintf("itest-arc-%d", time.Now().UnixNano())
	if err := h.db.Create(&models.User{DiscordID: makerID, Username: "Archive Al", Role: models.RoleMissionMaker}).Error; err != nil {
		t.Fatalf("seed user: %v", err)
	}
	m := models.Mission{
		Title: "Lifecycle Target", AuthorID: makerID, Terrain: models.TerrainEveron,
		GameMode: models.GameModePvECoop, Weather: models.WeatherClear,
		TimeOfDay: "14:00", MaxPlayers: 8, Status: models.MissionDraft,
	}
	if err := h.db.Create(&m).Error; err != nil {
		t.Fatalf("seed mission: %v", err)
	}
	tok, _, _ := h.JWT().IssueAccess(makerID, "mission_maker", false)
	return m.ID.String(), tok, makerID
}

// TestMissionArchiveLifecycle proves T-130.6 F2B-05: PATCH status=archived hides the
// mission from the global scope (still listed under mine, badged), unarchive returns
// it to draft, and other status values are rejected.
func TestMissionArchiveLifecycle(t *testing.T) {
	r, h, gdb := setupIT(t)
	mid, tok, makerID := seedLifecycleMission(t, h)
	t.Cleanup(func() {
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id = ?", makerID).Delete(&models.User{})
	})

	// Archive.
	w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"archived"}`})
	if w.Code != http.StatusOK {
		t.Fatalf("archive = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	var patched models.Mission
	mustJSON(t, w, &patched)
	if patched.Status != models.MissionArchived {
		t.Fatalf("status = %s, want archived", patched.Status)
	}

	// Global scope must not list it; mine must.
	var list struct {
		Data []models.Mission `json:"data"`
	}
	w = do(r, "GET", "/api/v1/missions?scope=global", reqOpt{bearer: tok})
	mustJSON(t, w, &list)
	for _, m := range list.Data {
		if m.ID.String() == mid {
			t.Fatal("archived mission leaked into global scope")
		}
	}
	w = do(r, "GET", "/api/v1/missions?scope=mine", reqOpt{bearer: tok})
	mustJSON(t, w, &list)
	found := false
	for _, m := range list.Data {
		if m.ID.String() == mid {
			found = true
		}
	}
	if !found {
		t.Fatal("archived mission missing from mine scope")
	}

	// Only archive/unarchive transitions are allowed through PATCH.
	if w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"live"}`}); w.Code != http.StatusBadRequest {
		t.Fatalf("status=live via PATCH = %d, want 400", w.Code)
	}

	// Unarchive → draft.
	w = do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"draft"}`})
	if w.Code != http.StatusOK {
		t.Fatalf("unarchive = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	mustJSON(t, w, &patched)
	if patched.Status != models.MissionDraft {
		t.Fatalf("status after unarchive = %s, want draft", patched.Status)
	}
	// Unarchive is only valid from archived.
	if w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"draft"}`}); w.Code != http.StatusOK {
		t.Fatalf("idempotent draft->draft = %d, want 200 (no-op)", w.Code)
	}
}

// TestMissionArchiveBlockedByUpcomingEvent proves the 409 guard: a mission attached to
// an upcoming event cannot be archived until it is detached.
func TestMissionArchiveBlockedByUpcomingEvent(t *testing.T) {
	r, h, gdb := setupIT(t)
	mid, tok, makerID := seedLifecycleMission(t, h)
	ev := models.Event{StartTime: time.Now().Add(48 * time.Hour), Status: models.EventScheduled}
	if err := gdb.Create(&ev).Error; err != nil {
		t.Fatalf("seed event: %v", err)
	}
	var m models.Mission
	gdb.First(&m, "id = ?", mid)
	em := models.EventMission{EventID: ev.ID, MissionID: m.ID, StartTime: ev.StartTime}
	if err := gdb.Create(&em).Error; err != nil {
		t.Fatalf("seed event mission: %v", err)
	}
	t.Cleanup(func() {
		gdb.Where("event_id = ?", ev.ID).Delete(&models.EventMission{})
		gdb.Where("id = ?", ev.ID).Delete(&models.Event{})
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id = ?", makerID).Delete(&models.User{})
	})

	if w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"archived"}`}); w.Code != http.StatusConflict {
		t.Fatalf("archive with upcoming event = %d, want 409 (body=%s)", w.Code, w.Body.String())
	}

	// Delete is blocked by ANY attachment, upcoming or past.
	if w := do(r, "DELETE", "/api/v1/missions/"+mid, reqOpt{bearer: tok}); w.Code != http.StatusConflict {
		t.Fatalf("delete with event attachment = %d, want 409 (body=%s)", w.Code, w.Body.String())
	}

	// Detach → archive succeeds.
	gdb.Where("event_id = ?", ev.ID).Delete(&models.EventMission{})
	if w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: tok, body: `{"status":"archived"}`}); w.Code != http.StatusOK {
		t.Fatalf("archive after detach = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
}

// TestMissionSoftDelete proves T-130.6 F2B-05: DELETE soft-deletes (404 afterwards,
// row retained with deleted_at), and a non-author cannot delete.
func TestMissionSoftDelete(t *testing.T) {
	r, h, gdb := setupIT(t)
	mid, tok, makerID := seedLifecycleMission(t, h)
	otherID := fmt.Sprintf("itest-arc-oth-%d", time.Now().UnixNano())
	gdb.Create(&models.User{DiscordID: otherID, Username: "Other Ola", Role: models.RoleMissionMaker})
	otherTok, _, _ := h.JWT().IssueAccess(otherID, "mission_maker", false)
	t.Cleanup(func() {
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id IN ?", []string{makerID, otherID}).Delete(&models.User{})
	})

	// A different mission_maker is not the author → 403.
	if w := do(r, "DELETE", "/api/v1/missions/"+mid, reqOpt{bearer: otherTok}); w.Code != http.StatusForbidden {
		t.Fatalf("non-author delete = %d, want 403", w.Code)
	}

	if w := do(r, "DELETE", "/api/v1/missions/"+mid, reqOpt{bearer: tok}); w.Code != http.StatusNoContent {
		t.Fatalf("delete = %d, want 204 (body=%s)", w.Code, w.Body.String())
	}

	// Gone from the API…
	if w := do(r, "GET", "/api/v1/missions/"+mid, reqOpt{bearer: tok}); w.Code != http.StatusNotFound {
		t.Fatalf("get after delete = %d, want 404", w.Code)
	}
	// …but the row is retained (soft delete) for operator recovery.
	var count int64
	gdb.Unscoped().Model(&models.Mission{}).Where("id = ? AND deleted_at IS NOT NULL", mid).Count(&count)
	if count != 1 {
		t.Fatalf("expected soft-deleted row retained, got %d", count)
	}
}
