package models

import (
	"encoding/json"
	"time"

	"github.com/google/uuid"
)

type Mission struct {
	ID            string          `json:"id"`
	Name          string          `json:"name"`
	SchemaVersion string          `json:"schemaVersion"`
	Content       json.RawMessage `json:"content"`
	ContentHash   string          `json:"contentHash"`
	AuthorUserID  *uuid.UUID      `json:"authorUserId,omitempty"`
	PublishedAt   time.Time       `json:"publishedAt"`
	CreatedAt     time.Time       `json:"createdAt"`
}

type GameIdentity struct {
	ID         uuid.UUID `json:"id"`
	UserID     uuid.UUID `json:"userId"`
	IdentityID string    `json:"identityId"`
	Platform   string    `json:"platform"`
	LinkedAt   time.Time `json:"linkedAt"`
}

type EventSlotAssignment struct {
	ID         uuid.UUID  `json:"id"`
	EventID    uuid.UUID  `json:"eventId"`
	SlotID     string     `json:"slotId"`
	UserID     *uuid.UUID `json:"userId,omitempty"`
	AssignedBy *uuid.UUID `json:"assignedBy,omitempty"`
	AssignedAt time.Time  `json:"assignedAt"`
	User       *User      `json:"user,omitempty"`
}

type GameRosterEntry struct {
	IdentityID string `json:"identityId"`
	SlotID     string `json:"slotId"`
}

type GameRosterResponse struct {
	EventID   uuid.UUID         `json:"eventId"`
	MissionID string            `json:"missionId,omitempty"`
	Assignments map[string]string `json:"assignments"`
}

type RegisterLinkCodeInput struct {
	Code       string     `json:"code"`
	IdentityID string     `json:"identityId"`
	EventID    *uuid.UUID `json:"eventId,omitempty"`
	Platform   string     `json:"platform"`
}

type ConsumeLinkCodeInput struct {
	Code string `json:"code"`
}

type AssignSlotInput struct {
	UserID *uuid.UUID `json:"userId"`
}

type PublishMissionInput struct {
	// Raw mission JSON body — validated against tbd-schema.
}
