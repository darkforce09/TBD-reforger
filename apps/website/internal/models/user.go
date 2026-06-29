package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// UserRole is the web permission level, synced from Discord roles.
// Backed by the Postgres ENUM type `user_role` (see migrations/01_enums.sql).
type UserRole string

// User roles, low to high privilege (user_role enum).
const (
	RoleEnlisted     UserRole = "enlisted"
	RoleLeader       UserRole = "leader" // community/squad leader: can reserve squads
	RoleMissionMaker UserRole = "mission_maker"
	RoleAdmin        UserRole = "admin"
)

// User is the identity root. Keyed by the Discord snowflake; there are no
// local passwords. Drives the TopBar, Personnel Roster, and the My Deployments
// header stats.
type User struct {
	DiscordID     string   `gorm:"column:discord_id;primaryKey" json:"discord_id"`
	Username      string   `gorm:"not null" json:"username"`                    // "Admin Dave"
	DiscordHandle string   `gorm:"column:discord_handle" json:"discord_handle"` // "Dave#1234"
	AvatarURL     string   `gorm:"column:avatar_url" json:"avatar_url"`
	ArmaID        *string  `gorm:"column:arma_id;uniqueIndex" json:"arma_id"`   // Enfusion/Steam ID, nil until linked
	ArmaCharacter string   `gorm:"column:arma_character" json:"arma_character"` // "[TBD] Admin Dave"
	Role          UserRole `gorm:"type:user_role;not null;default:'enlisted';index" json:"role"`

	IsBanned  bool       `gorm:"not null;default:false" json:"is_banned"`
	BanReason string     `gorm:"column:ban_reason" json:"ban_reason,omitempty"`
	BannedBy  *string    `gorm:"column:banned_by" json:"banned_by,omitempty"` // self-ref discord_id (no FK constraint to avoid cycle)
	BannedAt  *time.Time `gorm:"column:banned_at" json:"banned_at,omitempty"`

	// Denormalized headline metrics, recomputed from attendance + telemetry.
	TotalDeployments int     `gorm:"not null;default:0" json:"total_deployments"`                 // "Total Operations 42"
	AttendanceRate   float64 `gorm:"type:numeric(5,2);not null;default:0" json:"attendance_rate"` // "94%"

	LastLoginAt *time.Time     `gorm:"column:last_login_at" json:"last_login_at,omitempty"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
}

// DiscordRole maps a Discord guild role to a web permission so roles can be
// reconfigured without code changes. Highest `priority` wins on conflict.
type DiscordRole struct {
	DiscordRoleID string    `gorm:"column:discord_role_id;primaryKey" json:"discord_role_id"`
	Name          string    `gorm:"not null" json:"name"`                        // "MissionMaker"
	MappedRole    *UserRole `gorm:"type:user_role" json:"mapped_role,omitempty"` // nil = cosmetic
	Priority      int       `gorm:"not null;default:0" json:"priority"`
}

// UserDiscordRole is the join between a user and their synced Discord roles.
type UserDiscordRole struct {
	DiscordID     string    `gorm:"column:discord_id;primaryKey" json:"discord_id"`
	DiscordRoleID string    `gorm:"column:discord_role_id;primaryKey" json:"discord_role_id"`
	SyncedAt      time.Time `gorm:"column:synced_at;not null;default:now()" json:"synced_at"`
}

// IdentityLinkCode backs the 6-digit Arma linking flow (POST /api/me/link).
// The in-game mod confirms the code, which fills ArmaID and sets ConsumedAt.
type IdentityLinkCode struct {
	Code       string     `gorm:"type:char(6);primaryKey" json:"code"`
	DiscordID  string     `gorm:"column:discord_id;not null;index" json:"discord_id"`
	ArmaID     *string    `gorm:"column:arma_id" json:"arma_id,omitempty"`
	ConsumedAt *time.Time `gorm:"column:consumed_at" json:"consumed_at,omitempty"`
	ExpiresAt  time.Time  `gorm:"column:expires_at;not null" json:"expires_at"` // short TTL (~10 min)
	CreatedAt  time.Time  `json:"created_at"`
}

// RefreshToken is an opaque, rotating refresh credential stored hashed. Logout
// and rotation set RevokedAt; the raw token is only ever returned to the client.
type RefreshToken struct {
	ID        uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	DiscordID string     `gorm:"column:discord_id;not null;index" json:"discord_id"`
	TokenHash string     `gorm:"column:token_hash;not null;uniqueIndex" json:"-"`
	ExpiresAt time.Time  `gorm:"column:expires_at;not null" json:"expires_at"`
	RevokedAt *time.Time `gorm:"column:revoked_at" json:"revoked_at,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
}
