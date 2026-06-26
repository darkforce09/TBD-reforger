package models

import (
	"time"

	"github.com/google/uuid"
)

// MissionOutcome backs the Postgres ENUM `mission_outcome`.
type MissionOutcome string

const (
	OutcomeSuccess MissionOutcome = "success"
	OutcomeFailure MissionOutcome = "failure"
	OutcomeAborted MissionOutcome = "aborted"
	OutcomePending MissionOutcome = "pending"
)

// Match is one completed operation instance. Post-match telemetry attaches here
// and feeds the Global Leaderboards and the My Deployments service record.
type Match struct {
	ID             uuid.UUID      `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	SourceMatchID  *string        `gorm:"column:source_match_id;uniqueIndex" json:"source_match_id,omitempty"` // stable id from the game server for idempotent ingest
	EventID        *uuid.UUID     `gorm:"type:uuid;column:event_id" json:"event_id,omitempty"`                 // nil for unscheduled scrims
	MissionID      *uuid.UUID     `gorm:"type:uuid;column:mission_id" json:"mission_id,omitempty"`
	Terrain        *TerrainType   `gorm:"type:terrain_type" json:"terrain,omitempty"`
	StartedAt      time.Time      `gorm:"column:started_at;not null" json:"started_at"`
	EndedAt        *time.Time     `gorm:"column:ended_at" json:"ended_at,omitempty"`
	Outcome        MissionOutcome `gorm:"type:mission_outcome;not null;default:'pending'" json:"outcome"`
	WinningFaction string         `gorm:"column:winning_faction" json:"winning_faction,omitempty"`
	AARReplayURL   string         `gorm:"column:aar_replay_url" json:"aar_replay_url,omitempty"` // "View Replay"
	CreatedAt      time.Time      `json:"created_at"`
}

// MatchPlayerStat is a per-player line item ingested from the game server.
// Idempotent on (match_id, arma_id, source_event_id). Resolved to a user via arma_id.
type MatchPlayerStat struct {
	ID                uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	MatchID           uuid.UUID `gorm:"type:uuid;column:match_id;not null;index;uniqueIndex:idx_mps_dedupe" json:"match_id"`
	DiscordID         *string   `gorm:"column:discord_id;index" json:"discord_id,omitempty"`
	ArmaID            string    `gorm:"column:arma_id;not null;uniqueIndex:idx_mps_dedupe" json:"arma_id"`
	RolePlayed        string    `gorm:"column:role_played" json:"role_played,omitempty"` // "Alpha 1-2 (Rifleman)"
	Kills             int       `gorm:"not null;default:0" json:"kills"`
	Deaths            int       `gorm:"not null;default:0" json:"deaths"`
	TeamKills         int       `gorm:"column:team_kills;not null;default:0" json:"team_kills"` // Wall of Shame
	LongestKillM      int       `gorm:"column:longest_kill_m;not null;default:0" json:"longest_kill_m"`
	VehiclesDestroyed int       `gorm:"column:vehicles_destroyed;not null;default:0" json:"vehicles_destroyed"`
	IsCommand         bool      `gorm:"column:is_command;not null;default:false" json:"is_command"` // played Platoon HQ
	CommandWin        *bool     `gorm:"column:command_win" json:"command_win,omitempty"`
	SourceEventID     string    `gorm:"column:source_event_id;not null;uniqueIndex:idx_mps_dedupe" json:"source_event_id"`
	CreatedAt         time.Time `json:"created_at"`
}

// Server is a registered Arma Reforger server instance.
type Server struct {
	ID                uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Name              string     `gorm:"not null" json:"name"`
	IP                string     `gorm:"type:inet;not null" json:"ip"` // stored as Postgres inet, bound as text
	Port              int        `gorm:"not null" json:"port"`
	RequiredModpackID *uuid.UUID `gorm:"type:uuid;column:required_modpack_id" json:"required_modpack_id,omitempty"`
	IsActive          bool       `gorm:"column:is_active;not null;default:true" json:"is_active"`
}

// ServerStatus is the single hot row of current state per server. Drives
// Server Intel Column 1 (uptime, player count, FPS) and the dashboard badge.
type ServerStatus struct {
	ServerID       uuid.UUID  `gorm:"type:uuid;column:server_id;primaryKey" json:"server_id"`
	IsOnline       bool       `gorm:"column:is_online;not null;default:false" json:"is_online"`
	PlayerCount    int        `gorm:"column:player_count;not null;default:0" json:"player_count"`
	MaxPlayers     int        `gorm:"column:max_players;not null;default:64" json:"max_players"`
	ServerFPS      float64    `gorm:"column:server_fps;type:numeric(5,1);not null;default:0" json:"server_fps"`
	UptimeSeconds  int64      `gorm:"column:uptime_seconds;not null;default:0" json:"uptime_seconds"` // render "02:14:33"
	CurrentMatchID *uuid.UUID `gorm:"type:uuid;column:current_match_id" json:"current_match_id,omitempty"`
	IngameTime     string     `gorm:"column:ingame_time" json:"ingame_time,omitempty"`       // "14:30 Local"
	IngameWeather  string     `gorm:"column:ingame_weather" json:"ingame_weather,omitempty"` // "Overcast / Light Rain"
	UpdatedAt      time.Time  `json:"updated_at"`
}

// ServerStatusHistory is the time-series feed used for the "FPS dropped below 20"
// audit alert. Retention/partition candidate (drop > 30d).
type ServerStatusHistory struct {
	ID          int64     `gorm:"primaryKey;autoIncrement" json:"id"`
	ServerID    uuid.UUID `gorm:"type:uuid;column:server_id;not null;index:idx_status_hist,priority:1" json:"server_id"`
	PlayerCount int       `gorm:"column:player_count;not null" json:"player_count"`
	ServerFPS   float64   `gorm:"column:server_fps;type:numeric(5,1);not null" json:"server_fps"`
	RecordedAt  time.Time `gorm:"column:recorded_at;not null;default:now();index:idx_status_hist,priority:2,sort:desc" json:"recorded_at"`
}
