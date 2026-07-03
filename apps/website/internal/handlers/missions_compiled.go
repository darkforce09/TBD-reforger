package handlers

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// GetCompiledMission serves the mod-native mission document (mission.schema.json,
// string schemaVersion "1.1"/"1.2") the game server's TBD_MissionLoader consumes —
// flattened on demand from the mission row + its current version's editor payload by
// services.FlattenToModDocument (T-092.2). This is NOT the camelCase buildMissionDoc
// export wrapper; the body is the canonical document itself, byte-compatible with the
// loader's $profile:missions/{id}.json cache.
//
// Auth is the game-server tier (X-Service-Token), same as /ingest — no user session.
// Smoke test against a dev stack:
//
//	curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
//	  http://localhost:8080/api/v1/missions/{id}/compiled | jq .schemaVersion
//
// A mission without a saved version, or whose version has no placed slots, cannot
// produce a valid 1.1/1.2 document (slots[] is required) — both return 409 so the
// game server sees a deliberate "not compilable yet" instead of an invalid body.
//
// @route GET /api/v1/missions/:id/compiled
// @contract mission.schema.json#/
func (h *Handler) GetCompiledMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}

	if m.CurrentVersionID == nil {
		c.JSON(http.StatusConflict, gin.H{"error": "mission has no saved version to compile"})
		return
	}
	var v models.MissionVersion
	if err := h.db.First(&v, "id = ?", *m.CurrentVersionID).Error; err != nil {
		logHandlerErr(c, "GetCompiledMission", http.StatusInternalServerError, "could not load current version: "+err.Error())
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load mission version"})
		return
	}

	doc, err := services.FlattenToModDocument(m, v.JSONPayload)
	if err != nil {
		if errors.Is(err, services.ErrNoSlots) {
			c.JSON(http.StatusConflict, gin.H{"error": "mission version has no placed slots to compile"})
			return
		}
		logHandlerErr(c, "GetCompiledMission", http.StatusInternalServerError, "could not compile mission document: "+err.Error())
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not compile mission document"})
		return
	}

	c.JSON(http.StatusOK, doc)
}
