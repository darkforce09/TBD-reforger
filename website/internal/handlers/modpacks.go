package handlers

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// modpackDTO is a modpack with its mod list embedded.
type modpackDTO struct {
	models.Modpack
	Mods []models.ModpackMod `json:"mods"`
}

// withMods loads a modpack's mods (ordered) and wraps it as a DTO.
func (h *Handler) withMods(mp models.Modpack) *modpackDTO {
	var mods []models.ModpackMod
	h.db.Where("modpack_id = ?", mp.ID).
		Order("is_key_dependency DESC").Order("sort_order ASC").
		Find(&mods)
	return &modpackDTO{Modpack: mp, Mods: mods}
}

// ListModpacks returns every modpack with its included mods (current first).
func (h *Handler) ListModpacks(c *gin.Context) {
	var packs []models.Modpack
	if err := h.db.Order("is_current DESC").Order("created_at DESC").Find(&packs).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list modpacks"})
		return
	}
	out := make([]*modpackDTO, 0, len(packs))
	for _, mp := range packs {
		out = append(out, h.withMods(mp))
	}
	c.JSON(http.StatusOK, gin.H{"data": out})
}

// GetCurrentModpack returns the active modpack for the Server Intel verify pill.
func (h *Handler) GetCurrentModpack(c *gin.Context) {
	mp, err := h.loadCurrentModpack()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load modpack"})
		return
	}
	if mp == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "no current modpack configured"})
		return
	}
	c.JSON(http.StatusOK, mp)
}
