package services

import (
	"log"

	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// WriteAudit appends a row to the audit log. Errors are returned but callers
// generally log-and-continue: an audit failure must not break the primary action.
// A failed write is logged here so the dropped audit trail is at least traceable
// even when the caller ignores the return (T-122 M6).
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
	if err := db.Create(&entry).Error; err != nil {
		log.Printf("audit write failed: action=%s target=%s/%s: %v", action, targetType, targetID, err)
		return err
	}
	return nil
}
