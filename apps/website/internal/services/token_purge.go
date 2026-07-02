package services

import (
	"context"
	"log"
	"time"

	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// RefreshTokenRetention is the purge policy for refresh-token rows (T-130.1 F2B-09):
// a row is deleted once expires_at is more than this far in the past. Revoked rows
// must NOT be purged before expiry — they are the reuse-detection tripwire (a
// replayed spent token revokes the whole family, handlers Refresh, T-126 S2). Past
// expiry the token lookup rejects it regardless, so the extra week only buys clock
// skew tolerance and a forensic window.
const RefreshTokenRetention = 7 * 24 * time.Hour

// refreshPurgeInterval is how often StartRefreshTokenPurge re-sweeps after boot.
const refreshPurgeInterval = 6 * time.Hour

// PurgeExpiredRefreshTokens hard-deletes refresh-token rows that expired more than
// RefreshTokenRetention ago (RefreshToken has no soft-delete column) and returns
// the number of rows removed.
func PurgeExpiredRefreshTokens(db *gorm.DB) (int64, error) {
	res := db.Where("expires_at < ?", time.Now().Add(-RefreshTokenRetention)).
		Delete(&models.RefreshToken{})
	return res.RowsAffected, res.Error
}

// StartRefreshTokenPurge runs an immediate sweep and then re-sweeps every
// refreshPurgeInterval until ctx is cancelled. Failures are logged and retried on
// the next tick; the sweep never takes the API down.
func StartRefreshTokenPurge(ctx context.Context, db *gorm.DB) {
	go func() {
		sweep := func() {
			n, err := PurgeExpiredRefreshTokens(db)
			if err != nil {
				log.Printf("refresh token purge failed: %v", err)
				return
			}
			log.Printf("refresh token purge: %d rows", n)
		}
		sweep()
		ticker := time.NewTicker(refreshPurgeInterval)
		defer ticker.Stop()
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				sweep()
			}
		}
	}()
}
