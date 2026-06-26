package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// AnnouncementStatus backs the Postgres ENUM `announcement_status`.
type AnnouncementStatus string

const (
	AnnouncementDraft     AnnouncementStatus = "draft"
	AnnouncementPublished AnnouncementStatus = "published"
	AnnouncementArchived  AnnouncementStatus = "archived"
)

// AnnouncementTag backs the Postgres ENUM `announcement_tag` (Category Tag in CMS).
type AnnouncementTag string

const (
	TagUpdate        AnnouncementTag = "update"
	TagEvent         AnnouncementTag = "event"
	TagModpackUpdate AnnouncementTag = "modpack_update"
	TagImportant     AnnouncementTag = "important"
)

// Announcement powers the news feed, the Content Manager, and the dashboard
// "Recent Announcements" stack.
type Announcement struct {
	ID           uuid.UUID          `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Title        string             `gorm:"not null" json:"title"`
	Body         string             `gorm:"not null" json:"body"` // rich text / markdown HTML
	Snippet      string             `json:"snippet,omitempty"`    // derived preview
	Tag          AnnouncementTag    `gorm:"type:announcement_tag;not null;default:'update'" json:"tag"`
	ThumbnailURL string             `gorm:"column:thumbnail_url" json:"thumbnail_url,omitempty"`
	AuthorID     string             `gorm:"column:author_id;not null" json:"author_id"`
	Status       AnnouncementStatus `gorm:"type:announcement_status;not null;default:'draft'" json:"status"`
	IsPinned     bool               `gorm:"column:is_pinned;not null;default:false" json:"is_pinned"` // "PINNED"

	PushedToDiscord  bool       `gorm:"column:pushed_to_discord;not null;default:false" json:"pushed_to_discord"`
	DiscordMessageID string     `gorm:"column:discord_message_id" json:"discord_message_id,omitempty"`
	PublishedAt      *time.Time `gorm:"column:published_at" json:"published_at,omitempty"`

	CreatedAt time.Time      `json:"created_at"`
	UpdatedAt time.Time      `json:"updated_at"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
}

// WikiPage is a SOP / manual document. Left-nav category + markdown body with
// tables and warning callouts. Edited via the Content Manager "Edit Wiki Page" tab.
type WikiPage struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Slug      string    `gorm:"uniqueIndex;not null" json:"slug"` // "vehicle-database-iff"
	Category  string    `gorm:"not null" json:"category"`         // "Vehicle Database & IFF"
	Title     string    `gorm:"not null" json:"title"`
	Icon      string    `json:"icon,omitempty"` // material symbol ("local_shipping")
	BodyMD    string    `gorm:"column:body_md;not null" json:"body_md"`
	NavOrder  int       `gorm:"column:nav_order;not null;default:0" json:"nav_order"`
	UpdatedBy *string   `gorm:"column:updated_by" json:"updated_by,omitempty"`
	UpdatedAt time.Time `json:"updated_at"`
}

// VehicleDatabase is the structured IFF table on the Vehicle Database wiki page.
type VehicleDatabase struct {
	ID              uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Name            string    `gorm:"not null" json:"name"`                         // "BTR-70"
	Faction         string    `gorm:"not null" json:"faction"`                      // "USSR"
	ArmorType       string    `gorm:"column:armor_type;not null" json:"armor_type"` // "Light Armored"
	Amphibious      string    `json:"amphibious,omitempty"`                         // "Yes (5km/h)" / "No"
	PrimaryThreat   string    `gorm:"column:primary_threat" json:"primary_threat,omitempty"`
	ProfileImageURL string    `gorm:"column:profile_image_url" json:"profile_image_url,omitempty"`
}

// Modpack is a downloadable dependency set. Drives the Modpacks page and the
// Server Intel "Required Modpack" verify pill.
type Modpack struct {
	ID             uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	Name           string    `gorm:"not null" json:"name"`                                     // "Core Modern Expansion"
	Version        string    `gorm:"not null" json:"version"`                                  // "2.1"
	TotalSizeBytes int64     `gorm:"column:total_size_bytes;not null" json:"total_size_bytes"` // render "45.2 GB"
	WorkshopURL    string    `gorm:"column:workshop_url" json:"workshop_url,omitempty"`
	IsCurrent      bool      `gorm:"column:is_current;not null;default:false" json:"is_current"`
	CreatedAt      time.Time `json:"created_at"`
}

// ModpackMod is one mod inside a modpack ("RHS: Status Quo", "TFAR", "ACE3").
type ModpackMod struct {
	ID              uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()" json:"id"`
	ModpackID       uuid.UUID `gorm:"type:uuid;column:modpack_id;not null;index" json:"modpack_id"`
	Name            string    `gorm:"not null" json:"name"`
	IsKeyDependency bool      `gorm:"column:is_key_dependency;not null;default:false" json:"is_key_dependency"`
	SortOrder       int       `gorm:"column:sort_order;not null;default:0" json:"sort_order"`
}
