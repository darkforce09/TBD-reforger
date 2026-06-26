package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/datatypes"
)

// AuditSeverity backs the Postgres ENUM `audit_severity` (INFO / WARN / CRIT).
type AuditSeverity string

const (
	SeverityInfo AuditSeverity = "info"
	SeverityWarn AuditSeverity = "warn"
	SeverityCrit AuditSeverity = "crit"
)

// Warning is a disciplinary record. The Personnel Roster "Warnings" column is a
// COUNT(*) of these per user.
type Warning struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	DiscordID string    `gorm:"column:discord_id;not null;index" json:"discord_id"`
	IssuedBy  string    `gorm:"column:issued_by;not null" json:"issued_by"`
	Reason    string    `gorm:"not null" json:"reason"`
	CreatedAt time.Time `json:"created_at"`
}

// AuditLog is the admin papertrail rendered in the terminal-style Audit Logs
// console, e.g. "[14:22:01] [INFO] Admin Dave approved mission 'Op Overcast'".
type AuditLog struct {
	ID         int64          `gorm:"primaryKey;autoIncrement" json:"id"`
	Severity   AuditSeverity  `gorm:"type:audit_severity;not null;default:'info';index" json:"severity"`
	ActorID    *string        `gorm:"column:actor_id" json:"actor_id,omitempty"`     // nil for system events
	ActorName  string         `gorm:"column:actor_name" json:"actor_name,omitempty"` // denormalized "Admin Dave"
	Action     string         `gorm:"not null" json:"action"`                        // "mission.approve"
	Message    string         `gorm:"not null" json:"message"`                       // rendered line
	TargetType string         `gorm:"column:target_type" json:"target_type,omitempty"`
	TargetID   string         `gorm:"column:target_id" json:"target_id,omitempty"`
	Metadata   datatypes.JSON `gorm:"type:jsonb" json:"metadata,omitempty"`
	CreatedAt  time.Time      `gorm:"index:idx_audit_created,sort:desc" json:"created_at"`
}

// FireMission is a saved mortar firing solution from the Mortar Calculator,
// shareable across an operation's tactical planning.
type FireMission struct {
	ID            uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	EventID       *uuid.UUID `gorm:"type:uuid;column:event_id" json:"event_id,omitempty"`
	CreatedBy     string     `gorm:"column:created_by;not null" json:"created_by"`
	WeaponSystem  string     `gorm:"column:weapon_system;not null" json:"weapon_system"` // "M252 81mm"
	FPGrid        string     `gorm:"column:fp_grid;not null" json:"fp_grid"`             // firing position ("FP Alpha")
	TargetGrid    string     `gorm:"column:target_grid;not null" json:"target_grid"`     // "TGT 001"
	DistanceM     int        `gorm:"column:distance_m;not null" json:"distance_m"`
	AzimuthDeg    float64    `gorm:"column:azimuth_deg;type:numeric(5,1);not null" json:"azimuth_deg"`
	ElevationMils int        `gorm:"column:elevation_mils;not null" json:"elevation_mils"`
	CreatedAt     time.Time  `json:"created_at"`
}
