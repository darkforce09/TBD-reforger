package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// EventStatus backs the Postgres ENUM `event_status`.
type EventStatus string

// Event lifecycle states (event_status enum).
const (
	EventScheduled EventStatus = "scheduled"
	EventOpen      EventStatus = "open"
	EventLocked    EventStatus = "locked"
	EventLive      EventStatus = "live"
	EventCompleted EventStatus = "completed"
	EventCancelled EventStatus = "cancelled"
)

// RegistrationState backs the Postgres ENUM `registration_state`.
type RegistrationState string

// Registration states (registration_state enum).
const (
	RegRegistered RegistrationState = "registered"
	RegWaitlisted RegistrationState = "waitlisted"
	RegWithdrawn  RegistrationState = "withdrawn"
	RegAttended   RegistrationState = "attended"
	RegNoShow     RegistrationState = "no_show"
)

// LeaveStatus backs the Postgres ENUM `leave_status`.
type LeaveStatus string

// Leave-request states (leave_status enum).
const (
	LeavePending  LeaveStatus = "pending"
	LeaveApproved LeaveStatus = "approved"
	LeaveDenied   LeaveStatus = "denied"
)

// Event is a scheduled operation that contains one or more sequential missions
// (the "campaign" container). Drives the Event Hub, the Event Manager, the
// Upcoming Operations list, and Server Intel "Active Deployment".
type Event struct {
	ID                 uuid.UUID   `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	NameOverride       string      `gorm:"column:name_override" json:"name_override,omitempty"`
	StartTime          time.Time   `gorm:"column:start_time;not null;index" json:"start_time"` // "Oct 28, 20:00 EST"
	Briefing           string      `json:"briefing,omitempty"`                                 // overarching event lore/briefing
	BannerImageURL     string      `gorm:"column:banner_image_url" json:"banner_image_url,omitempty"`
	Status             EventStatus `gorm:"type:event_status;not null;default:'scheduled';index" json:"status"`
	RegistrationLocked bool        `gorm:"column:registration_locked;not null;default:false" json:"registration_locked"`
	MaxSlots           int         `gorm:"column:max_slots;not null;default:0" json:"max_slots"` // informational overall cap
	CreatedBy          string      `gorm:"column:created_by;not null" json:"created_by"`
	MatchID            *uuid.UUID  `gorm:"type:uuid;column:match_id" json:"match_id,omitempty"` // set once played

	CreatedAt time.Time      `json:"created_at"`
	UpdatedAt time.Time      `json:"updated_at"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
}

// EventMission links an Event (the container) to a Mission, with its own start
// time so missions in one event can run sequentially. ORBAT slots and
// registrations hang off this row, not the Event.
type EventMission struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	EventID   uuid.UUID `gorm:"type:uuid;column:event_id;not null;index;uniqueIndex:idx_event_mission" json:"event_id"`
	MissionID uuid.UUID `gorm:"type:uuid;column:mission_id;not null;uniqueIndex:idx_event_mission" json:"mission_id"`
	StartTime time.Time `gorm:"column:start_time;not null" json:"start_time"`

	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

// OrbatSlot is one fillable position in the Order of Battle for a mission within
// an event, e.g. faction "GAR", squad "Alpha 1-1", role "Combat Medic".
type OrbatSlot struct {
	ID             uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	EventMissionID uuid.UUID  `gorm:"type:uuid;column:event_mission_id;not null;index;uniqueIndex:idx_orbat_slot" json:"event_mission_id"`
	Faction        string     `gorm:"not null" json:"faction"`
	Squad          string     `gorm:"not null;uniqueIndex:idx_orbat_slot" json:"squad"` // "Alpha 1-1"
	Callsign       string     `json:"callsign,omitempty"`                               // "Platoon HQ"
	Role           string     `gorm:"not null" json:"role"`                             // "Combat Medic"
	Loadout        string     `json:"loadout,omitempty"`                                // "L85A3 + GL"
	Tag            string     `json:"tag,omitempty"`                                    // specialization, e.g. "MED" / "ENG"
	SlotIndex      int        `gorm:"column:slot_index;not null;uniqueIndex:idx_orbat_slot" json:"slot_index"`
	AssignedTo     *string    `gorm:"column:assigned_to;index" json:"assigned_to,omitempty"`
	AssignedAt     *time.Time `gorm:"column:assigned_at" json:"assigned_at,omitempty"`
}

// OrbatReservation is a one-click hold a leader places on an entire squad within
// a mission: while it exists, only the reserver (or an admin) may fill its slots.
type OrbatReservation struct {
	ID             uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	EventMissionID uuid.UUID `gorm:"type:uuid;column:event_mission_id;not null;index;uniqueIndex:idx_orbat_reservation" json:"event_mission_id"`
	Squad          string    `gorm:"not null;uniqueIndex:idx_orbat_reservation" json:"squad"`
	ReservedBy     string    `gorm:"column:reserved_by;not null;index" json:"reserved_by"`
	ReservedAt     time.Time `gorm:"column:reserved_at;not null;default:now()" json:"reserved_at"`
}

// EventRegistration is a user signed up for a specific mission within an event.
// Feeds the ORBAT fill counts, the waitlist, and the My Deployments
// "ASSIGNED SLOT" badge. A user may register for several missions in one event.
type EventRegistration struct {
	ID             uuid.UUID         `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	EventMissionID uuid.UUID         `gorm:"type:uuid;column:event_mission_id;not null;index;uniqueIndex:idx_reg_unique" json:"event_mission_id"`
	DiscordID      string            `gorm:"column:discord_id;not null;index;uniqueIndex:idx_reg_unique" json:"discord_id"`
	SlotID         *uuid.UUID        `gorm:"type:uuid;column:slot_id" json:"slot_id,omitempty"`
	State          RegistrationState `gorm:"type:registration_state;not null;default:'registered'" json:"state"`
	RegisteredAt   time.Time         `gorm:"column:registered_at;not null;default:now()" json:"registered_at"`
}

// LeaveRequest backs "Submit Leave of Absence (LOA)".
type LeaveRequest struct {
	ID         uuid.UUID   `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	DiscordID  string      `gorm:"column:discord_id;not null;index" json:"discord_id"`
	StartsOn   time.Time   `gorm:"column:starts_on;type:date;not null" json:"starts_on"`
	EndsOn     time.Time   `gorm:"column:ends_on;type:date;not null" json:"ends_on"`
	Reason     string      `json:"reason,omitempty"`
	Status     LeaveStatus `gorm:"type:leave_status;not null;default:'pending'" json:"status"`
	ReviewedBy *string     `gorm:"column:reviewed_by" json:"reviewed_by,omitempty"`
	CreatedAt  time.Time   `json:"created_at"`
}
