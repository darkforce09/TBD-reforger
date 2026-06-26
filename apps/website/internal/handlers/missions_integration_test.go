package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestMissionLifecycleIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	makerID := fmt.Sprintf("itest-mm-%d", time.Now().UnixNano())
	adminID := fmt.Sprintf("itest-adm-%d", time.Now().UnixNano())
	otherID := fmt.Sprintf("itest-oth-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		// Missions cascade to versions/armory/bookmarks via FK-less manual cleanup.
		var ms []models.Mission
		gdb.Unscoped().Where("author_id = ?", makerID).Find(&ms)
		for _, m := range ms {
			gdb.Where("mission_id = ?", m.ID).Delete(&models.MissionVersion{})
			gdb.Where("mission_id = ?", m.ID).Delete(&models.MissionArmory{})
			gdb.Where("mission_id = ?", m.ID).Delete(&models.MissionBookmark{})
		}
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Where("actor_id IN ?", []string{adminID}).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id IN ?", []string{makerID, adminID, otherID}).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: makerID, Username: "Maker Mike", Role: models.RoleMissionMaker})
	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Dave", Role: models.RoleAdmin})
	gdb.Create(&models.User{DiscordID: otherID, Username: "Other Otto", Role: models.RoleEnlisted})
	makerTok, _, _ := h.JWT().IssueAccess(makerID, "mission_maker", false)
	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	otherTok, _, _ := h.JWT().IssueAccess(otherID, "enlisted", false)

	// --- authz: enlisted cannot create missions ---
	if w := do(r, "POST", "/api/v1/missions", reqOpt{bearer: otherTok, body: `{"title":"x","terrain":"everon","game_mode":"pve_coop","max_players":8}`}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted create = %d, want 403", w.Code)
	}

	// --- create (Creator wizard) ---
	createBody := `{"title":"Operation Enduring Freedom","terrain":"everon","game_mode":"pve_coop","weather":"overcast","time_of_day":"14:30","max_players":64,"briefing":"Secure the relay."}`
	w := do(r, "POST", "/api/v1/missions", reqOpt{bearer: makerTok, body: createBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("create = %d, body=%s", w.Code, w.Body.String())
	}
	var mission models.Mission
	mustJSON(t, w, &mission)
	if mission.Status != models.MissionDraft || mission.CurrentVersionID == nil {
		t.Fatalf("expected draft with initial version, got %+v", mission)
	}
	mid := mission.ID.String()

	// --- a non-author cannot edit ---
	if w := do(r, "PATCH", "/api/v1/missions/"+mid, reqOpt{bearer: otherTok, body: `{"title":"hax"}`}); w.Code != http.StatusForbidden {
		t.Fatalf("non-author PATCH = %d, want 403", w.Code)
	}

	// --- author saves a new version (2D editor output) ---
	verBody := `{"semver":"1.0.0","payload":{"spawns":[{"x":1,"y":2}],"markers":[]},"editor_notes":"first pass"}`
	w = do(r, "POST", "/api/v1/missions/"+mid+"/versions", reqOpt{bearer: makerTok, body: verBody})
	if w.Code != http.StatusCreated {
		t.Fatalf("create version = %d, body=%s", w.Code, w.Body.String())
	}

	// --- author sets the armory ---
	armoryBody := `{"items":[{"faction":"US Forces","category":"weapon","item_name":"M16A2 Rifle","quantity":45,"sort_order":0},{"faction":"US Forces","category":"vehicle","item_name":"M113 APC","quantity":2,"sort_order":1}]}`
	if w := do(r, "PUT", "/api/v1/missions/"+mid+"/armory", reqOpt{bearer: makerTok, body: armoryBody}); w.Code != http.StatusOK {
		t.Fatalf("set armory = %d, body=%s", w.Code, w.Body.String())
	}

	// --- submit for approval ---
	if w := do(r, "POST", "/api/v1/missions/"+mid+"/submit", reqOpt{bearer: makerTok}); w.Code != http.StatusOK {
		t.Fatalf("submit = %d, body=%s", w.Code, w.Body.String())
	}

	// --- another user must NOT yet see the pending mission in the global library ---
	w = do(r, "GET", "/api/v1/missions?scope=global&terrain=everon", reqOpt{bearer: otherTok})
	if containsMission(t, w, mid) {
		t.Fatal("pending mission should not appear in another user's global library")
	}

	// --- but the author DOES see their own pending mission in global (mine ⊆ global) ---
	w = do(r, "GET", "/api/v1/missions?scope=global&terrain=everon", reqOpt{bearer: makerTok})
	if !containsMission(t, w, mid) {
		t.Fatal("author should see their own pending mission in the global library")
	}

	// --- it appears in the admin approvals queue ---
	w = do(r, "GET", "/api/v1/approvals", reqOpt{bearer: adminTok})
	var queue struct {
		Data []approvalRow `json:"data"`
	}
	mustJSON(t, w, &queue)
	found := false
	for _, row := range queue.Data {
		if row.MissionID == mid {
			found = true
			if row.AuthorName != "Maker Mike" {
				t.Errorf("approval author = %q", row.AuthorName)
			}
		}
	}
	if !found {
		t.Fatal("submitted mission missing from approvals queue")
	}

	// --- enlisted cannot approve ---
	if w := do(r, "POST", "/api/v1/approvals/"+mid+"/approve", reqOpt{bearer: otherTok}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted approve = %d, want 403", w.Code)
	}

	// --- admin approves -> mission goes live ---
	if w := do(r, "POST", "/api/v1/approvals/"+mid+"/approve", reqOpt{bearer: adminTok}); w.Code != http.StatusOK {
		t.Fatalf("approve = %d, body=%s", w.Code, w.Body.String())
	}
	var critCount int64
	gdb.Model(&models.AuditLog{}).Where("action = ? AND target_id = ?", "mission.approve", mid).Count(&critCount)
	if critCount < 1 {
		t.Fatal("expected an audit row for mission approval")
	}

	// --- now it shows in the global library ---
	w = do(r, "GET", "/api/v1/missions?scope=global&terrain=everon", reqOpt{bearer: otherTok})
	if !containsMission(t, w, mid) {
		t.Fatal("approved mission should appear in global library")
	}

	// --- player_count filter excludes it from the 1-16 bucket ---
	w = do(r, "GET", "/api/v1/missions?scope=global&player_count=1-16", reqOpt{bearer: otherTok})
	if containsMission(t, w, mid) {
		t.Fatal("64-player mission should be excluded from 1-16 bucket")
	}

	// --- bookmark, then it shows under the bookmarked scope ---
	if w := do(r, "POST", "/api/v1/missions/"+mid+"/bookmark", reqOpt{bearer: otherTok}); w.Code != http.StatusOK {
		t.Fatalf("bookmark = %d", w.Code)
	}
	w = do(r, "GET", "/api/v1/missions?scope=bookmarked", reqOpt{bearer: otherTok})
	if !containsMission(t, w, mid) {
		t.Fatal("mission should appear under bookmarked scope")
	}

	// --- export strict mission.json (mission_maker) ---
	w = do(r, "GET", "/api/v1/missions/"+mid+"/export", reqOpt{bearer: makerTok})
	if w.Code != http.StatusOK {
		t.Fatalf("export = %d", w.Code)
	}
	if cd := w.Header().Get("Content-Disposition"); cd == "" {
		t.Error("export missing Content-Disposition header")
	}
	var doc missionJSON
	mustJSON(t, w, &doc)
	if doc.SchemaVersion != 1 || doc.Version != "1.0.0" || doc.MaxPlayers != 64 || len(doc.Armory) != 2 {
		t.Fatalf("unexpected mission.json: %+v", doc)
	}

	// --- overview returns armory + current version ---
	w = do(r, "GET", "/api/v1/missions/"+mid, reqOpt{bearer: otherTok})
	var detail missionDetail
	mustJSON(t, w, &detail)
	if len(detail.Armory) != 2 || detail.CurrentVersion == nil || detail.CurrentVersion.Semver != "1.0.0" {
		t.Fatalf("overview missing armory/version: %+v", detail)
	}
	if !detail.Bookmarked {
		t.Error("overview should show bookmarked=true for this viewer")
	}

	// --- a version payload larger than the old 1 MB global cap still round-trips ---
	// (T-060). CreateVersion itself has no size ceiling; the body cap is purely the
	// middleware.BodyLimit on the route (256 MB by default). The oversize/413 path is
	// middleware-level — exercised manually against the running stack, not here. Run
	// last so making this the current version doesn't disturb the 1.0.0 assertions above.
	bigNote := strings.Repeat("x", 2<<20) // ~2 MB, comfortably over the old 1 MB default
	bigBody := `{"semver":"1.1.0","payload":{"spawns":[]},"editor_notes":"` + bigNote + `"}`
	if w := do(r, "POST", "/api/v1/missions/"+mid+"/versions", reqOpt{bearer: makerTok, body: bigBody}); w.Code != http.StatusCreated {
		t.Fatalf("create large (~2MB) version = %d (want 201), body len=%d", w.Code, w.Body.Len())
	}
}

// containsMission reports whether a /missions list response includes the id.
func containsMission(t *testing.T, w *httptest.ResponseRecorder, id string) bool {
	t.Helper()
	var lr struct {
		Data []struct {
			ID string `json:"id"`
		} `json:"data"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &lr); err != nil {
		t.Fatalf("decode list: %v (body=%s)", err, w.Body.String())
	}
	for _, m := range lr.Data {
		if m.ID == id {
			return true
		}
	}
	return false
}
