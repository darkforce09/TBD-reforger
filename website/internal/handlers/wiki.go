package handlers

import (
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// ListWiki returns the SOP nav list ordered for the left sidebar. The frontend
// groups by category.
func (h *Handler) ListWiki(c *gin.Context) {
	var pages []models.WikiPage
	if err := h.db.Order("nav_order ASC").Order("title ASC").Find(&pages).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list wiki"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"data": pages})
}

// GetWikiPage returns a single SOP/manual document by slug.
func (h *Handler) GetWikiPage(c *gin.Context) {
	var page models.WikiPage
	if err := h.db.First(&page, "slug = ?", c.Param("slug")).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "wiki page not found"})
		return
	}
	c.JSON(http.StatusOK, page)
}

// ListVehicles returns the structured Vehicle Database / IFF table.
func (h *Handler) ListVehicles(c *gin.Context) {
	var vehicles []models.VehicleDatabase
	if err := h.db.Order("name ASC").Find(&vehicles).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list vehicles"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"data": vehicles})
}

// wikiInput is the body for authoring a wiki page (admin).
type wikiInput struct {
	Category string `json:"category" binding:"required"`
	Title    string `json:"title" binding:"required"`
	Icon     string `json:"icon"`
	BodyMD   string `json:"body_md" binding:"required"`
	NavOrder int    `json:"nav_order"`
}

// UpsertWikiPage creates or replaces a wiki page at the given slug (admin only).
func (h *Handler) UpsertWikiPage(c *gin.Context) {
	var in wikiInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "category, title and body_md are required"})
		return
	}
	editor := middleware.DiscordID(c)
	page := models.WikiPage{
		Slug:      c.Param("slug"),
		Category:  in.Category,
		Title:     in.Title,
		Icon:      in.Icon,
		BodyMD:    in.BodyMD,
		NavOrder:  in.NavOrder,
		UpdatedBy: &editor,
		UpdatedAt: time.Now(),
	}
	if err := h.db.Clauses(clause.OnConflict{
		Columns: []clause.Column{{Name: "slug"}},
		DoUpdates: clause.AssignmentColumns([]string{
			"category", "title", "icon", "body_md", "nav_order", "updated_by", "updated_at",
		}),
	}).Create(&page).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not save wiki page"})
		return
	}
	// Reload to reflect generated/normalized fields.
	_ = h.db.First(&page, "slug = ?", page.Slug).Error
	c.JSON(http.StatusOK, page)
}

// loadCurrentModpack is shared by the dashboard and modpack endpoints.
func (h *Handler) loadCurrentModpack() (*modpackDTO, error) {
	var mp models.Modpack
	err := h.db.First(&mp, "is_current = ?", true).Error
	if err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, nil
		}
		return nil, err
	}
	return h.withMods(mp), nil
}
