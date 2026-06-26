package handlers

import (
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/config"
	"github.com/tbd-milsim/reforger-backend/internal/db"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// setupDevIT spins the router with Env=development so the dev-login route is
// registered (the shared setupIT leaves Env empty, gating the route off).
// Skips unless TEST_DATABASE_URL is set, since DevLogin writes to the DB.
func setupDevIT(t *testing.T) (*gin.Engine, *Handler, *gorm.DB) {
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
		Env:             "development",
		FrontendURL:     "http://frontend.test",
		JWTSecret:       "itest-secret",
		JWTAccessTTLMin: 15,
		ServiceToken:    "svc-token",
	}
	h := New(gdb, cfg)
	gin.SetMode(gin.TestMode)
	r := gin.New()
	h.Register(r.Group("/api/v1"))
	return r, h, gdb
}

func TestDevLoginRedirectsToSPA(t *testing.T) {
	r, _, gdb := setupDevIT(t)

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", devUserID).Delete(&models.RefreshToken{})
		gdb.Unscoped().Where("discord_id = ?", devUserID).Delete(&models.User{})
	})

	req := httptest.NewRequest(http.MethodGet, "/api/v1/auth/dev-login?role=admin", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusFound {
		t.Fatalf("dev-login status = %d, want 302; body=%s", w.Code, w.Body.String())
	}
	loc := w.Header().Get("Location")
	if !strings.HasPrefix(loc, "http://frontend.test/auth/callback#") {
		t.Fatalf("redirect location = %q", loc)
	}
	for _, want := range []string{"access_token=", "refresh_token=", "expires_at=", "arma_linked=true"} {
		if !strings.Contains(loc, want) {
			t.Errorf("redirect missing %q: %s", want, loc)
		}
	}

	var u models.User
	if err := gdb.First(&u, "discord_id = ?", devUserID).Error; err != nil {
		t.Fatalf("dev user not persisted: %v", err)
	}
	if u.Role != models.RoleAdmin {
		t.Errorf("role = %q, want admin", u.Role)
	}
}

func TestDevLoginUnknownRoleDefaultsToAdmin(t *testing.T) {
	r, _, gdb := setupDevIT(t)

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", devUserID).Delete(&models.RefreshToken{})
		gdb.Unscoped().Where("discord_id = ?", devUserID).Delete(&models.User{})
	})

	req := httptest.NewRequest(http.MethodGet, "/api/v1/auth/dev-login?role=bogus", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusFound {
		t.Fatalf("dev-login status = %d, want 302; body=%s", w.Code, w.Body.String())
	}

	var u models.User
	if err := gdb.First(&u, "discord_id = ?", devUserID).Error; err != nil {
		t.Fatalf("dev user not persisted: %v", err)
	}
	if u.Role != models.RoleAdmin {
		t.Errorf("unknown role should default to admin, got %q", u.Role)
	}
}
