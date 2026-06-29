package handlers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// ListRegistry returns a modpack's flat Virtual Arsenal catalog (T-068).
//
// @route GET /api/v1/registry
// @contract registry-items.schema.json#/$defs/item (each row in "data")
//
// GET /api/v1/registry?modpack=<uuid> — mission_maker+ (wired on the mm group).
// The modpack query param is optional; when omitted the current (is_current)
// modpack is used. Unknown/unparseable modpack -> 404.
//
// Response: { data, etag, modpack_id, modpack_version }. A weak ETag derived from
// modpack_id + row count + max(updated_at) supports If-None-Match -> 304.
func (h *Handler) ListRegistry(c *gin.Context) {
	// Resolve the modpack: explicit ?modpack= or the current one.
	var mp models.Modpack
	if raw := c.Query("modpack"); raw != "" {
		id, err := uuid.Parse(raw)
		if err != nil {
			c.JSON(http.StatusNotFound, gin.H{"error": "modpack not found"})
			return
		}
		if err := h.db.First(&mp, "id = ?", id).Error; err != nil {
			if err == gorm.ErrRecordNotFound {
				c.JSON(http.StatusNotFound, gin.H{"error": "modpack not found"})
				return
			}
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load modpack"})
			return
		}
	} else {
		if err := h.db.First(&mp, "is_current = ?", true).Error; err != nil {
			if err == gorm.ErrRecordNotFound {
				c.JSON(http.StatusNotFound, gin.H{"error": "no current modpack configured"})
				return
			}
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load modpack"})
			return
		}
	}

	var items []models.RegistryItem
	if err := h.db.Where("modpack_id = ?", mp.ID).
		Order("sort_order ASC").Order("display_name ASC").
		Find(&items).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list registry items"})
		return
	}

	// Weak ETag: modpack_id + row count + newest updated_at. Cheap to compute and
	// changes whenever the catalog is re-imported (upserts bump updated_at).
	var maxUpdated int64
	for _, it := range items {
		if u := it.UpdatedAt.UnixNano(); u > maxUpdated {
			maxUpdated = u
		}
	}
	etag := fmt.Sprintf(`W/"%s-%d-%d"`, mp.ID, len(items), maxUpdated)

	if match := c.GetHeader("If-None-Match"); match == etag {
		c.Header("ETag", etag)
		c.Status(http.StatusNotModified)
		return
	}

	c.Header("ETag", etag)
	c.JSON(http.StatusOK, gin.H{
		"data":            items,
		"etag":            etag,
		"modpack_id":      mp.ID,
		"modpack_version": mp.Version,
	})
}
