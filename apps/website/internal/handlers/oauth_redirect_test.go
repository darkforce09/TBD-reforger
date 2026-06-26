package handlers

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestAuthCallbackURL(t *testing.T) {
	got := authCallbackURL("http://localhost:5173/", url.Values{
		"access_token": {"abc"},
		"error":        {""},
	})
	if !strings.HasPrefix(got, "http://localhost:5173/auth/callback#") {
		t.Fatalf("bad base: %s", got)
	}
	if !strings.Contains(got, "access_token=abc") {
		t.Errorf("missing token in fragment: %s", got)
	}
	// Trailing slash on the frontend URL must not double up.
	if strings.Contains(got, "5173//auth") {
		t.Errorf("double slash: %s", got)
	}
}

func TestOAuthCallbackRedirectsToSPA(t *testing.T) {
	r, h, gdb := setupIT(t)
	h.cfg.FrontendURL = "http://frontend.test"

	discordID := fmt.Sprintf("itest-oauth-%d", time.Now().UnixNano())
	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.RefreshToken{})
		gdb.Where("actor_id = ?", discordID).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	// Mock the Discord endpoints used by the callback.
	mux := http.NewServeMux()
	mux.HandleFunc("/oauth2/token", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"access_token":"tok","token_type":"Bearer"}`))
	})
	mux.HandleFunc("/users/@me", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(fmt.Sprintf(`{"id":%q,"username":"OAuthUser","global_name":"OAuth User","discriminator":"0"}`, discordID)))
	})
	// Guild member fetch is allowed to 404 (tolerated -> enlisted).
	srv := httptest.NewServer(mux)
	defer srv.Close()
	h.Discord().SetAPIBase(srv.URL)

	req := httptest.NewRequest(http.MethodGet, "/api/v1/auth/discord/callback?code=good&state=xyz", nil)
	req.AddCookie(&http.Cookie{Name: "oauth_state", Value: "xyz"})
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusFound {
		t.Fatalf("callback status = %d, want 302; body=%s", w.Code, w.Body.String())
	}
	loc := w.Header().Get("Location")
	if !strings.HasPrefix(loc, "http://frontend.test/auth/callback#") {
		t.Fatalf("redirect location = %q", loc)
	}
	for _, want := range []string{"access_token=", "refresh_token=", "expires_at=", "arma_linked=false"} {
		if !strings.Contains(loc, want) {
			t.Errorf("redirect missing %q: %s", want, loc)
		}
	}

	// The user was upserted.
	var u models.User
	if err := gdb.First(&u, "discord_id = ?", discordID).Error; err != nil {
		t.Fatalf("user not persisted: %v", err)
	}
	if u.Username != "OAuth User" {
		t.Errorf("username = %q", u.Username)
	}
}

func TestOAuthCallbackBadStateRedirectsWithError(t *testing.T) {
	r, h, _ := setupIT(t)
	h.cfg.FrontendURL = "http://frontend.test"

	req := httptest.NewRequest(http.MethodGet, "/api/v1/auth/discord/callback?code=good&state=mismatch", nil)
	req.AddCookie(&http.Cookie{Name: "oauth_state", Value: "different"})
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusFound {
		t.Fatalf("status = %d, want 302", w.Code)
	}
	if loc := w.Header().Get("Location"); !strings.Contains(loc, "/auth/callback#error=invalid_state") {
		t.Fatalf("expected invalid_state error redirect, got %q", loc)
	}
}
