package models

import (
	"time"

	"github.com/google/uuid"
)

type Event struct {
	ID           uuid.UUID  `json:"id"`
	Title        string     `json:"title"`
	Slug         string     `json:"slug"`
	Description  string     `json:"description"`
	MapName      string     `json:"mapName"`
	StartsAt     time.Time  `json:"startsAt"`
	EndsAt       *time.Time `json:"endsAt,omitempty"`
	Status       string     `json:"status"`
	MaxPlayers   *int       `json:"maxPlayers,omitempty"`
	SignupsOpen  bool       `json:"signupsOpen"`
	Published    bool       `json:"published"`
	MissionID    *string    `json:"missionId,omitempty"`
	CreatedAt    time.Time  `json:"createdAt"`
	UpdatedAt    *time.Time `json:"updatedAt,omitempty"`
	Registered   int        `json:"registeredCount,omitempty"`
	Waitlisted   int        `json:"waitlistCount,omitempty"`
	UserReg      *Registration `json:"userRegistration,omitempty"`
}

type EventSummary struct {
	ID          uuid.UUID  `json:"id"`
	Title       string     `json:"title"`
	Slug        string     `json:"slug"`
	MapName     string     `json:"mapName"`
	StartsAt    time.Time  `json:"startsAt"`
	EndsAt      *time.Time `json:"endsAt,omitempty"`
	Status      string     `json:"status"`
	MaxPlayers  *int       `json:"maxPlayers,omitempty"`
	SignupsOpen bool       `json:"signupsOpen"`
	Registered  int        `json:"registeredCount"`
}

type Announcement struct {
	ID          uuid.UUID  `json:"id"`
	Title       string     `json:"title"`
	Body        string     `json:"body"`
	Pinned      bool       `json:"pinned"`
	Published   bool       `json:"published"`
	PublishedAt *time.Time `json:"publishedAt,omitempty"`
	CreatedAt   time.Time  `json:"createdAt"`
	UpdatedAt   *time.Time `json:"updatedAt,omitempty"`
}

type Registration struct {
	ID         uuid.UUID  `json:"id"`
	EventID    uuid.UUID  `json:"eventId"`
	UserID     uuid.UUID  `json:"userId"`
	Status     string     `json:"status"`
	SignedUpAt time.Time  `json:"signedUpAt"`
	CancelledAt *time.Time `json:"cancelledAt,omitempty"`
	User       *User      `json:"user,omitempty"`
	Event      *EventSummary `json:"event,omitempty"`
}

type CreateEventInput struct {
	Title       string     `json:"title"`
	Slug        string     `json:"slug"`
	Description string     `json:"description"`
	MapName     string     `json:"mapName"`
	StartsAt    time.Time  `json:"startsAt"`
	EndsAt      *time.Time `json:"endsAt,omitempty"`
	Status      string     `json:"status"`
	MaxPlayers  *int       `json:"maxPlayers,omitempty"`
	SignupsOpen bool       `json:"signupsOpen"`
	Published   bool       `json:"published"`
	MissionID   *string    `json:"missionId,omitempty"`
}

type UpdateEventInput struct {
	Title       *string    `json:"title,omitempty"`
	Slug        *string    `json:"slug,omitempty"`
	Description *string    `json:"description,omitempty"`
	MapName     *string    `json:"mapName,omitempty"`
	StartsAt    *time.Time `json:"startsAt,omitempty"`
	EndsAt      *time.Time `json:"endsAt,omitempty"`
	Status      *string    `json:"status,omitempty"`
	MaxPlayers  *int       `json:"maxPlayers,omitempty"`
	SignupsOpen *bool      `json:"signupsOpen,omitempty"`
	Published   *bool      `json:"published,omitempty"`
	MissionID   *string    `json:"missionId,omitempty"`
}

type CreateAnnouncementInput struct {
	Title       string     `json:"title"`
	Body        string     `json:"body"`
	Pinned      bool       `json:"pinned"`
	Published   bool       `json:"published"`
	PublishedAt *time.Time `json:"publishedAt,omitempty"`
}

type UpdateAnnouncementInput struct {
	Title       *string    `json:"title,omitempty"`
	Body        *string    `json:"body,omitempty"`
	Pinned      *bool      `json:"pinned,omitempty"`
	Published   *bool      `json:"published,omitempty"`
	PublishedAt *time.Time `json:"publishedAt,omitempty"`
}

type UpdateRegistrationInput struct {
	Status string `json:"status"`
}
