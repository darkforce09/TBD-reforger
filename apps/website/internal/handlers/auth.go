package handlers

import (
	"errors"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// DiscordLogin starts the OAuth2 flow: it sets a short-lived state cookie and
// redirects the browser to Discord's consent screen.
func (h *Handler) DiscordLogin(c *gin.Context) {
	state, err := auth.RandomToken(16)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not start login"})
		return
	}
	// 10-minute, httpOnly state cookie to defend against CSRF on the callback.
	c.SetCookie("oauth_state", state, 600, "/", "", false, true)
	c.Redirect(http.StatusTemporaryRedirect, h.discord.AuthorizeURL(state))
}

// DiscordCallback completes the flow: validate state, exchange the code, upsert
// the user, sync roles, then redirect the browser back to the SPA callback with
// the tokens in the URL fragment. The SPA parses the fragment, stores the tokens,
// and calls GET /me for the user object.
func (h *Handler) DiscordCallback(c *gin.Context) {
	code := c.Query("code")
	state := c.Query("state")
	if code == "" || state == "" {
		h.redirectAuthError(c, "missing_code")
		return
	}
	cookieState, _ := c.Cookie("oauth_state")
	if cookieState == "" || !auth.ConstantTimeEqual(state, cookieState) {
		h.redirectAuthError(c, "invalid_state")
		return
	}
	c.SetCookie("oauth_state", "", -1, "/", "", false, true) // clear

	ctx := c.Request.Context()
	tok, err := h.discord.ExchangeCode(ctx, code)
	if err != nil {
		h.redirectAuthError(c, "discord_unreachable")
		return
	}
	du, err := h.discord.FetchUser(ctx, tok.AccessToken)
	if err != nil {
		h.redirectAuthError(c, "discord_unreachable")
		return
	}

	// Member roles drive the web role; tolerate non-members (nil) as enlisted.
	var roleIDs []string
	if member, mErr := h.discord.FetchGuildMember(ctx, tok.AccessToken); mErr == nil && member != nil {
		roleIDs = member.Roles
	}

	// Upsert the user record from the fresh Discord profile.
	now := time.Now()
	user := models.User{
		DiscordID:     du.ID,
		Username:      du.DisplayName(),
		DiscordHandle: du.Handle(),
		AvatarURL:     du.AvatarURL(),
		LastLoginAt:   &now,
	}
	if err := h.db.Clauses(clause.OnConflict{
		Columns: []clause.Column{{Name: "discord_id"}},
		DoUpdates: clause.AssignmentColumns([]string{
			"username", "discord_handle", "avatar_url", "last_login_at", "updated_at",
		}),
	}).Create(&user).Error; err != nil {
		h.redirectAuthError(c, "server_error")
		return
	}

	role, err := services.SyncRoles(h.db, du.ID, roleIDs)
	if err != nil {
		h.redirectAuthError(c, "server_error")
		return
	}
	if err := h.db.Model(&models.User{}).Where("discord_id = ?", du.ID).
		Update("role", role).Error; err != nil {
		h.redirectAuthError(c, "server_error")
		return
	}

	// Reload to learn current Arma-link + ban state.
	var fresh models.User
	if err := h.db.First(&fresh, "discord_id = ?", du.ID).Error; err != nil {
		h.redirectAuthError(c, "server_error")
		return
	}
	if fresh.IsBanned {
		h.redirectAuthError(c, "banned")
		return
	}
	armaLinked := fresh.ArmaID != nil

	access, accessExp, refresh, err := h.issueSession(du.ID, string(role), armaLinked)
	if err != nil {
		h.redirectAuthError(c, "server_error")
		return
	}

	_ = services.WriteAudit(h.db, models.SeverityInfo, &du.ID, fresh.Username,
		"auth.login", fresh.Username+" signed in via Discord", "user", du.ID)

	c.Redirect(http.StatusFound, authCallbackURL(h.cfg.FrontendURL, url.Values{
		"access_token":  {access},
		"refresh_token": {refresh},
		"expires_at":    {accessExp.Format(time.RFC3339)},
		"arma_linked":   {strconv.FormatBool(armaLinked)},
	}))
}

// issueSession mints a fresh access + refresh token pair for a user.
func (h *Handler) issueSession(discordID, role string, armaLinked bool) (string, time.Time, string, error) {
	access, exp, err := h.jwt.IssueAccess(discordID, role, armaLinked)
	if err != nil {
		return "", time.Time{}, "", err
	}
	refresh, err := h.issueRefresh(discordID)
	if err != nil {
		return "", time.Time{}, "", err
	}
	return access, exp, refresh, nil
}

// authCallbackURL builds the SPA callback URL with the given values placed in
// the URL fragment (kept out of query strings so tokens aren't logged upstream).
func authCallbackURL(frontendURL string, vals url.Values) string {
	return strings.TrimRight(frontendURL, "/") + "/auth/callback#" + vals.Encode()
}

// redirectAuthError sends the browser back to the SPA callback with an error code.
func (h *Handler) redirectAuthError(c *gin.Context, reason string) {
	c.Redirect(http.StatusFound, authCallbackURL(h.cfg.FrontendURL, url.Values{"error": {reason}}))
}

// refreshRequest is the body for /auth/refresh and /auth/logout.
type refreshRequest struct {
	RefreshToken string `json:"refresh_token" binding:"required"`
}

// Refresh rotates a valid refresh token: the presented token is revoked and a
// new access + refresh pair is issued.
func (h *Handler) Refresh(c *gin.Context) {
	var req refreshRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "refresh_token required"})
		return
	}
	hash := auth.HashToken(req.RefreshToken)

	var rt models.RefreshToken
	err := h.db.First(&rt, "token_hash = ?", hash).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "invalid refresh token"})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "lookup failed"})
		return
	}
	if rt.RevokedAt != nil || time.Now().After(rt.ExpiresAt) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "expired or revoked refresh token"})
		return
	}

	// Rotate: revoke the old token.
	revokedAt := time.Now()
	if err := h.db.Model(&models.RefreshToken{}).Where("id = ?", rt.ID).
		Update("revoked_at", revokedAt).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "rotation failed"})
		return
	}

	var user models.User
	if err := h.db.First(&user, "discord_id = ?", rt.DiscordID).Error; err != nil {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "user not found"})
		return
	}
	armaLinked := user.ArmaID != nil
	access, accessExp, err := h.jwt.IssueAccess(user.DiscordID, string(user.Role), armaLinked)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not issue token"})
		return
	}
	newRefresh, err := h.issueRefresh(user.DiscordID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not issue refresh token"})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"access_token":  access,
		"expires_at":    accessExp,
		"refresh_token": newRefresh,
		"token_type":    "Bearer",
	})
}

// Logout revokes the presented refresh token. Always returns 204, even if the
// token was unknown, to avoid leaking which tokens exist.
func (h *Handler) Logout(c *gin.Context) {
	var req refreshRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "refresh_token required"})
		return
	}
	hash := auth.HashToken(req.RefreshToken)
	h.db.Model(&models.RefreshToken{}).
		Where("token_hash = ? AND revoked_at IS NULL", hash).
		Update("revoked_at", time.Now())
	c.Status(http.StatusNoContent)
}

// issueRefresh creates and stores a new opaque refresh token (hashed) and
// returns the raw value to the caller.
func (h *Handler) issueRefresh(discordID string) (string, error) {
	raw, err := auth.RandomToken(32)
	if err != nil {
		return "", err
	}
	rt := models.RefreshToken{
		DiscordID: discordID,
		TokenHash: auth.HashToken(raw),
		ExpiresAt: time.Now().Add(refreshTTL),
	}
	if err := h.db.Create(&rt).Error; err != nil {
		return "", err
	}
	return raw, nil
}
