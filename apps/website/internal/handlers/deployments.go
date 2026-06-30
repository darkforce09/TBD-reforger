package handlers

import (
	"errors"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// deploymentUpcoming is an "Awaiting Deployment" card with the assigned slot.
type deploymentUpcoming struct {
	EventID        string    `json:"event_id"`
	EventMissionID string    `json:"event_mission_id"`
	Name           string    `json:"name"`
	Terrain        string    `json:"terrain"`
	StartTime      time.Time `json:"start_time"`
	State          string    `json:"state"`
	Faction        string    `json:"faction,omitempty"`
	Squad          string    `json:"squad,omitempty"`
	Role           string    `json:"role,omitempty"`
}

// serviceRecord is a past-operation row in the Service Record table.
type serviceRecord struct {
	Date         time.Time `json:"date"`
	Operation    string    `json:"operation"`
	Role         string    `json:"role"`
	Outcome      string    `json:"outcome"`
	AARReplayURL string    `json:"aar_replay_url,omitempty"`
}

// GetMyDeployments returns the player's service record: headline stats, upcoming
// operations (with assigned ORBAT slot), and past-operation history.
//
// @route GET /api/v1/me/deployments
func (h *Handler) GetMyDeployments(c *gin.Context) {
	me := middleware.DiscordID(c)
	now := time.Now()

	var user models.User
	if err := h.db.First(&user, "discord_id = ?", me).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}

	// Upcoming: my registrations on future missions within events.
	var regs []models.EventRegistration
	h.db.Joins("JOIN event_missions ON event_missions.id = event_registrations.event_mission_id").
		Joins("JOIN events ON events.id = event_missions.event_id").
		Where("event_registrations.discord_id = ? AND event_missions.start_time > ? AND events.deleted_at IS NULL", me, now).
		Order("event_missions.start_time ASC").Find(&regs)

	upcoming := make([]deploymentUpcoming, 0, len(regs))
	for _, reg := range regs {
		var em models.EventMission
		if err := h.db.First(&em, "id = ?", reg.EventMissionID).Error; err != nil {
			continue
		}
		var ev models.Event
		if err := h.db.First(&ev, "id = ?", em.EventID).Error; err != nil {
			continue
		}
		var m models.Mission
		if err := h.db.First(&m, "id = ?", em.MissionID).Error; err != nil && !errors.Is(err, gorm.ErrRecordNotFound) {
			logHandlerErr(c, "GetMyDeployments", http.StatusOK, "mission enrich lookup failed")
		}
		name := ev.NameOverride
		if name == "" {
			name = m.Title
		}
		d := deploymentUpcoming{
			EventID:        ev.ID.String(),
			EventMissionID: em.ID.String(),
			Name:           name,
			Terrain:        string(m.Terrain),
			StartTime:      em.StartTime,
			State:          string(reg.State),
		}
		// Resolve assigned slot for this mission.
		var slot models.OrbatSlot
		if err := h.db.First(&slot, "event_mission_id = ? AND assigned_to = ?", em.ID, me).Error; err == nil {
			d.Faction, d.Squad, d.Role = slot.Faction, slot.Squad, slot.Role
		}
		upcoming = append(upcoming, d)
	}

	// Service history: past match participation (populated by telemetry in M7).
	var stats []models.MatchPlayerStat
	h.db.Where("discord_id = ?", me).Order("created_at DESC").Limit(50).Find(&stats)
	history := make([]serviceRecord, 0, len(stats))
	for _, s := range stats {
		rec := serviceRecord{Role: s.RolePlayed}
		var match models.Match
		if err := h.db.First(&match, "id = ?", s.MatchID).Error; err == nil {
			rec.Date = match.StartedAt
			rec.Outcome = string(match.Outcome)
			rec.AARReplayURL = match.AARReplayURL
			if match.MissionID != nil {
				var m models.Mission
				if err := h.db.First(&m, "id = ?", *match.MissionID).Error; err == nil {
					rec.Operation = m.Title
				}
			}
		}
		history = append(history, rec)
	}

	c.JSON(http.StatusOK, gin.H{
		"total_operations": user.TotalDeployments,
		"attendance_rate":  user.AttendanceRate,
		"upcoming":         upcoming,
		"service_history":  history,
	})
}

// --- Leave of Absence ---

type createLeaveInput struct {
	StartsOn string `json:"starts_on" binding:"required"` // YYYY-MM-DD
	EndsOn   string `json:"ends_on" binding:"required"`   // YYYY-MM-DD
	Reason   string `json:"reason"`
}

// SubmitLeave files a Leave of Absence request for the caller.
//
// @route POST /api/v1/me/leave-requests
func (h *Handler) SubmitLeave(c *gin.Context) {
	var in createLeaveInput
	if err := c.ShouldBindJSON(&in); err != nil {
		logHandlerErr(c, "SubmitLeave", http.StatusBadRequest, "starts_on and ends_on are required")
		c.JSON(http.StatusBadRequest, gin.H{"error": "starts_on and ends_on are required"})
		return
	}
	start, err1 := time.Parse("2006-01-02", in.StartsOn)
	end, err2 := time.Parse("2006-01-02", in.EndsOn)
	if err1 != nil || err2 != nil {
		logHandlerErr(c, "SubmitLeave", http.StatusBadRequest, "dates must be YYYY-MM-DD")
		c.JSON(http.StatusBadRequest, gin.H{"error": "dates must be YYYY-MM-DD"})
		return
	}
	if end.Before(start) {
		logHandlerErr(c, "SubmitLeave", http.StatusBadRequest, "ends_on must be on or after starts_on")
		c.JSON(http.StatusBadRequest, gin.H{"error": "ends_on must be on or after starts_on"})
		return
	}
	loa := models.LeaveRequest{
		DiscordID: middleware.DiscordID(c),
		StartsOn:  start,
		EndsOn:    end,
		Reason:    in.Reason,
		Status:    models.LeavePending,
	}
	if err := h.db.Create(&loa).Error; err != nil {
		logHandlerErr(c, "SubmitLeave", http.StatusInternalServerError, "could not submit LOA")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not submit LOA"})
		return
	}
	c.JSON(http.StatusCreated, loa)
}

// ListMyLeave lists the caller's own LOA requests.
//
// @route GET /api/v1/me/leave-requests
func (h *Handler) ListMyLeave(c *gin.Context) {
	var loas []models.LeaveRequest
	h.db.Where("discord_id = ?", middleware.DiscordID(c)).Order("created_at DESC").Find(&loas)
	c.JSON(http.StatusOK, gin.H{"data": loas})
}

// ListAllLeave lists LOA requests for review (admin), pending first.
//
// @route GET /api/v1/admin/leave-requests
func (h *Handler) ListAllLeave(c *gin.Context) {
	limit, offset := parsePage(c)
	var loas []models.LeaveRequest
	var total int64
	h.db.Model(&models.LeaveRequest{}).Count(&total)
	h.db.Order("status::text = 'pending' DESC").Order("created_at DESC").
		Limit(limit).Offset(offset).Find(&loas)
	c.JSON(http.StatusOK, gin.H{"data": loas, "total": total, "limit": limit, "offset": offset})
}

type reviewLeaveInput struct {
	Status string `json:"status" binding:"required"` // approved | denied
}

// ReviewLeave approves or denies a LOA request (admin).
//
// @route PATCH /api/v1/admin/leave-requests/:id
func (h *Handler) ReviewLeave(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		logHandlerErr(c, "ReviewLeave", http.StatusBadRequest, "invalid id")
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var in reviewLeaveInput
	if err := c.ShouldBindJSON(&in); err != nil {
		logHandlerErr(c, "ReviewLeave", http.StatusBadRequest, "status required")
		c.JSON(http.StatusBadRequest, gin.H{"error": "status required"})
		return
	}
	var status models.LeaveStatus
	switch models.LeaveStatus(in.Status) {
	case models.LeaveApproved, models.LeaveDenied:
		status = models.LeaveStatus(in.Status)
	default:
		logHandlerErr(c, "ReviewLeave", http.StatusBadRequest, "status must be approved or denied")
		c.JSON(http.StatusBadRequest, gin.H{"error": "status must be approved or denied"})
		return
	}
	reviewer := middleware.DiscordID(c)
	res := h.db.Model(&models.LeaveRequest{}).Where("id = ?", id).
		Updates(map[string]any{"status": status, "reviewed_by": reviewer})
	if res.Error != nil {
		logHandlerErr(c, "ReviewLeave", http.StatusInternalServerError, "could not review LOA")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not review LOA"})
		return
	}
	if res.RowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "LOA not found"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": status})
}
