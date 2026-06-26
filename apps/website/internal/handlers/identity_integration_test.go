package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/config"
	"github.com/tbd-milsim/reforger-backend/internal/db"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// setupIT spins the identity router against a real Postgres. Skips unless
// TEST_DATABASE_URL is set (e.g. the local validation container).
func setupIT(t *testing.T) (*gin.Engine, *Handler, *gorm.DB) {
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
	cfg := &config.Config{JWTSecret: "itest-secret", JWTAccessTTLMin: 15, ServiceToken: "svc-token"}
	h := New(gdb, cfg)
	gin.SetMode(gin.TestMode)
	r := gin.New()
	h.Register(r.Group("/api/v1"))
	return r, h, gdb
}

type reqOpt struct {
	bearer  string
	service string
	body    string
}

func do(r http.Handler, method, path string, opt reqOpt) *httptest.ResponseRecorder {
	var body *strings.Reader
	if opt.body != "" {
		body = strings.NewReader(opt.body)
	} else {
		body = strings.NewReader("")
	}
	req := httptest.NewRequest(method, path, body)
	if opt.body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	if opt.bearer != "" {
		req.Header.Set("Authorization", "Bearer "+opt.bearer)
	}
	if opt.service != "" {
		req.Header.Set("X-Service-Token", opt.service)
	}
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func TestIdentityFlowIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)
	discordID := fmt.Sprintf("itest-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.IdentityLinkCode{})
		gdb.Where("discord_id = ?", discordID).Delete(&models.RefreshToken{})
		gdb.Where("actor_id = ?", discordID).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	if err := gdb.Create(&models.User{DiscordID: discordID, Username: "ITest", Role: models.RoleEnlisted}).Error; err != nil {
		t.Fatalf("seed user: %v", err)
	}
	token, _, err := h.JWT().IssueAccess(discordID, "enlisted", false)
	if err != nil {
		t.Fatalf("mint token: %v", err)
	}

	// GET /me — authenticated, not yet linked.
	w := do(r, "GET", "/api/v1/me", reqOpt{bearer: token})
	if w.Code != http.StatusOK {
		t.Fatalf("GET /me = %d, body=%s", w.Code, w.Body.String())
	}
	var me struct {
		ArmaLinked bool `json:"arma_linked"`
	}
	mustJSON(t, w, &me)
	if me.ArmaLinked {
		t.Fatal("expected arma_linked=false initially")
	}

	// GET /me without token — 401.
	if w := do(r, "GET", "/api/v1/me", reqOpt{}); w.Code != http.StatusUnauthorized {
		t.Fatalf("unauthenticated GET /me = %d, want 401", w.Code)
	}

	// POST /me/link — issue a 6-digit code.
	w = do(r, "POST", "/api/v1/me/link", reqOpt{bearer: token})
	if w.Code != http.StatusCreated {
		t.Fatalf("POST /me/link = %d, body=%s", w.Code, w.Body.String())
	}
	var lc struct {
		Code string `json:"code"`
	}
	mustJSON(t, w, &lc)
	if len(lc.Code) != 6 {
		t.Fatalf("link code = %q, want 6 digits", lc.Code)
	}

	// Ingest with wrong service token — 401.
	body := fmt.Sprintf(`{"code":%q,"arma_id":"steam-123","arma_character":"[TBD] ITest"}`, lc.Code)
	if w := do(r, "POST", "/api/v1/ingest/link-confirm", reqOpt{service: "bad", body: body}); w.Code != http.StatusUnauthorized {
		t.Fatalf("ingest with bad service token = %d, want 401", w.Code)
	}

	// Ingest with correct service token — links the identity.
	w = do(r, "POST", "/api/v1/ingest/link-confirm", reqOpt{service: "svc-token", body: body})
	if w.Code != http.StatusOK {
		t.Fatalf("ingest link-confirm = %d, body=%s", w.Code, w.Body.String())
	}

	// Status now reports linked.
	w = do(r, "GET", "/api/v1/me/link/status", reqOpt{bearer: token})
	var st struct {
		Linked        bool   `json:"linked"`
		ArmaCharacter string `json:"arma_character"`
	}
	mustJSON(t, w, &st)
	if !st.Linked || st.ArmaCharacter != "[TBD] ITest" {
		t.Fatalf("status after link = %+v, want linked with character", st)
	}

	// Re-using the consumed code fails.
	if w := do(r, "POST", "/api/v1/ingest/link-confirm", reqOpt{service: "svc-token", body: body}); w.Code != http.StatusNotFound {
		t.Fatalf("reused code = %d, want 404", w.Code)
	}
}

func TestRoleSyncIntegration(t *testing.T) {
	_, _, gdb := setupIT(t)
	roleID := fmt.Sprintf("r-admin-%d", time.Now().UnixNano())
	discordID := fmt.Sprintf("itest-rs-%d", time.Now().UnixNano())
	adminRole := models.RoleAdmin

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.UserDiscordRole{})
		gdb.Where("discord_role_id = ?", roleID).Delete(&models.DiscordRole{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	if err := gdb.Create(&models.DiscordRole{DiscordRoleID: roleID, Name: "Admin", MappedRole: &adminRole, Priority: 10}).Error; err != nil {
		t.Fatalf("seed discord role: %v", err)
	}
	if err := gdb.Create(&models.User{DiscordID: discordID, Username: "RS", Role: models.RoleEnlisted}).Error; err != nil {
		t.Fatalf("seed user: %v", err)
	}

	// One mapped role + one unmapped/unknown role -> resolves to admin, and
	// both IDs are stored for future remapping.
	resolved, err := services.SyncRoles(gdb, discordID, []string{roleID, "r-unknown"})
	if err != nil {
		t.Fatalf("SyncRoles: %v", err)
	}
	if resolved != models.RoleAdmin {
		t.Fatalf("resolved role = %q, want admin", resolved)
	}
	var stored int64
	gdb.Model(&models.UserDiscordRole{}).Where("discord_id = ?", discordID).Count(&stored)
	if stored != 2 {
		t.Fatalf("stored discord roles = %d, want 2", stored)
	}
}

func mustJSON(t *testing.T, w *httptest.ResponseRecorder, v any) {
	t.Helper()
	if err := json.Unmarshal(w.Body.Bytes(), v); err != nil {
		t.Fatalf("decode body %q: %v", w.Body.String(), err)
	}
}
