package handlers

import (
	"encoding/json"
	"net/http"
	"os"
	"path/filepath"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// missionStageDir is where injected mission.json files are written for the game
// server's bridge to pick up (e.g. a shared volume). Configurable in production.
const missionStageDir = "missions"

// --- Mortar fire missions ---

// solveInput carries the firing position and target in game-world meters.
type solveInput struct {
	WeaponSystem string  `json:"weapon_system"`
	FPX          float64 `json:"fp_x"`
	FPY          float64 `json:"fp_y"`
	TgtX         float64 `json:"tgt_x"`
	TgtY         float64 `json:"tgt_y"`
}

// SolveFire computes a firing solution without persisting it (live recompute
// while dragging markers on the map).
func (h *Handler) SolveFire(c *gin.Context) {
	var in solveInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid body"})
		return
	}
	sol, ok := services.SolveFireMission(in.WeaponSystem, in.FPX, in.FPY, in.TgtX, in.TgtY)
	if !ok {
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": "target out of range", "solution": sol})
		return
	}
	c.JSON(http.StatusOK, sol)
}

// saveFireInput is solveInput plus the grid labels and optional event link.
type saveFireInput struct {
	solveInput
	EventID    *string `json:"event_id"`
	FPGrid     string  `json:"fp_grid" binding:"required"`
	TargetGrid string  `json:"target_grid" binding:"required"`
}

// SaveFire computes and persists a fire mission, optionally shared on an event.
func (h *Handler) SaveFire(c *gin.Context) {
	var in saveFireInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "fp_grid and target_grid are required"})
		return
	}
	sol, ok := services.SolveFireMission(in.WeaponSystem, in.FPX, in.FPY, in.TgtX, in.TgtY)
	if !ok {
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": "target out of range", "solution": sol})
		return
	}

	fm := models.FireMission{
		CreatedBy:     middleware.DiscordID(c),
		WeaponSystem:  sol.WeaponSystem,
		FPGrid:        in.FPGrid,
		TargetGrid:    in.TargetGrid,
		DistanceM:     sol.DistanceM,
		AzimuthDeg:    sol.AzimuthDeg,
		ElevationMils: sol.ElevationMils,
	}
	if eid := parseUUIDPtr(in.EventID); eid != nil {
		fm.EventID = eid
	}
	if err := h.db.Create(&fm).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not save fire mission"})
		return
	}
	c.JSON(http.StatusCreated, gin.H{"solution": sol, "fire_mission": fm})
}

// ListEventFireMissions returns saved fire missions shared on an event.
func (h *Handler) ListEventFireMissions(c *gin.Context) {
	eid, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var fms []models.FireMission
	h.db.Where("event_id = ?", eid).Order("created_at ASC").Find(&fms)
	c.JSON(http.StatusOK, gin.H{"data": fms})
}

// --- Mission injection ---

// InjectMission writes the mission.json into the staging directory for the game
// server's bridge to deploy, and records the action (admin only). Returns the
// staged file path; the actual transport to the live server is handled by the
// deployment's file/RCON bridge.
func (h *Handler) InjectMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	if m.Status != models.MissionLive {
		c.JSON(http.StatusConflict, gin.H{"error": "only live missions can be injected"})
		return
	}

	doc := h.buildMissionDoc(m)
	data, err := json.MarshalIndent(doc, "", "  ")
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not build mission.json"})
		return
	}
	if err := os.MkdirAll(missionStageDir, 0o755); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "staging unavailable"})
		return
	}
	path := filepath.Join(missionStageDir, m.ID.String()+".mission.json")
	if err := os.WriteFile(path, data, 0o644); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not stage mission"})
		return
	}

	actor := middleware.DiscordID(c)
	_ = services.WriteAudit(h.db, models.SeverityInfo, &actor, h.username(actor),
		"mission.inject", h.username(actor)+" injected mission '"+m.Title+"' to the server staging directory",
		"mission", m.ID.String())

	c.JSON(http.StatusAccepted, gin.H{"staged_path": path, "version": doc.Version})
}
