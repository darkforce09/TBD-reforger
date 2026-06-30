package handlers

import (
	"net/http"
	"net/url"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// devUserID is the stable Discord snowflake used for the local dev operator.
const devUserID = "000000000000000001"

// DevLogin is a development-only shortcut that mints a session without Discord.
// It is registered only when APP_ENV=development and additionally guards itself
// at request time. It upserts a dev user (role selectable via ?role=) and then
// redirects to the SPA callback with the token fragment, exactly like the real
// Discord callback — so the frontend handles both paths identically.
//
// Visit /api/v1/auth/dev-login?role=admin (or mission_maker / enlisted).
//
// @route GET /api/v1/auth/dev-login
func (h *Handler) DevLogin(c *gin.Context) {
	if h.cfg.Env != "development" {
		c.JSON(http.StatusNotFound, gin.H{"error": "not found"})
		return
	}

	role := models.UserRole(c.DefaultQuery("role", "admin"))
	switch role {
	case models.RoleEnlisted, models.RoleLeader, models.RoleMissionMaker, models.RoleAdmin:
	default:
		role = models.RoleAdmin
	}

	now := time.Now()
	armaID := "dev-arma-76561190000000001"
	user := models.User{
		DiscordID:     devUserID,
		Username:      "Dev Operator",
		DiscordHandle: "devoperator",
		AvatarURL:     "",
		ArmaID:        &armaID,
		ArmaCharacter: "[TBD] Dev Operator",
		Role:          role,
		LastLoginAt:   &now,
	}
	if err := h.db.Clauses(clause.OnConflict{
		Columns: []clause.Column{{Name: "discord_id"}},
		DoUpdates: clause.AssignmentColumns([]string{
			"username", "discord_handle", "role", "last_login_at", "updated_at",
		}),
	}).Create(&user).Error; err != nil {
		logHandlerErr(c, "DevLogin", http.StatusInternalServerError, "dev login failed")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "dev login failed"})
		return
	}

	access, accessExp, refresh, err := h.issueSession(devUserID, string(role), true)
	if err != nil {
		logHandlerErr(c, "DevLogin", http.StatusInternalServerError, "dev login failed")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "dev login failed"})
		return
	}

	c.Redirect(http.StatusFound, authCallbackURL(h.cfg.FrontendURL, url.Values{
		"access_token":  {access},
		"refresh_token": {refresh},
		"expires_at":    {accessExp.Format(time.RFC3339)},
		"arma_linked":   {strconv.FormatBool(true)},
	}))
}
