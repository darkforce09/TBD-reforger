package handlers

import (
	"fmt"
	"net/http"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/config"
	"github.com/tbd-milsim/reforger-backend/internal/db"
	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// setupITProd mirrors setupIT but mounts the *production* cross-cutting middleware
// that matters for body size — GlobalBodyLimit(MaxJSONBody) on the root engine, exactly
// like cmd/api/main.go. The plain setupIT registers routes on a bare gin.New(), so the
// version route's interaction with the 1 MB global cap was never exercised in CI — the
// blind spot behind the T-060.1.4 mid-upload reset. routeCapBytes pins the version
// route's own BodyLimit so the over-cap path is testable without allocating 256 MB.
func setupITProd(t *testing.T, routeCapBytes int) (*gin.Engine, *Handler, *gorm.DB) {
	t.Helper()
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set; skipping DB integration test")
	}
	gdb, err := db.Open(dsn, false)
	if err != nil {
		t.Fatalf("open db: %v", err)
	}
	if err := db.Migrate(gdb); err != nil {
		t.Fatalf("migrate: %v", err)
	}
	cfg := &config.Config{
		JWTSecret:                  "itest-secret",
		JWTAccessTTLMin:            15,
		ServiceToken:               "svc-token",
		MissionVersionMaxBodyBytes: routeCapBytes,
	}
	h := New(gdb, cfg)
	gin.SetMode(gin.TestMode)
	r := gin.New()
	// The one piece of the production chain that governs body size (see main.go).
	r.Use(middleware.GlobalBodyLimit(middleware.MaxJSONBody))
	h.Register(r.Group("/api/v1"))
	return r, h, gdb
}

// TestMissionVersionBodyLimitProd proves the large mission-version POST is NOT capped
// at the 1 MB global default in the real middleware stack, and that an over-cap body
// returns a clean 413 JSON. Regression guard for T-060.1.4 (mid-upload ERR_NETWORK).
func TestMissionVersionBodyLimitProd(t *testing.T) {
	// 4 MB route cap so the over-limit case is cheap; well above the 1 MB global default
	// so the "global must skip this route" property is still under test.
	const routeCap = 4 << 20
	r, h, gdb := setupITProd(t, routeCap)

	makerID := fmt.Sprintf("itest-bl-mm-%d", time.Now().UnixNano())
	t.Cleanup(func() {
		var ms []models.Mission
		gdb.Unscoped().Where("author_id = ?", makerID).Find(&ms)
		for _, m := range ms {
			gdb.Where("mission_id = ?", m.ID).Delete(&models.MissionVersion{})
		}
		gdb.Unscoped().Where("author_id = ?", makerID).Delete(&models.Mission{})
		gdb.Unscoped().Where("discord_id = ?", makerID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: makerID, Username: "Body Limit Maker", Role: models.RoleMissionMaker})
	makerTok, _, _ := h.JWT().IssueAccess(makerID, "mission_maker", false)

	// Create a mission to own the versions.
	w := do(r, "POST", "/api/v1/missions",
		reqOpt{bearer: makerTok, body: `{"title":"Body Limit IT","terrain":"everon","game_mode":"pve_coop","max_players":8}`})
	if w.Code != http.StatusCreated {
		t.Fatalf("create mission = %d, body=%s", w.Code, w.Body.String())
	}
	var mission models.Mission
	mustJSON(t, w, &mission)
	mid := mission.ID.String()

	// versionBody builds a valid create-version body whose total size is ~targetBytes,
	// padding editor_notes (which is not size-validated) to hit the target.
	versionBody := func(semver string, targetBytes int) string {
		head := `{"semver":"` + semver + `","payload":{"spawns":[]},"editor_notes":"`
		tail := `"}`
		pad := targetBytes - len(head) - len(tail)
		if pad < 0 {
			pad = 0
		}
		return head + strings.Repeat("x", pad) + tail
	}

	// --- Bodies over the 1 MB global default but under the 4 MB route cap must round-trip
	//     to 201, proving the global limiter skips this route. ---

	// 2 MB: over the 1 MB global default, under the 4 MB route cap → 201.
	if w := do(r, "POST", "/api/v1/missions/"+mid+"/versions",
		reqOpt{bearer: makerTok, body: versionBody("1.0.0", 2<<20)}); w.Code != http.StatusCreated {
		t.Fatalf("2 MB version POST = %d (want 201 — global 1 MB cap must skip this route), body=%s",
			w.Code, w.Body.String())
	}

	// 3.5 MB: comfortably over the 1 MB global, still under the 4 MB route cap → 201.
	if w := do(r, "POST", "/api/v1/missions/"+mid+"/versions",
		reqOpt{bearer: makerTok, body: versionBody("1.1.0", 7<<19)}); w.Code != http.StatusCreated {
		t.Fatalf("3.5 MB version POST = %d (want 201), body=%s", w.Code, w.Body.String())
	}

	// --- Over the route cap (4 MB) → 413 JSON, not a generic 400 or silent reset. ---
	w = do(r, "POST", "/api/v1/missions/"+mid+"/versions",
		reqOpt{bearer: makerTok, body: versionBody("2.0.0", 5<<20)})
	if w.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("5 MB version POST over 4 MB route cap = %d (want 413), body=%s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), "too large") {
		t.Errorf("over-cap response should carry a 'too large' JSON error, got %s", w.Body.String())
	}
}
