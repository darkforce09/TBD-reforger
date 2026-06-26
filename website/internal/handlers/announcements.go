package handlers

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// ListAnnouncements returns the published news feed, pinned items first then
// newest, with offset pagination for the "Load More" button.
func (h *Handler) ListAnnouncements(c *gin.Context) {
	limit, offset := parsePage(c)

	base := h.db.Model(&models.Announcement{}).Where("status = ?", models.AnnouncementPublished)

	var total int64
	base.Count(&total)

	var items []models.Announcement
	if err := base.
		Order("is_pinned DESC").
		Order("published_at DESC").
		Limit(limit).Offset(offset).
		Find(&items).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list announcements"})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"data":   items,
		"total":  total,
		"limit":  limit,
		"offset": offset,
	})
}

// GetAnnouncement returns a single published announcement (full briefing).
func (h *Handler) GetAnnouncement(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var a models.Announcement
	if err := h.db.First(&a, "id = ? AND status = ?", id, models.AnnouncementPublished).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "announcement not found"})
		return
	}
	c.JSON(http.StatusOK, a)
}
