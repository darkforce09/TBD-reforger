package handlers

import (
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// approvalRow is a row in the Mission Approvals queue table.
type approvalRow struct {
	MissionID   string    `json:"mission_id"`
	Title       string    `json:"title"`
	Terrain     string    `json:"terrain"`
	AuthorID    string    `json:"author_id"`
	AuthorName  string    `json:"author_name"`
	SubmittedAt time.Time `json:"submitted_at"`
}

// ListApprovals returns missions awaiting review (admin only).
func (h *Handler) ListApprovals(c *gin.Context) {
	limit, offset := parsePage(c)

	base := h.db.Model(&models.Mission{}).Where("status = ?", models.MissionPendingApproval)
	var total int64
	base.Count(&total)

	var missions []models.Mission
	if err := base.Order("updated_at ASC").Limit(limit).Offset(offset).Find(&missions).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list approvals"})
		return
	}

	authorIDs := make([]string, 0, len(missions))
	for _, m := range missions {
		authorIDs = append(authorIDs, m.AuthorID)
	}
	authors := map[string]string{}
	if len(authorIDs) > 0 {
		var us []models.User
		h.db.Where("discord_id IN ?", authorIDs).Find(&us)
		for _, u := range us {
			authors[u.DiscordID] = u.Username
		}
	}

	rows := make([]approvalRow, 0, len(missions))
	for _, m := range missions {
		rows = append(rows, approvalRow{
			MissionID:   m.ID.String(),
			Title:       m.Title,
			Terrain:     string(m.Terrain),
			AuthorID:    m.AuthorID,
			AuthorName:  authors[m.AuthorID],
			SubmittedAt: m.UpdatedAt,
		})
	}
	c.JSON(http.StatusOK, gin.H{"data": rows, "total": total, "limit": limit, "offset": offset})
}

// loadPending parses :id and loads a mission that must be pending approval.
func (h *Handler) loadPending(c *gin.Context) (*models.Mission, bool) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return nil, false
	}
	var m models.Mission
	if err := h.db.First(&m, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "mission not found"})
		return nil, false
	}
	if m.Status != models.MissionPendingApproval {
		c.JSON(http.StatusConflict, gin.H{"error": "mission is not pending approval"})
		return nil, false
	}
	return &m, true
}

// ApproveMission promotes a pending mission to the live library (admin only).
func (h *Handler) ApproveMission(c *gin.Context) {
	m, ok := h.loadPending(c)
	if !ok {
		return
	}
	reviewer := middleware.DiscordID(c)
	now := time.Now()
	if err := h.db.Model(m).Updates(map[string]any{
		"status":      models.MissionLive,
		"reviewed_by": reviewer,
		"reviewed_at": now,
	}).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not approve mission"})
		return
	}

	reviewerName := h.username(reviewer)
	_ = services.WriteAudit(h.db, models.SeverityInfo, &reviewer, reviewerName,
		"mission.approve", reviewerName+" approved mission '"+m.Title+"'", "mission", m.ID.String())

	_ = h.db.First(m, "id = ?", m.ID).Error
	c.JSON(http.StatusOK, m)
}

// rejectInput carries the rejection reason shown back to the author.
type rejectInput struct {
	Reason string `json:"reason"`
}

// RejectMission sends a pending mission back to the author (admin only).
func (h *Handler) RejectMission(c *gin.Context) {
	m, ok := h.loadPending(c)
	if !ok {
		return
	}
	var in rejectInput
	_ = c.ShouldBindJSON(&in)

	reviewer := middleware.DiscordID(c)
	now := time.Now()
	if err := h.db.Model(m).Updates(map[string]any{
		"status":           models.MissionRejected,
		"rejection_reason": in.Reason,
		"reviewed_by":      reviewer,
		"reviewed_at":      now,
	}).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not reject mission"})
		return
	}

	reviewerName := h.username(reviewer)
	_ = services.WriteAudit(h.db, models.SeverityWarn, &reviewer, reviewerName,
		"mission.reject", reviewerName+" rejected mission '"+m.Title+"'", "mission", m.ID.String())

	_ = h.db.First(m, "id = ?", m.ID).Error
	c.JSON(http.StatusOK, m)
}

// username looks up a display name for audit messages, falling back to the ID.
func (h *Handler) username(discordID string) string {
	var u models.User
	if err := h.db.Select("username").First(&u, "discord_id = ?", discordID).Error; err == nil && u.Username != "" {
		return u.Username
	}
	return discordID
}
