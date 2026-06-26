// Package db handles the GORM connection and the migration pipeline:
// raw-SQL pre-hooks -> AutoMigrate -> raw-SQL post-hooks.
package db

import (
	"database/sql"
	"embed"
	"fmt"
	"time"

	"gorm.io/driver/postgres"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

//go:embed migrations/*.sql
var migrationsFS embed.FS

// preMigrationFiles run before AutoMigrate: extensions, then ENUM types, since
// AutoMigrate creates columns that reference those types.
var preMigrationFiles = []string{
	"migrations/00_extensions.sql",
	"migrations/01_enums.sql",
	"migrations/02_campaign_refactor.sql",
}

// postMigrationFiles run after AutoMigrate: index types and materialized views
// that GORM struct tags cannot express.
var postMigrationFiles = []string{
	"migrations/03_indexes.sql",
	"migrations/04_leaderboard_mv.sql",
	"migrations/05_registry_items.sql",
}

// allModels is the full set of tables managed by AutoMigrate. Order matters only
// loosely; GORM resolves FK creation, and self-referential FKs are intentionally
// left as plain columns to avoid create-time cycles.
func allModels() []any {
	return []any{
		&models.User{},
		&models.DiscordRole{},
		&models.UserDiscordRole{},
		&models.IdentityLinkCode{},
		&models.RefreshToken{},
		&models.Mission{},
		&models.MissionVersion{},
		&models.MissionArmory{},
		&models.MissionBookmark{},
		&models.Event{},
		&models.EventMission{},
		&models.OrbatSlot{},
		&models.OrbatReservation{},
		&models.EventRegistration{},
		&models.LeaveRequest{},
		&models.Match{},
		&models.MatchPlayerStat{},
		&models.Server{},
		&models.ServerStatus{},
		&models.ServerStatusHistory{},
		&models.Announcement{},
		&models.WikiPage{},
		&models.VehicleDatabase{},
		&models.Modpack{},
		&models.ModpackMod{},
		&models.RegistryItem{},
		&models.Warning{},
		&models.AuditLog{},
		&models.FireMission{},
	}
}

// Open connects to Postgres with GORM, tuning the connection pool and retrying
// the initial connection with backoff (Postgres can briefly refuse connections
// just after it reports ready).
func Open(dsn string, devLogging bool) (*gorm.DB, error) {
	logLevel := logger.Warn
	if devLogging {
		logLevel = logger.Info
	}

	var gdb *gorm.DB
	var err error
	for attempt := 1; attempt <= 10; attempt++ {
		gdb, err = gorm.Open(postgres.Open(dsn), &gorm.Config{
			Logger: logger.Default.LogMode(logLevel),
		})
		if err == nil {
			var sqlDB *sql.DB
			if sqlDB, err = gdb.DB(); err == nil {
				err = sqlDB.Ping()
			}
		}
		if err == nil {
			break
		}
		time.Sleep(time.Duration(attempt) * 250 * time.Millisecond)
	}
	if err != nil {
		return nil, fmt.Errorf("connect postgres: %w", err)
	}

	sqlDB, err := gdb.DB()
	if err != nil {
		return nil, fmt.Errorf("get sql.DB: %w", err)
	}
	sqlDB.SetMaxOpenConns(25)
	sqlDB.SetMaxIdleConns(5)
	sqlDB.SetConnMaxLifetime(30 * time.Minute)
	sqlDB.SetConnMaxIdleTime(5 * time.Minute)

	return gdb, nil
}

// Migrate runs the full pipeline: pre-hooks -> AutoMigrate -> post-hooks.
func Migrate(gdb *gorm.DB) error {
	if err := runSQLFiles(gdb, preMigrationFiles); err != nil {
		return fmt.Errorf("pre-migrations: %w", err)
	}
	if err := gdb.AutoMigrate(allModels()...); err != nil {
		return fmt.Errorf("automigrate: %w", err)
	}
	if err := runSQLFiles(gdb, postMigrationFiles); err != nil {
		return fmt.Errorf("post-migrations: %w", err)
	}
	return nil
}

func runSQLFiles(gdb *gorm.DB, files []string) error {
	for _, f := range files {
		sql, err := migrationsFS.ReadFile(f)
		if err != nil {
			return fmt.Errorf("read %s: %w", f, err)
		}
		if err := gdb.Exec(string(sql)).Error; err != nil {
			return fmt.Errorf("exec %s: %w", f, err)
		}
	}
	return nil
}

// RefreshLeaderboard refreshes the leaderboard_totals materialized view. Call
// after match telemetry ingest (debounced). Falls back to a non-concurrent
// refresh if the view has not been populated yet.
func RefreshLeaderboard(gdb *gorm.DB) error {
	if err := gdb.Exec("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals").Error; err != nil {
		return gdb.Exec("REFRESH MATERIALIZED VIEW leaderboard_totals").Error
	}
	return nil
}
