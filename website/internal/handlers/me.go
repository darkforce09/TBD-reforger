package handlers

import (
	"errors"
	"net/http"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// GetMe returns the authenticated user's profile, including Arma-link status —
// powering the TopBar avatar, the "Linked" pill, and the My Deployments header.
func (h *Handler) GetMe(c *gin.Context) {
	var user models.User
	if err := h.db.First(&user, "discord_id = ?", middleware.DiscordID(c)).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	c.JSON(http.StatusOK, gin.H{
		"user":        user,
		"arma_linked": user.ArmaID != nil,
	})
}

// UpdateMe is a placeholder for user-editable settings. The profile fields are
// sourced from Discord and the Arma link flow, so there is nothing mutable yet;
// it echoes the current user so the frontend Settings screen has a stable shape.
func (h *Handler) UpdateMe(c *gin.Context) {
	var user models.User
	if err := h.db.First(&user, "discord_id = ?", middleware.DiscordID(c)).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"user": user})
}

// linkCodeResponse is returned to the user to type into the in-game mod.
type linkCodeResponse struct {
	Code      string    `json:"code"`
	ExpiresAt time.Time `json:"expires_at"`
}

// CreateLinkCode issues a fresh 6-digit code for linking an Arma identity, after
// invalidating any of the user's prior unconsumed codes.
func (h *Handler) CreateLinkCode(c *gin.Context) {
	discordID := middleware.DiscordID(c)

	// Expire previous outstanding codes so only the newest is valid.
	h.db.Model(&models.IdentityLinkCode{}).
		Where("discord_id = ? AND consumed_at IS NULL", discordID).
		Update("expires_at", time.Now())

	// Generate a unique code (retry on the rare PK collision).
	var code string
	for attempt := 0; attempt < 5; attempt++ {
		generated, err := auth.NumericCode(6)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not generate code"})
			return
		}
		entry := models.IdentityLinkCode{
			Code:      generated,
			DiscordID: discordID,
			ExpiresAt: time.Now().Add(linkCodeTTL),
		}
		err = h.db.Create(&entry).Error
		if err == nil {
			code = generated
			break
		}
		if !strings.Contains(strings.ToLower(err.Error()), "duplicate") {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not store code"})
			return
		}
	}
	if code == "" {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not allocate code"})
		return
	}

	c.JSON(http.StatusCreated, linkCodeResponse{
		Code:      code,
		ExpiresAt: time.Now().Add(linkCodeTTL),
	})
}

// LinkStatus reports whether the caller has a linked Arma ID and whether a code
// is currently pending (so the UI can poll while the in-game mod confirms).
func (h *Handler) LinkStatus(c *gin.Context) {
	discordID := middleware.DiscordID(c)

	var user models.User
	if err := h.db.First(&user, "discord_id = ?", discordID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}

	var pending int64
	h.db.Model(&models.IdentityLinkCode{}).
		Where("discord_id = ? AND consumed_at IS NULL AND expires_at > ?", discordID, time.Now()).
		Count(&pending)

	c.JSON(http.StatusOK, gin.H{
		"linked":         user.ArmaID != nil,
		"arma_id":        user.ArmaID,
		"arma_character": user.ArmaCharacter,
		"pending_code":   pending > 0,
	})
}

// Unlink removes the Arma identity association from the caller's account.
func (h *Handler) Unlink(c *gin.Context) {
	discordID := middleware.DiscordID(c)
	if err := h.db.Model(&models.User{}).Where("discord_id = ?", discordID).
		Updates(map[string]any{"arma_id": nil, "arma_character": nil}).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not unlink"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"linked": false})
}

// linkConfirmRequest is posted by the in-game mod (service-token authenticated).
type linkConfirmRequest struct {
	Code          string `json:"code" binding:"required"`
	ArmaID        string `json:"arma_id" binding:"required"`
	ArmaCharacter string `json:"arma_character"`
}

// IngestLinkConfirm consumes a pending link code and attaches the Arma identity
// to the corresponding user. Called by the game server, not the browser.
func (h *Handler) IngestLinkConfirm(c *gin.Context) {
	var req linkConfirmRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "code and arma_id required"})
		return
	}

	now := time.Now()
	var code models.IdentityLinkCode
	err := h.db.First(&code, "code = ? AND consumed_at IS NULL AND expires_at > ?", req.Code, now).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		c.JSON(http.StatusNotFound, gin.H{"error": "invalid or expired code"})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "lookup failed"})
		return
	}

	// Guard against linking an Arma ID already owned by another account.
	var clash int64
	h.db.Model(&models.User{}).
		Where("arma_id = ? AND discord_id <> ?", req.ArmaID, code.DiscordID).
		Count(&clash)
	if clash > 0 {
		c.JSON(http.StatusConflict, gin.H{"error": "arma id already linked to another account"})
		return
	}

	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		if err := tx.Model(&models.IdentityLinkCode{}).Where("code = ?", code.Code).
			Updates(map[string]any{"consumed_at": now, "arma_id": req.ArmaID}).Error; err != nil {
			return err
		}
		return tx.Model(&models.User{}).Where("discord_id = ?", code.DiscordID).
			Updates(map[string]any{"arma_id": req.ArmaID, "arma_character": req.ArmaCharacter}).Error
	})
	if txErr != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not link identity"})
		return
	}

	var user models.User
	_ = h.db.First(&user, "discord_id = ?", code.DiscordID).Error
	_ = services.WriteAudit(h.db, models.SeverityInfo, &code.DiscordID, user.Username,
		"identity.link", user.Username+" successfully linked their Arma Steam ID", "user", code.DiscordID)

	c.JSON(http.StatusOK, gin.H{
		"linked":         true,
		"discord_id":     code.DiscordID,
		"arma_id":        req.ArmaID,
		"arma_character": req.ArmaCharacter,
	})
}
