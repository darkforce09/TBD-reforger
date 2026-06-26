package handlers

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// serverIntelDTO is the full Server Intel card: server config + live status +
// the required modpack (for the connection panel).
type serverIntelDTO struct {
	models.Server
	Status          *models.ServerStatus `json:"status"`
	RequiredModpack *modpackDTO          `json:"required_modpack,omitempty"`
}

// ListServers returns all servers with their current status summary.
func (h *Handler) ListServers(c *gin.Context) {
	var servers []models.Server
	if err := h.db.Order("name ASC").Find(&servers).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list servers"})
		return
	}
	out := make([]serverIntelDTO, 0, len(servers))
	for _, s := range servers {
		out = append(out, h.serverIntel(s))
	}
	c.JSON(http.StatusOK, gin.H{"data": out})
}

// GetServerStatus returns the Server Intel card for one server.
func (h *Handler) GetServerStatus(c *gin.Context) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var s models.Server
	if err := h.db.First(&s, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "server not found"})
		return
	}
	c.JSON(http.StatusOK, h.serverIntel(s))
}

// serverIntel composes a server with its status and required modpack.
func (h *Handler) serverIntel(s models.Server) serverIntelDTO {
	dto := serverIntelDTO{Server: s}

	var status models.ServerStatus
	if err := h.db.First(&status, "server_id = ?", s.ID).Error; err == nil {
		dto.Status = &status
	}

	if s.RequiredModpackID != nil {
		var mp models.Modpack
		if err := h.db.First(&mp, "id = ?", *s.RequiredModpackID).Error; err == nil {
			dto.RequiredModpack = h.withMods(mp)
		} else if err != gorm.ErrRecordNotFound {
			// non-fatal; leave modpack nil
		}
	}
	return dto
}
