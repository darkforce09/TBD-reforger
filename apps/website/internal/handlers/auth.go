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
//
// @route GET /api/v1/auth/discord/login
func (h *Handler) DiscordLogin(c *gin.Context) {
	state, err := auth.RandomToken(16)
	if err != nil {
		logHandlerErr(c, "DiscordLogin", http.StatusInternalServerError, "could not start login")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not start login"})
		return
	}
	// A blank client_id would 302 the user onto an opaque Discord error page —
	// surface the misconfiguration through the SPA instead (T-130.2 F3-03).
	authorizeURL, err := h.discord.AuthorizeURL(state)
	if err != nil {
		logHandlerErr(c, "DiscordLogin", http.StatusFound, "discord client_id not configured")
		h.redirectAuthError(c, "oauth_unconfigured")
		return
	}
	// 10-minute, httpOnly, SameSite=Lax state cookie to defend against CSRF on the callback
	// (Lax still rides the top-level OAuth redirect back from Discord). Secure outside dev.
	c.SetSameSite(http.SameSiteLaxMode)
	c.SetCookie("oauth_state", state, 600, "/", "", h.cfg.Env != "development", true)
	c.Redirect(http.StatusTemporaryRedirect, authorizeURL)
}

// DiscordCallback completes the flow: validate state, exchange the code, upsert
// the user, sync roles, then redirect the browser back to the SPA callback with
// the tokens in the URL fragment. The SPA parses the fragment, stores the tokens,
// and calls GET /me for the user object.
//
// @route GET /api/v1/auth/discord/callback
func (h *Handler) DiscordCallback(c *gin.Context) {
	code := c.Query("code")
	state := c.Query("state")
	if code == "" || state == "" {
		h.redirectAuthError(c, "missing_code")
		return
	}
	//nolint:errcheck // best-effort: a missing oauth_state cookie is handled by the empty-string check below.
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

	//nolint:errcheck // best-effort: audit log is non-blocking; a failed write must not fail the request.
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
//
// @route POST /api/v1/auth/refresh
func (h *Handler) Refresh(c *gin.Context) {
	var req refreshRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		logHandlerErr(c, "Refresh", http.StatusBadRequest, "refresh_token required")
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
		logHandlerErr(c, "Refresh", http.StatusInternalServerError, "lookup failed")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "lookup failed"})
		return
	}
	// Presenting an already-revoked token is a reuse signal (the legitimate client
	// received a new token at rotation; only a replayed/stolen copy comes back) —
	// revoke the whole family so neither branch of the fork stays valid (T-126 S2).
	if rt.RevokedAt != nil {
		h.revokeTokenFamily(c, "Refresh", rt.DiscordID)
		c.JSON(http.StatusUnauthorized, gin.H{"error": "refresh token reuse detected"})
		return
	}
	if time.Now().After(rt.ExpiresAt) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "expired refresh token"})
		return
	}

	var user models.User
	if err := h.db.First(&user, "discord_id = ?", rt.DiscordID).Error; err != nil {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "user not found"})
		return
	}
	// Belt-and-braces with BanUser's own revocation (T-126 S4): a banned user must
	// not be able to keep a session alive through rotation.
	if user.IsBanned {
		h.revokeTokenFamily(c, "Refresh", rt.DiscordID)
		c.JSON(http.StatusForbidden, gin.H{"error": "account is banned"})
		return
	}

	// Rotate atomically: only the request that actually flips revoked_at wins. A
	// concurrent double-spend of the same token loses the conditional UPDATE and is
	// treated as reuse — family revoked, both callers re-authenticate (T-126 S2).
	res := h.db.Model(&models.RefreshToken{}).
		Where("id = ? AND revoked_at IS NULL", rt.ID).
		Update("revoked_at", time.Now())
	if res.Error != nil {
		logHandlerErr(c, "Refresh", http.StatusInternalServerError, "rotation failed")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "rotation failed"})
		return
	}
	if res.RowsAffected != 1 {
		h.revokeTokenFamily(c, "Refresh", rt.DiscordID)
		c.JSON(http.StatusUnauthorized, gin.H{"error": "refresh token reuse detected"})
		return
	}
	armaLinked := user.ArmaID != nil
	access, accessExp, err := h.jwt.IssueAccess(user.DiscordID, string(user.Role), armaLinked)
	if err != nil {
		logHandlerErr(c, "Refresh", http.StatusInternalServerError, "could not issue token")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not issue token"})
		return
	}
	newRefresh, err := h.issueRefresh(user.DiscordID)
	if err != nil {
		logHandlerErr(c, "Refresh", http.StatusInternalServerError, "could not issue refresh token")
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
//
// @route POST /api/v1/auth/logout
func (h *Handler) Logout(c *gin.Context) {
	var req refreshRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		logHandlerErr(c, "Logout", http.StatusBadRequest, "refresh_token required")
		c.JSON(http.StatusBadRequest, gin.H{"error": "refresh_token required"})
		return
	}
	hash := auth.HashToken(req.RefreshToken)
	h.db.Model(&models.RefreshToken{}).
		Where("token_hash = ? AND revoked_at IS NULL", hash).
		Update("revoked_at", time.Now())
	c.Status(http.StatusNoContent)
}

// revokeTokenFamily revokes every active refresh token for a user — the response
// to a detected token reuse (rotation double-spend / replay) or a banned account
// presenting a still-valid token. Best-effort: the caller's 401/403 stands even if
// the sweep fails, but a failure is logged since it leaves live tokens behind.
func (h *Handler) revokeTokenFamily(c *gin.Context, handler, discordID string) {
	if err := h.db.Model(&models.RefreshToken{}).
		Where("discord_id = ? AND revoked_at IS NULL", discordID).
		Update("revoked_at", time.Now()).Error; err != nil {
		logHandlerErr(c, handler, http.StatusUnauthorized, "token family revocation failed: "+err.Error())
	}
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
