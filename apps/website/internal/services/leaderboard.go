package services

import (
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/db"
)

// RefreshLeaderboard refreshes the leaderboard materialized view. It is a thin
// wrapper over db.RefreshLeaderboard so HTTP handlers depend on the services
// package rather than reaching into internal/db directly (GO-9).
func RefreshLeaderboard(gdb *gorm.DB) error {
	return db.RefreshLeaderboard(gdb)
}
