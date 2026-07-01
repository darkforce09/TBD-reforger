package handlers

import (
	"fmt"
	"net/http"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// seedRefreshToken inserts an active refresh token for a user and returns its raw value.
func seedRefreshToken(t *testing.T, h *Handler, discordID string) string {
	t.Helper()
	raw, err := auth.RandomToken(32)
	if err != nil {
		t.Fatalf("random token: %v", err)
	}
	rt := models.RefreshToken{
		DiscordID: discordID,
		TokenHash: auth.HashToken(raw),
		ExpiresAt: time.Now().Add(24 * time.Hour),
	}
	if err := h.db.Create(&rt).Error; err != nil {
		t.Fatalf("seed refresh token: %v", err)
	}
	return raw
}

// TestRefreshReuseRevokesFamily proves T-126 S2: a rotated (single-use) refresh token
// rotates once; replaying the spent token is treated as reuse (401) and revokes the
// whole token family, so the freshly issued token is invalidated too.
func TestRefreshReuseRevokesFamily(t *testing.T) {
	r, h, gdb := setupIT(t)
	discordID := fmt.Sprintf("itest-rt-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.RefreshToken{})
		gdb.Where("actor_id = ?", discordID).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: discordID, Username: "RT Rita", Role: models.RoleEnlisted})
	raw := seedRefreshToken(t, h, discordID)

	// First rotation succeeds and hands back a new pair.
	w := do(r, "POST", "/api/v1/auth/refresh", reqOpt{body: fmt.Sprintf(`{"refresh_token":%q}`, raw)})
	if w.Code != http.StatusOK {
		t.Fatalf("first refresh = %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	var rotated struct {
		RefreshToken string `json:"refresh_token"`
	}
	mustJSON(t, w, &rotated)
	if rotated.RefreshToken == "" || rotated.RefreshToken == raw {
		t.Fatalf("expected a rotated refresh token, got %q", rotated.RefreshToken)
	}

	// Replaying the spent token is reuse → 401.
	if w := do(r, "POST", "/api/v1/auth/refresh", reqOpt{body: fmt.Sprintf(`{"refresh_token":%q}`, raw)}); w.Code != http.StatusUnauthorized {
		t.Fatalf("reused token refresh = %d, want 401 (body=%s)", w.Code, w.Body.String())
	}

	// Reuse revokes the whole family: the freshly issued token is now dead too.
	if w := do(r, "POST", "/api/v1/auth/refresh", reqOpt{body: fmt.Sprintf(`{"refresh_token":%q}`, rotated.RefreshToken)}); w.Code != http.StatusUnauthorized {
		t.Fatalf("post-reuse rotated token refresh = %d, want 401 (family revoked) (body=%s)", w.Code, w.Body.String())
	}

	var live int64
	gdb.Model(&models.RefreshToken{}).Where("discord_id = ? AND revoked_at IS NULL", discordID).Count(&live)
	if live != 0 {
		t.Fatalf("expected 0 live tokens after family revoke, got %d", live)
	}
}

// TestRefreshBannedRejected proves T-126 S4: a banned user cannot keep a session alive
// through rotation — refresh returns 403 and revokes their token family.
func TestRefreshBannedRejected(t *testing.T) {
	r, h, gdb := setupIT(t)
	discordID := fmt.Sprintf("itest-ban-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.RefreshToken{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: discordID, Username: "Banned Ben", Role: models.RoleEnlisted, IsBanned: true})
	raw := seedRefreshToken(t, h, discordID)

	if w := do(r, "POST", "/api/v1/auth/refresh", reqOpt{body: fmt.Sprintf(`{"refresh_token":%q}`, raw)}); w.Code != http.StatusForbidden {
		t.Fatalf("banned refresh = %d, want 403 (body=%s)", w.Code, w.Body.String())
	}

	var live int64
	gdb.Model(&models.RefreshToken{}).Where("discord_id = ? AND revoked_at IS NULL", discordID).Count(&live)
	if live != 0 {
		t.Fatalf("expected banned user's token family revoked, got %d live", live)
	}
}
