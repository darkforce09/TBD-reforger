package handlers

import (
	"net/http"
	"strings"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

func validRole(s string) (models.UserRole, bool) {
	switch models.UserRole(s) {
	case models.RoleEnlisted, models.RoleMissionMaker, models.RoleAdmin:
		return models.UserRole(s), true
	default:
		return "", false
	}
}

// rosterRow is a Personnel Roster table row.
type rosterRow struct {
	DiscordID     string  `json:"discord_id"`
	Username      string  `json:"username"`
	DiscordHandle string  `json:"discord_handle"`
	ArmaID        *string `json:"arma_id"`
	ArmaCharacter string  `json:"arma_character"`
	Role          string  `json:"role"`
	IsBanned      bool    `json:"is_banned"`
	Warnings      int64   `json:"warnings"`
}

// ListUsers returns the Personnel Roster with per-user warning counts (admin).
func (h *Handler) ListUsers(c *gin.Context) {
	limit, offset := parsePage(c)

	q := h.db.Model(&models.User{})
	if search := strings.TrimSpace(c.Query("q")); search != "" {
		like := "%" + search + "%"
		q = q.Where("username ILIKE ? OR discord_handle ILIKE ? OR arma_character ILIKE ? OR arma_id ILIKE ?",
			like, like, like, like)
	}

	var total int64
	q.Count(&total)

	var users []models.User
	if err := q.Order("username ASC").Limit(limit).Offset(offset).Find(&users).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list users"})
		return
	}

	ids := make([]string, 0, len(users))
	for _, u := range users {
		ids = append(ids, u.DiscordID)
	}
	warnCounts := map[string]int64{}
	if len(ids) > 0 {
		type row struct {
			DiscordID string
			N         int64
		}
		var rows []row
		h.db.Model(&models.Warning{}).
			Select("discord_id, count(*) as n").
			Where("discord_id IN ?", ids).Group("discord_id").Scan(&rows)
		for _, r := range rows {
			warnCounts[r.DiscordID] = r.N
		}
	}

	out := make([]rosterRow, 0, len(users))
	for _, u := range users {
		out = append(out, rosterRow{
			DiscordID:     u.DiscordID,
			Username:      u.Username,
			DiscordHandle: u.DiscordHandle,
			ArmaID:        u.ArmaID,
			ArmaCharacter: u.ArmaCharacter,
			Role:          string(u.Role),
			IsBanned:      u.IsBanned,
			Warnings:      warnCounts[u.DiscordID],
		})
	}
	c.JSON(http.StatusOK, gin.H{"data": out, "total": total, "limit": limit, "offset": offset})
}

type updateUserInput struct {
	Role string `json:"role" binding:"required"`
}

// UpdateUser sets a user's web role (admin override of the synced role).
func (h *Handler) UpdateUser(c *gin.Context) {
	discordID := c.Param("discordId")
	var in updateUserInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "role required"})
		return
	}
	role, ok := validRole(in.Role)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid role"})
		return
	}
	res := h.db.Model(&models.User{}).Where("discord_id = ?", discordID).Update("role", role)
	if res.Error != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not update user"})
		return
	}
	if res.RowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	actor := middleware.DiscordID(c)
	_ = services.WriteAudit(h.db, models.SeverityInfo, &actor, h.username(actor),
		"user.role_change", h.username(actor)+" set "+h.username(discordID)+" role to "+string(role), "user", discordID)
	c.JSON(http.StatusOK, gin.H{"discord_id": discordID, "role": role})
}

type banInput struct {
	Reason string `json:"reason"`
}

// BanUser bans a user, revokes their refresh tokens, and records the action.
func (h *Handler) BanUser(c *gin.Context) {
	discordID := c.Param("discordId")
	var in banInput
	_ = c.ShouldBindJSON(&in)

	actor := middleware.DiscordID(c)
	now := time.Now()
	res := h.db.Model(&models.User{}).Where("discord_id = ?", discordID).Updates(map[string]any{
		"is_banned": true, "ban_reason": in.Reason, "banned_by": actor, "banned_at": now,
	})
	if res.Error != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not ban user"})
		return
	}
	if res.RowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	// Revoke active refresh tokens so the ban takes hold once the access token expires.
	h.db.Model(&models.RefreshToken{}).
		Where("discord_id = ? AND revoked_at IS NULL", discordID).
		Update("revoked_at", now)

	_ = services.WriteAudit(h.db, models.SeverityWarn, &actor, h.username(actor),
		"user.ban", h.username(actor)+" permanently banned user '"+h.username(discordID)+"'. Reason: '"+in.Reason+"'",
		"user", discordID)
	c.JSON(http.StatusOK, gin.H{"banned": true})
}

// UnbanUser lifts a ban.
func (h *Handler) UnbanUser(c *gin.Context) {
	discordID := c.Param("discordId")
	res := h.db.Model(&models.User{}).Where("discord_id = ?", discordID).Updates(map[string]any{
		"is_banned": false, "ban_reason": "", "banned_by": nil, "banned_at": nil,
	})
	if res.Error != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not unban user"})
		return
	}
	if res.RowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	actor := middleware.DiscordID(c)
	_ = services.WriteAudit(h.db, models.SeverityInfo, &actor, h.username(actor),
		"user.unban", h.username(actor)+" unbanned user '"+h.username(discordID)+"'", "user", discordID)
	c.JSON(http.StatusOK, gin.H{"banned": false})
}

type warnInput struct {
	Reason string `json:"reason" binding:"required"`
}

// IssueWarning records a disciplinary warning against a user.
func (h *Handler) IssueWarning(c *gin.Context) {
	discordID := c.Param("discordId")
	var in warnInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "reason required"})
		return
	}
	var target models.User
	if err := h.db.First(&target, "discord_id = ?", discordID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	actor := middleware.DiscordID(c)
	warning := models.Warning{DiscordID: discordID, IssuedBy: actor, Reason: in.Reason}
	if err := h.db.Create(&warning).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not issue warning"})
		return
	}
	_ = services.WriteAudit(h.db, models.SeverityWarn, &actor, h.username(actor),
		"user.warn", h.username(actor)+" warned '"+target.Username+"': "+in.Reason, "user", discordID)
	c.JSON(http.StatusCreated, warning)
}

// ResyncRoles re-applies discord_roles mappings to all users (admin).
func (h *Handler) ResyncRoles(c *gin.Context) {
	updated, err := services.ResyncAllRoles(h.db)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "resync failed"})
		return
	}
	actor := middleware.DiscordID(c)
	_ = services.WriteAudit(h.db, models.SeverityInfo, &actor, h.username(actor),
		"roles.resync", h.username(actor)+" triggered a role resync", "system", "")
	c.JSON(http.StatusOK, gin.H{"updated": updated})
}

type rconInput struct {
	Action  string `json:"action" binding:"required"` // restart | change_map | kick | custom
	Map     string `json:"map"`
	Command string `json:"command"`
}

var validRconActions = map[string]bool{"restart": true, "change_map": true, "kick": true, "custom": true}

// SendRCON records an admin server-control command. The actual BattlEye RCON
// socket dispatch is an environment-specific integration handled by the
// deployment's RCON bridge; this endpoint validates and audits the command.
func (h *Handler) SendRCON(c *gin.Context) {
	serverID := c.Param("id")
	var in rconInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "action required"})
		return
	}
	if !validRconActions[in.Action] {
		c.JSON(http.StatusBadRequest, gin.H{"error": "unknown action"})
		return
	}
	var srv models.Server
	if err := h.db.First(&srv, "id = ?", serverID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "server not found"})
		return
	}
	actor := middleware.DiscordID(c)
	detail := in.Action
	if in.Action == "change_map" && in.Map != "" {
		detail += " -> " + in.Map
	}
	_ = services.WriteAudit(h.db, models.SeverityInfo, &actor, h.username(actor),
		"server.rcon", h.username(actor)+" issued RCON '"+detail+"' on "+srv.Name, "server", serverID)

	c.JSON(http.StatusAccepted, gin.H{"accepted": true, "action": in.Action})
}
