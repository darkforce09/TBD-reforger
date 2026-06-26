package services

import (
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// WriteAudit appends a row to the audit log. Errors are returned but callers
// generally log-and-continue: an audit failure must not break the primary action.
func WriteAudit(db *gorm.DB, severity models.AuditSeverity, actorID *string, actorName, action, message, targetType, targetID string) error {
	entry := models.AuditLog{
		Severity:   severity,
		ActorID:    actorID,
		ActorName:  actorName,
		Action:     action,
		Message:    message,
		TargetType: targetType,
		TargetID:   targetID,
	}
	return db.Create(&entry).Error
}
