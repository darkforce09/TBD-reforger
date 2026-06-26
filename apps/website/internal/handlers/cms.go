package handlers

import (
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

const maxUploadBytes = 5 << 20 // 5MB

var allowedImageExt = map[string]bool{".jpg": true, ".jpeg": true, ".png": true, ".webp": true}

// validTag normalizes/validates an announcement category tag.
func validTag(s string) (models.AnnouncementTag, bool) {
	switch models.AnnouncementTag(s) {
	case models.TagUpdate, models.TagEvent, models.TagModpackUpdate, models.TagImportant:
		return models.AnnouncementTag(s), true
	case "":
		return models.TagUpdate, true
	default:
		return "", false
	}
}

// snippetFrom derives a preview if the author did not supply one.
func snippetFrom(explicit, body string) string {
	if explicit != "" {
		return explicit
	}
	return services.Snippet(body, 200)
}

// announcementInput is the body for creating an announcement.
type announcementInput struct {
	Title         string `json:"title" binding:"required"`
	Body          string `json:"body" binding:"required"`
	Snippet       string `json:"snippet"`
	Tag           string `json:"tag"`
	ThumbnailURL  string `json:"thumbnail_url"`
	IsPinned      bool   `json:"is_pinned"`
	Status        string `json:"status"` // draft (default) | published
	PushToDiscord bool   `json:"push_to_discord"`
}

// CreateAnnouncement creates a draft or published announcement and optionally
// pushes it to the Discord #announcements webhook.
func (h *Handler) CreateAnnouncement(c *gin.Context) {
	var in announcementInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "title and body are required"})
		return
	}
	tag, ok := validTag(in.Tag)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid tag"})
		return
	}
	author := middleware.DiscordID(c)

	// Sanitize author-supplied HTML before it ever touches the DB so the body is
	// safe to render verbatim on the client (no stored XSS).
	body := services.SanitizeHTML(in.Body)

	a := models.Announcement{
		Title:        in.Title,
		Body:         body,
		Snippet:      snippetFrom(in.Snippet, body),
		Tag:          tag,
		ThumbnailURL: in.ThumbnailURL,
		AuthorID:     author,
		IsPinned:     in.IsPinned,
		Status:       models.AnnouncementDraft,
	}
	if in.Status == string(models.AnnouncementPublished) {
		a.Status = models.AnnouncementPublished
		now := time.Now()
		a.PublishedAt = &now
	}

	if err := h.db.Create(&a).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not create announcement"})
		return
	}

	if a.Status == models.AnnouncementPublished && in.PushToDiscord {
		h.pushToDiscord(c, &a)
	}

	name := author
	_ = services.WriteAudit(h.db, models.SeverityInfo, &author, name,
		"announcement.create", name+" created announcement '"+a.Title+"'", "announcement", a.ID.String())

	c.JSON(http.StatusCreated, a)
}

// announcementUpdate is a partial update; only present fields are applied.
type announcementUpdate struct {
	Title         *string `json:"title"`
	Body          *string `json:"body"`
	Snippet       *string `json:"snippet"`
	Tag           *string `json:"tag"`
	ThumbnailURL  *string `json:"thumbnail_url"`
	IsPinned      *bool   `json:"is_pinned"`
	Status        *string `json:"status"`
	PushToDiscord *bool   `json:"push_to_discord"`
}

// UpdateAnnouncement edits an announcement, supporting draft->published
// transitions (which set published_at and may push to Discord).
func (h *Handler) UpdateAnnouncement(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var a models.Announcement
	if err := h.db.First(&a, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "announcement not found"})
		return
	}

	var in announcementUpdate
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid body"})
		return
	}

	updates := map[string]any{}
	if in.Title != nil {
		updates["title"] = *in.Title
	}
	if in.Body != nil {
		updates["body"] = services.SanitizeHTML(*in.Body)
	}
	if in.Snippet != nil {
		updates["snippet"] = *in.Snippet
	}
	if in.Tag != nil {
		tag, ok := validTag(*in.Tag)
		if !ok {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid tag"})
			return
		}
		updates["tag"] = tag
	}
	if in.ThumbnailURL != nil {
		updates["thumbnail_url"] = *in.ThumbnailURL
	}
	if in.IsPinned != nil {
		updates["is_pinned"] = *in.IsPinned
	}

	nowPublishing := false
	if in.Status != nil {
		switch models.AnnouncementStatus(*in.Status) {
		case models.AnnouncementDraft, models.AnnouncementPublished, models.AnnouncementArchived:
			updates["status"] = *in.Status
			if models.AnnouncementStatus(*in.Status) == models.AnnouncementPublished && a.PublishedAt == nil {
				now := time.Now()
				updates["published_at"] = now
				nowPublishing = true
			}
		default:
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid status"})
			return
		}
	}

	if len(updates) > 0 {
		if err := h.db.Model(&a).Updates(updates).Error; err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not update announcement"})
			return
		}
	}
	_ = h.db.First(&a, "id = ?", id).Error

	if in.PushToDiscord != nil && *in.PushToDiscord &&
		a.Status == models.AnnouncementPublished && (nowPublishing || !a.PushedToDiscord) {
		h.pushToDiscord(c, &a)
		_ = h.db.First(&a, "id = ?", id).Error
	}

	c.JSON(http.StatusOK, a)
}

// DeleteAnnouncement archives an announcement (removes it from the feed but
// keeps it recoverable).
func (h *Handler) DeleteAnnouncement(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	res := h.db.Model(&models.Announcement{}).Where("id = ?", id).
		Update("status", models.AnnouncementArchived)
	if res.Error != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not delete announcement"})
		return
	}
	if res.RowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "announcement not found"})
		return
	}
	c.Status(http.StatusNoContent)
}

// PushAnnouncementDiscord manually (re)pushes an announcement to the webhook.
func (h *Handler) PushAnnouncementDiscord(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	if !h.webhook.Enabled() {
		c.JSON(http.StatusBadRequest, gin.H{"error": "discord webhook not configured"})
		return
	}
	var a models.Announcement
	if err := h.db.First(&a, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "announcement not found"})
		return
	}
	if ok := h.pushToDiscord(c, &a); !ok {
		c.JSON(http.StatusBadGateway, gin.H{"error": "webhook push failed"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"pushed": true})
}

// pushToDiscord pushes an announcement and records the result. On failure it
// writes a CRIT audit row and returns false. On success it persists the message
// id and pushed flag.
func (h *Handler) pushToDiscord(c *gin.Context, a *models.Announcement) bool {
	msgID, err := h.webhook.PushAnnouncement(c.Request.Context(), a)
	if err != nil {
		_ = services.WriteAudit(h.db, models.SeverityCrit, nil, "system",
			"webhook.push_failed",
			"Webhook failed to push payload to Discord channel #announcements ('"+a.Title+"')",
			"announcement", a.ID.String())
		return false
	}
	h.db.Model(&models.Announcement{}).Where("id = ?", a.ID).
		Updates(map[string]any{"pushed_to_discord": true, "discord_message_id": msgID})
	a.PushedToDiscord = true
	a.DiscordMessageID = msgID
	return true
}

// UploadImage accepts a thumbnail (multipart "file") and returns its URL.
// Local-disk storage for now; swap for S3/MinIO in production.
func (h *Handler) UploadImage(c *gin.Context) {
	file, err := c.FormFile("file")
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "file field required"})
		return
	}
	if file.Size > maxUploadBytes {
		c.JSON(http.StatusRequestEntityTooLarge, gin.H{"error": "file exceeds 5MB"})
		return
	}
	ext := strings.ToLower(filepath.Ext(file.Filename))
	if !allowedImageExt[ext] {
		c.JSON(http.StatusUnsupportedMediaType, gin.H{"error": "only JPG, PNG, WEBP allowed"})
		return
	}
	if err := os.MkdirAll(uploadDir, 0o755); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "storage unavailable"})
		return
	}
	name := uuid.NewString() + ext
	if err := c.SaveUploadedFile(file, filepath.Join(uploadDir, name)); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not save file"})
		return
	}
	c.JSON(http.StatusCreated, gin.H{"url": "/uploads/" + name})
}
