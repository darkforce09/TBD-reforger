package models

import (
	"time"

	"github.com/google/uuid"
)

type User struct {
	ID        uuid.UUID `json:"id"`
	DiscordID string    `json:"discordId"`
	Username  string    `json:"username"`
	AvatarURL string    `json:"avatarUrl"`
	CreatedAt time.Time `json:"createdAt"`
}

type PageSummary struct {
	Slug  string `json:"slug"`
	Title string `json:"title"`
}

type PageSection struct {
	ID         uuid.UUID `json:"id"`
	SectionKey string    `json:"sectionKey"`
	Heading    string    `json:"heading"`
	Content    string    `json:"content"`
	SortOrder  int       `json:"sortOrder"`
}

type Page struct {
	Slug      string        `json:"slug"`
	Title     string        `json:"title"`
	Published bool          `json:"published"`
	UpdatedAt *time.Time    `json:"updatedAt,omitempty"`
	Sections  []PageSection `json:"sections"`
}

type UpdatePageInput struct {
	Title     *string `json:"title"`
	Published *bool   `json:"published"`
}

type UpsertSectionInput struct {
	ID         *uuid.UUID `json:"id,omitempty"`
	SectionKey string     `json:"sectionKey"`
	Heading    string     `json:"heading"`
	Content    string     `json:"content"`
	SortOrder  int        `json:"sortOrder"`
}

type CreateSectionInput struct {
	SectionKey string `json:"sectionKey"`
	Heading    string `json:"heading"`
	Content    string `json:"content"`
	SortOrder  int    `json:"sortOrder"`
}

type AuthMeResponse struct {
	User    User `json:"user"`
	IsAdmin bool `json:"isAdmin"`
}
