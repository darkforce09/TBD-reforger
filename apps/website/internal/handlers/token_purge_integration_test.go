package handlers

import (
	"fmt"
	"testing"
	"time"

	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// seedRefreshTokenAt inserts a refresh token with a fixed expiry (and optional
// revocation) so the purge cutoff can be exercised precisely.
func seedRefreshTokenAt(t *testing.T, h *Handler, discordID string, expiresAt time.Time, revoked bool) models.RefreshToken {
	t.Helper()
	raw, err := auth.RandomToken(32)
	if err != nil {
		t.Fatalf("random token: %v", err)
	}
	rt := models.RefreshToken{
		DiscordID: discordID,
		TokenHash: auth.HashToken(raw),
		ExpiresAt: expiresAt,
	}
	if revoked {
		now := time.Now()
		rt.RevokedAt = &now
	}
	if err := h.db.Create(&rt).Error; err != nil {
		t.Fatalf("seed refresh token: %v", err)
	}
	return rt
}

// TestPurgeExpiredRefreshTokens proves T-130.1 F2B-09: rows more than
// services.RefreshTokenRetention past expiry are deleted (revoked or not), while
// still-valid rows survive. Assertions are per-row, not on the deleted count —
// leftovers from other suites may be swept in the same call.
func TestPurgeExpiredRefreshTokens(t *testing.T) {
	_, h, gdb := setupIT(t)
	discordID := fmt.Sprintf("itest-purge-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Where("discord_id = ?", discordID).Delete(&models.RefreshToken{})
		gdb.Unscoped().Where("discord_id = ?", discordID).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: discordID, Username: "Purge Pat", Role: models.RoleEnlisted})
	stale := seedRefreshTokenAt(t, h, discordID, time.Now().Add(-services.RefreshTokenRetention-24*time.Hour), false)
	staleRevoked := seedRefreshTokenAt(t, h, discordID, time.Now().Add(-services.RefreshTokenRetention-24*time.Hour), true)
	live := seedRefreshTokenAt(t, h, discordID, time.Now().Add(24*time.Hour), false)
	// Expired but inside the retention window: kept (reuse-detection grace).
	graced := seedRefreshTokenAt(t, h, discordID, time.Now().Add(-time.Hour), true)

	if _, err := services.PurgeExpiredRefreshTokens(gdb); err != nil {
		t.Fatalf("purge: %v", err)
	}

	var count int64
	gdb.Model(&models.RefreshToken{}).Where("id IN ?", []uuid.UUID{stale.ID, staleRevoked.ID}).Count(&count)
	if count != 0 {
		t.Errorf("expected stale rows purged, %d remain", count)
	}
	gdb.Model(&models.RefreshToken{}).Where("id IN ?", []uuid.UUID{live.ID, graced.ID}).Count(&count)
	if count != 2 {
		t.Errorf("expected live + graced rows kept, got %d of 2", count)
	}
}
