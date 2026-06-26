package services

import (
	"errors"
	"time"

	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// SyncRoles reconciles a user's Discord role snowflakes into user_discord_roles
// and resolves their web role from the highest-priority mapped discord_roles
// entry. Unmapped role IDs are still stored, so a later admin mapping + resync
// promotes the user without needing them to log in again. Returns the resolved
// web role (defaults to enlisted when nothing maps).
func SyncRoles(db *gorm.DB, discordID string, roleIDs []string) (models.UserRole, error) {
	err := db.Transaction(func(tx *gorm.DB) error {
		// Replace the user's stored membership wholesale.
		if err := tx.Where("discord_id = ?", discordID).Delete(&models.UserDiscordRole{}).Error; err != nil {
			return err
		}
		if len(roleIDs) == 0 {
			return nil
		}
		now := time.Now()
		rows := make([]models.UserDiscordRole, 0, len(roleIDs))
		for _, rid := range roleIDs {
			rows = append(rows, models.UserDiscordRole{
				DiscordID:     discordID,
				DiscordRoleID: rid,
				SyncedAt:      now,
			})
		}
		return tx.Clauses(clause.OnConflict{DoNothing: true}).Create(&rows).Error
	})
	if err != nil {
		return "", err
	}
	return resolveRole(db, roleIDs)
}

// ResyncAllRoles re-resolves every user's web role from their already-stored
// Discord roles against the current discord_roles mappings. Used after an admin
// changes a role mapping, applying it without users needing to log in again.
// (A full re-fetch of live roles from Discord requires the bot token and is a
// separate operation.) Returns the number of users whose role changed.
func ResyncAllRoles(db *gorm.DB) (int, error) {
	var users []models.User
	if err := db.Find(&users).Error; err != nil {
		return 0, err
	}
	updated := 0
	for _, u := range users {
		var roleIDs []string
		db.Model(&models.UserDiscordRole{}).
			Where("discord_id = ?", u.DiscordID).
			Pluck("discord_role_id", &roleIDs)
		role, err := resolveRole(db, roleIDs)
		if err != nil {
			continue
		}
		if role != u.Role {
			if err := db.Model(&models.User{}).Where("discord_id = ?", u.DiscordID).
				Update("role", role).Error; err == nil {
				updated++
			}
		}
	}
	return updated, nil
}

// resolveRole returns the highest-priority mapped web role among the given
// Discord role IDs, or enlisted if none are mapped.
func resolveRole(db *gorm.DB, roleIDs []string) (models.UserRole, error) {
	if len(roleIDs) == 0 {
		return models.RoleEnlisted, nil
	}
	var dr models.DiscordRole
	err := db.
		Where("discord_role_id IN ? AND mapped_role IS NOT NULL", roleIDs).
		Order("priority DESC").
		First(&dr).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return models.RoleEnlisted, nil
	}
	if err != nil {
		return "", err
	}
	if dr.MappedRole == nil {
		return models.RoleEnlisted, nil
	}
	return *dr.MappedRole, nil
}
