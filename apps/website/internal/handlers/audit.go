package handlers

import (
	"encoding/csv"
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func validSeverity(s string) (string, bool) {
	switch s {
	case "info", "warn", "crit":
		return s, true
	default:
		return "", false
	}
}

// auditQuery applies ?severity= and ?q= filters to an audit_logs query.
func (h *Handler) auditQuery(c *gin.Context) *gorm.DB {
	q := h.db.Model(&models.AuditLog{})
	if sev := c.Query("severity"); sev != "" {
		if v, ok := validSeverity(sev); ok {
			q = q.Where("severity::text = ?", v)
		}
	}
	if search := strings.TrimSpace(c.Query("q")); search != "" {
		q = q.Where("message ILIKE ?", "%"+search+"%")
	}
	return q
}

// ListAuditLogs returns audit entries newest-first with keyset pagination via
// ?before=<id>. Returns next_cursor when a full page is returned.
func (h *Handler) ListAuditLogs(c *gin.Context) {
	limit, _ := parsePage(c)

	q := h.auditQuery(c)
	if before := c.Query("before"); before != "" {
		if id, err := strconv.ParseInt(before, 10, 64); err == nil {
			q = q.Where("id < ?", id)
		}
	}

	var logs []models.AuditLog
	if err := q.Order("id DESC").Limit(limit).Find(&logs).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list audit logs"})
		return
	}

	var nextCursor *int64
	if len(logs) == limit && limit > 0 {
		c := logs[len(logs)-1].ID
		nextCursor = &c
	}
	c.JSON(http.StatusOK, gin.H{"data": logs, "next_cursor": nextCursor})
}

// ExportAuditLogsCSV streams the filtered audit log as a CSV download.
func (h *Handler) ExportAuditLogsCSV(c *gin.Context) {
	var logs []models.AuditLog
	if err := h.auditQuery(c).Order("id DESC").Limit(10000).Find(&logs).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not export"})
		return
	}

	c.Header("Content-Type", "text/csv")
	c.Header("Content-Disposition", `attachment; filename="audit-logs.csv"`)

	w := csv.NewWriter(c.Writer)
	_ = w.Write([]string{"timestamp", "severity", "actor", "action", "message", "target_type", "target_id"})
	for _, l := range logs {
		_ = w.Write([]string{
			l.CreatedAt.UTC().Format(time.RFC3339),
			string(l.Severity),
			l.ActorName,
			l.Action,
			l.Message,
			l.TargetType,
			l.TargetID,
		})
	}
	w.Flush()
}

// StreamAuditLogs is a terminal-style live feed (SSE). It polls for rows newer
// than the latest at connect time and pushes them as they appear.
func (h *Handler) StreamAuditLogs(c *gin.Context) {
	c.Writer.Header().Set("Content-Type", "text/event-stream")
	c.Writer.Header().Set("Cache-Control", "no-cache")
	c.Writer.Header().Set("Connection", "keep-alive")
	c.Writer.Header().Set("X-Accel-Buffering", "no")

	flusher, ok := c.Writer.(interface{ Flush() })
	if !ok {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "streaming unsupported"})
		return
	}

	// Start from the current tail so the client only sees new events.
	var lastID int64
	var latest models.AuditLog
	if err := h.db.Order("id DESC").First(&latest).Error; err == nil {
		lastID = latest.ID
	}

	ctx := c.Request.Context()
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			var rows []models.AuditLog
			h.db.Where("id > ?", lastID).Order("id ASC").Limit(100).Find(&rows)
			for _, r := range rows {
				if b, err := json.Marshal(r); err == nil {
					writeSSE(c.Writer, b)
				}
				lastID = r.ID
			}
			if len(rows) > 0 {
				flusher.Flush()
			}
		}
	}
}
