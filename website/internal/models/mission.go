package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

// MissionStatus backs the Postgres ENUM `mission_status`.
type MissionStatus string

const (
	MissionDraft           MissionStatus = "draft"
	MissionPendingApproval MissionStatus = "pending_approval"
	MissionLive            MissionStatus = "live"
	MissionRejected        MissionStatus = "rejected"
	MissionArchived        MissionStatus = "archived"
)

// TerrainType backs the Postgres ENUM `terrain_type`.
type TerrainType string

const (
	TerrainEveron TerrainType = "everon"
	TerrainArland TerrainType = "arland"
	TerrainCustom TerrainType = "custom"
)

// GameMode backs the Postgres ENUM `game_mode`.
type GameMode string

const (
	GameModePvECoop GameMode = "pve_coop"
	GameModePvP     GameMode = "pvp"
	GameModeZeus    GameMode = "zeus"
)

// WeatherType backs the Postgres ENUM `weather_type`.
type WeatherType string

const (
	WeatherClear     WeatherType = "clear"
	WeatherOvercast  WeatherType = "overcast"
	WeatherHeavyRain WeatherType = "heavy_rain"
	WeatherDenseFog  WeatherType = "dense_fog"
)

// Mission is a custom mission in the library. The heavy 2D-editor payload lives
// in MissionVersion; this row carries metadata and points at the current version.
type Mission struct {
	ID                uuid.UUID     `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Title             string        `gorm:"not null" json:"title"` // "Operation Enduring Freedom"
	AuthorID          string        `gorm:"column:author_id;not null;index" json:"author_id"`
	Terrain           TerrainType   `gorm:"type:terrain_type;not null;index" json:"terrain"`
	CustomTerrainName string        `gorm:"column:custom_terrain_name" json:"custom_terrain_name,omitempty"` // when terrain=custom
	GameMode          GameMode      `gorm:"type:game_mode;not null;index" json:"game_mode"`
	Weather           WeatherType   `gorm:"type:weather_type;not null;default:'clear'" json:"weather"`
	TimeOfDay         string        `gorm:"type:time without time zone;not null;default:'14:00'" json:"time_of_day"` // insertion time (HH:MM)
	MaxPlayers        int           `gorm:"column:max_players;not null" json:"max_players"`                          // "64 Players max"
	Status            MissionStatus `gorm:"type:mission_status;not null;default:'draft';index" json:"status"`
	ThumbnailURL      string        `gorm:"column:thumbnail_url" json:"thumbnail_url,omitempty"`
	Briefing          string        `json:"briefing,omitempty"` // long-form briefing markdown

	CurrentVersionID *uuid.UUID `gorm:"type:uuid;column:current_version_id" json:"current_version_id,omitempty"`

	RejectionReason string     `gorm:"column:rejection_reason" json:"rejection_reason,omitempty"`
	ReviewedBy      *string    `gorm:"column:reviewed_by" json:"reviewed_by,omitempty"`
	ReviewedAt      *time.Time `gorm:"column:reviewed_at" json:"reviewed_at,omitempty"`

	CreatedAt time.Time      `json:"created_at"`
	UpdatedAt time.Time      `json:"updated_at"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
}

// MissionVersion stores an immutable snapshot of the 2D editor output. The
// json_payload holds spawns, vehicles, and map markers. Unique per (mission, semver).
type MissionVersion struct {
	ID          uuid.UUID      `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	MissionID   uuid.UUID      `gorm:"type:uuid;column:mission_id;not null;index;uniqueIndex:idx_mission_semver" json:"mission_id"`
	Semver      string         `gorm:"not null;uniqueIndex:idx_mission_semver" json:"semver"` // "1.2.0"
	JSONPayload datatypes.JSON `gorm:"type:jsonb;column:json_payload;not null" json:"json_payload"`
	EditorNotes string         `gorm:"column:editor_notes" json:"editor_notes,omitempty"`
	CreatedBy   string         `gorm:"column:created_by;not null" json:"created_by"`
	CreatedAt   time.Time      `json:"created_at"`
}

// MissionArmory is one weapon/vehicle/equipment line on the Mission Overview
// armory ("M16A2 Rifle x45 Available"), normalized for cheap list rendering.
type MissionArmory struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	MissionID uuid.UUID `gorm:"type:uuid;column:mission_id;not null;index" json:"mission_id"`
	Faction   string    `gorm:"not null" json:"faction"`  // "US Forces" / "USSR" / "FIA"
	Category  string    `gorm:"not null" json:"category"` // weapon / vehicle / equipment
	ItemName  string    `gorm:"column:item_name;not null" json:"item_name"`
	Quantity  *int      `json:"quantity,omitempty"` // nil = unlimited
	Icon      string    `json:"icon,omitempty"`
	SortOrder int       `gorm:"column:sort_order;not null;default:0" json:"sort_order"`
}

// MissionBookmark backs the "Bookmarked" tab in the Mission Library.
type MissionBookmark struct {
	DiscordID string    `gorm:"column:discord_id;primaryKey" json:"discord_id"`
	MissionID uuid.UUID `gorm:"type:uuid;column:mission_id;primaryKey" json:"mission_id"`
	CreatedAt time.Time `json:"created_at"`
}
