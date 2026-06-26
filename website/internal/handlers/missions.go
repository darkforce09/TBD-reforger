package handlers

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/datatypes"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// --- enum validation helpers ---

func validTerrain(s string) (models.TerrainType, bool) {
	switch models.TerrainType(s) {
	case models.TerrainEveron, models.TerrainArland, models.TerrainCustom:
		return models.TerrainType(s), true
	default:
		return "", false
	}
}

func validGameMode(s string) (models.GameMode, bool) {
	switch models.GameMode(s) {
	case models.GameModePvECoop, models.GameModePvP, models.GameModeZeus:
		return models.GameMode(s), true
	default:
		return "", false
	}
}

func validWeather(s string) (models.WeatherType, bool) {
	switch models.WeatherType(s) {
	case models.WeatherClear, models.WeatherOvercast, models.WeatherHeavyRain, models.WeatherDenseFog:
		return models.WeatherType(s), true
	case "":
		return models.WeatherClear, true
	default:
		return "", false
	}
}

// canEditMission returns true if the caller authored the mission or is an admin.
func (h *Handler) canEditMission(c *gin.Context, m *models.Mission) bool {
	return m.AuthorID == middleware.DiscordID(c) || middleware.Role(c) == "admin"
}

// missionCard is a library list item with denormalized author + bookmark state.
type missionCard struct {
	models.Mission
	AuthorName   string `json:"author_name"`
	AuthorAvatar string `json:"author_avatar"`
	Bookmarked   bool   `json:"bookmarked"`
}

// ListMissions powers the library browser with tabs (scope) and filters.
// Query: ?scope=global|mine|bookmarked &terrain= &mode= &player_count=1-16 &q= &limit= &offset=
func (h *Handler) ListMissions(c *gin.Context) {
	me := middleware.DiscordID(c)
	limit, offset := parsePage(c)

	q := h.db.Model(&models.Mission{})

	switch c.DefaultQuery("scope", "global") {
	case "mine":
		q = q.Where("author_id = ?", me)
	case "bookmarked":
		q = q.Where("id IN (?)", h.db.Model(&models.MissionBookmark{}).
			Select("mission_id").Where("discord_id = ?", me))
	default: // global = all live missions plus the caller's own drafts/pending (mine ⊆ global)
		q = q.Where("status = ? OR author_id = ?", models.MissionLive, me)
	}

	if t := c.Query("terrain"); t != "" && t != "all" {
		if terrain, ok := validTerrain(t); ok {
			q = q.Where("terrain = ?", terrain)
		}
	}
	if m := c.Query("mode"); m != "" && m != "all" {
		if mode, ok := validGameMode(m); ok {
			q = q.Where("game_mode = ?", mode)
		}
	}
	if pc := c.Query("player_count"); pc != "" && pc != "all" {
		if lo, hi, ok := parseRange(pc); ok {
			q = q.Where("max_players >= ? AND max_players <= ?", lo, hi)
		}
	}
	if search := strings.TrimSpace(c.Query("q")); search != "" {
		q = q.Where("title ILIKE ?", "%"+search+"%")
	}

	var total int64
	q.Count(&total)

	var missions []models.Mission
	if err := q.Order("updated_at DESC").Limit(limit).Offset(offset).Find(&missions).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list missions"})
		return
	}

	cards := h.decorateMissions(me, missions)
	c.JSON(http.StatusOK, gin.H{"data": cards, "total": total, "limit": limit, "offset": offset})
}

// decorateMissions batch-loads authors and the caller's bookmarks for the page.
func (h *Handler) decorateMissions(me string, missions []models.Mission) []missionCard {
	authorIDs := make([]string, 0, len(missions))
	missionIDs := make([]uuid.UUID, 0, len(missions))
	for _, m := range missions {
		authorIDs = append(authorIDs, m.AuthorID)
		missionIDs = append(missionIDs, m.ID)
	}

	authors := map[string]models.User{}
	if len(authorIDs) > 0 {
		var us []models.User
		h.db.Where("discord_id IN ?", authorIDs).Find(&us)
		for _, u := range us {
			authors[u.DiscordID] = u
		}
	}

	bookmarked := map[uuid.UUID]bool{}
	if len(missionIDs) > 0 {
		var bs []models.MissionBookmark
		h.db.Where("discord_id = ? AND mission_id IN ?", me, missionIDs).Find(&bs)
		for _, b := range bs {
			bookmarked[b.MissionID] = true
		}
	}

	cards := make([]missionCard, 0, len(missions))
	for _, m := range missions {
		cards = append(cards, missionCard{
			Mission:      m,
			AuthorName:   authors[m.AuthorID].Username,
			AuthorAvatar: authors[m.AuthorID].AvatarURL,
			Bookmarked:   bookmarked[m.ID],
		})
	}
	return cards
}

// missionDetail is the full Mission Overview payload.
type missionDetail struct {
	missionCard
	Armory         []models.MissionArmory `json:"armory"`
	CurrentVersion *models.MissionVersion `json:"current_version,omitempty"`
}

// GetMission returns the Mission Overview: metadata, armory, and current version
// (whose json_payload carries the ORBAT template and map markers).
func (h *Handler) GetMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	me := middleware.DiscordID(c)
	detail := missionDetail{missionCard: h.decorateMissions(me, []models.Mission{*m})[0]}

	h.db.Where("mission_id = ?", m.ID).Order("sort_order ASC").Find(&detail.Armory)

	if m.CurrentVersionID != nil {
		var v models.MissionVersion
		if err := h.db.First(&v, "id = ?", *m.CurrentVersionID).Error; err == nil {
			detail.CurrentVersion = &v
		}
	}
	c.JSON(http.StatusOK, detail)
}

// createMissionInput is the Creator wizard body.
type createMissionInput struct {
	Title             string          `json:"title" binding:"required"`
	Terrain           string          `json:"terrain" binding:"required"`
	CustomTerrainName string          `json:"custom_terrain_name"`
	GameMode          string          `json:"game_mode" binding:"required"`
	Weather           string          `json:"weather"`
	TimeOfDay         string          `json:"time_of_day"`
	MaxPlayers        int             `json:"max_players" binding:"required"`
	Briefing          string          `json:"briefing"`
	Payload           json.RawMessage `json:"payload"` // optional initial canvas
}

// CreateMission creates a draft mission plus an initial v0.1.0 version and points
// the mission at it. Requires mission_maker (or admin).
func (h *Handler) CreateMission(c *gin.Context) {
	var in createMissionInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "title, terrain, game_mode and max_players are required"})
		return
	}
	terrain, ok := validTerrain(in.Terrain)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid terrain"})
		return
	}
	mode, ok := validGameMode(in.GameMode)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid game_mode"})
		return
	}
	weather, ok := validWeather(in.Weather)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid weather"})
		return
	}
	timeOfDay := in.TimeOfDay
	if timeOfDay == "" {
		timeOfDay = "14:00"
	}
	payload := datatypes.JSON([]byte("{}"))
	if len(in.Payload) > 0 {
		payload = datatypes.JSON(in.Payload)
	}

	author := middleware.DiscordID(c)
	mission := models.Mission{
		Title:             in.Title,
		AuthorID:          author,
		Terrain:           terrain,
		CustomTerrainName: in.CustomTerrainName,
		GameMode:          mode,
		Weather:           weather,
		TimeOfDay:         timeOfDay,
		MaxPlayers:        in.MaxPlayers,
		Status:            models.MissionDraft,
		Briefing:          in.Briefing,
	}

	err := h.db.Transaction(func(tx *gorm.DB) error {
		if err := tx.Create(&mission).Error; err != nil {
			return err
		}
		version := models.MissionVersion{
			MissionID:   mission.ID,
			Semver:      "0.1.0",
			JSONPayload: payload,
			CreatedBy:   author,
		}
		if err := tx.Create(&version).Error; err != nil {
			return err
		}
		return tx.Model(&mission).Update("current_version_id", version.ID).Error
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not create mission"})
		return
	}

	_ = h.db.First(&mission, "id = ?", mission.ID).Error
	c.JSON(http.StatusCreated, mission)
}

// patchMissionInput is a partial metadata update.
type patchMissionInput struct {
	Title             *string `json:"title"`
	Terrain           *string `json:"terrain"`
	CustomTerrainName *string `json:"custom_terrain_name"`
	GameMode          *string `json:"game_mode"`
	Weather           *string `json:"weather"`
	TimeOfDay         *string `json:"time_of_day"`
	MaxPlayers        *int    `json:"max_players"`
	Briefing          *string `json:"briefing"`
	ThumbnailURL      *string `json:"thumbnail_url"`
}

// UpdateMission edits mission metadata (author or admin only).
func (h *Handler) UpdateMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	if !h.canEditMission(c, m) {
		c.JSON(http.StatusForbidden, gin.H{"error": "not your mission"})
		return
	}
	var in patchMissionInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid body"})
		return
	}

	updates := map[string]any{}
	if in.Title != nil {
		updates["title"] = *in.Title
	}
	if in.Terrain != nil {
		t, ok := validTerrain(*in.Terrain)
		if !ok {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid terrain"})
			return
		}
		updates["terrain"] = t
	}
	if in.CustomTerrainName != nil {
		updates["custom_terrain_name"] = *in.CustomTerrainName
	}
	if in.GameMode != nil {
		mode, ok := validGameMode(*in.GameMode)
		if !ok {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid game_mode"})
			return
		}
		updates["game_mode"] = mode
	}
	if in.Weather != nil {
		w, ok := validWeather(*in.Weather)
		if !ok {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid weather"})
			return
		}
		updates["weather"] = w
	}
	if in.TimeOfDay != nil {
		updates["time_of_day"] = *in.TimeOfDay
	}
	if in.MaxPlayers != nil {
		updates["max_players"] = *in.MaxPlayers
	}
	if in.Briefing != nil {
		updates["briefing"] = *in.Briefing
	}
	if in.ThumbnailURL != nil {
		updates["thumbnail_url"] = *in.ThumbnailURL
	}

	if len(updates) > 0 {
		if err := h.db.Model(m).Updates(updates).Error; err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not update mission"})
			return
		}
	}
	_ = h.db.First(m, "id = ?", m.ID).Error
	c.JSON(http.StatusOK, m)
}

// createVersionInput saves a new 2D-editor snapshot.
type createVersionInput struct {
	Semver      string          `json:"semver" binding:"required"`
	Payload     json.RawMessage `json:"payload" binding:"required"`
	EditorNotes string          `json:"editor_notes"`
}

// CreateVersion stores a new mission version and makes it current (author/admin).
func (h *Handler) CreateVersion(c *gin.Context) {
	// Observability for large-payload saves (T-060.1.3): log the declared body size on entry and
	// the outcome + duration on exit, so a mid-upload failure is diagnosable from the server side.
	start := time.Now()
	log.Printf("CreateVersion: mission=%s content_length=%d", c.Param("id"), c.Request.ContentLength)

	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	if !h.canEditMission(c, m) {
		c.JSON(http.StatusForbidden, gin.H{"error": "not your mission"})
		return
	}
	var in createVersionInput
	if err := c.ShouldBindJSON(&in); err != nil {
		// A compiled mission can be hundreds of MB; the version route allows it up to
		// cfg.MissionVersionMaxBodyBytes. Over that, http.MaxBytesReader trips the read —
		// surface a clear 413 instead of a generic "required fields" 400.
		var maxErr *http.MaxBytesError
		if errors.As(err, &maxErr) || strings.Contains(err.Error(), "request body too large") {
			log.Printf("CreateVersion: mission=%s status=413 over_limit max_mb=%d dur=%s",
				c.Param("id"), h.cfg.MissionVersionBodyLimit()>>20, time.Since(start))
			c.JSON(http.StatusRequestEntityTooLarge, gin.H{
				"error": fmt.Sprintf("payload too large (max %d MB)", h.cfg.MissionVersionBodyLimit()>>20),
			})
			return
		}
		log.Printf("CreateVersion: mission=%s status=400 bind_error=%v dur=%s", c.Param("id"), err, time.Since(start))
		c.JSON(http.StatusBadRequest, gin.H{"error": "semver and payload are required"})
		return
	}

	version := models.MissionVersion{
		MissionID:   m.ID,
		Semver:      in.Semver,
		JSONPayload: datatypes.JSON(in.Payload),
		EditorNotes: in.EditorNotes,
		CreatedBy:   middleware.DiscordID(c),
	}
	if err := h.db.Create(&version).Error; err != nil {
		if strings.Contains(strings.ToLower(err.Error()), "duplicate") {
			c.JSON(http.StatusConflict, gin.H{"error": "version already exists"})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not save version"})
		return
	}
	if err := h.db.Model(m).Update("current_version_id", version.ID).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not set current version"})
		return
	}
	log.Printf("CreateVersion: mission=%s semver=%s payload_bytes=%d status=201 dur=%s",
		m.ID, in.Semver, len(in.Payload), time.Since(start))
	c.JSON(http.StatusCreated, version)
}

// GetVersion returns a specific mission version payload.
func (h *Handler) GetVersion(c *gin.Context) {
	mID, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid mission id"})
		return
	}
	vID, err := uuid.Parse(c.Param("vid"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid version id"})
		return
	}
	var v models.MissionVersion
	if err := h.db.First(&v, "id = ? AND mission_id = ?", vID, mID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "version not found"})
		return
	}
	c.JSON(http.StatusOK, v)
}

// SubmitMission moves a draft into the approval queue (author/admin).
func (h *Handler) SubmitMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	if !h.canEditMission(c, m) {
		c.JSON(http.StatusForbidden, gin.H{"error": "not your mission"})
		return
	}
	if m.Status != models.MissionDraft && m.Status != models.MissionRejected {
		c.JSON(http.StatusConflict, gin.H{"error": "only draft or rejected missions can be submitted"})
		return
	}
	if err := h.db.Model(m).Updates(map[string]any{
		"status":           models.MissionPendingApproval,
		"rejection_reason": "",
	}).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not submit mission"})
		return
	}
	_ = h.db.First(m, "id = ?", m.ID).Error
	c.JSON(http.StatusOK, m)
}

// --- Armory ---

// GetArmory lists a mission's armory.
func (h *Handler) GetArmory(c *gin.Context) {
	mID, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	var items []models.MissionArmory
	h.db.Where("mission_id = ?", mID).Order("sort_order ASC").Find(&items)
	c.JSON(http.StatusOK, gin.H{"data": items})
}

type armoryItemInput struct {
	Faction   string `json:"faction" binding:"required"`
	Category  string `json:"category" binding:"required"`
	ItemName  string `json:"item_name" binding:"required"`
	Quantity  *int   `json:"quantity"`
	Icon      string `json:"icon"`
	SortOrder int    `json:"sort_order"`
}

type setArmoryInput struct {
	Items []armoryItemInput `json:"items"`
}

// SetArmory replaces a mission's armory list wholesale (author/admin).
func (h *Handler) SetArmory(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	if !h.canEditMission(c, m) {
		c.JSON(http.StatusForbidden, gin.H{"error": "not your mission"})
		return
	}
	var in setArmoryInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid body"})
		return
	}

	err := h.db.Transaction(func(tx *gorm.DB) error {
		if err := tx.Where("mission_id = ?", m.ID).Delete(&models.MissionArmory{}).Error; err != nil {
			return err
		}
		if len(in.Items) == 0 {
			return nil
		}
		rows := make([]models.MissionArmory, 0, len(in.Items))
		for _, it := range in.Items {
			rows = append(rows, models.MissionArmory{
				MissionID: m.ID,
				Faction:   it.Faction,
				Category:  it.Category,
				ItemName:  it.ItemName,
				Quantity:  it.Quantity,
				Icon:      it.Icon,
				SortOrder: it.SortOrder,
			})
		}
		return tx.Create(&rows).Error
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not set armory"})
		return
	}

	var items []models.MissionArmory
	h.db.Where("mission_id = ?", m.ID).Order("sort_order ASC").Find(&items)
	c.JSON(http.StatusOK, gin.H{"data": items})
}

// --- Bookmarks ---

// BookmarkMission adds the mission to the caller's bookmarks (idempotent).
func (h *Handler) BookmarkMission(c *gin.Context) {
	mID, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	bm := models.MissionBookmark{DiscordID: middleware.DiscordID(c), MissionID: mID}
	if err := h.db.FirstOrCreate(&bm, "discord_id = ? AND mission_id = ?", bm.DiscordID, mID).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not bookmark"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"bookmarked": true})
}

// RemoveBookmark removes the mission from the caller's bookmarks.
func (h *Handler) RemoveBookmark(c *gin.Context) {
	mID, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return
	}
	h.db.Where("discord_id = ? AND mission_id = ?", middleware.DiscordID(c), mID).
		Delete(&models.MissionBookmark{})
	c.JSON(http.StatusOK, gin.H{"bookmarked": false})
}

// --- Export ---

// missionJSON is the strict export consumed by the game server's mission loader.
type missionJSON struct {
	SchemaVersion int             `json:"schemaVersion"`
	MissionID     string          `json:"missionId"`
	Title         string          `json:"title"`
	Terrain       string          `json:"terrain"`
	GameMode      string          `json:"gameMode"`
	Weather       string          `json:"weather"`
	TimeOfDay     string          `json:"timeOfDay"`
	MaxPlayers    int             `json:"maxPlayers"`
	Version       string          `json:"version"`
	Briefing      string          `json:"briefing,omitempty"`
	Armory        []armoryExport  `json:"armory"`
	Payload       json.RawMessage `json:"payload"`
	ExportedAt    time.Time       `json:"exportedAt"`
}

type armoryExport struct {
	Faction  string `json:"faction"`
	Category string `json:"category"`
	Item     string `json:"item"`
	Quantity *int   `json:"quantity,omitempty"`
}

// buildMissionDoc assembles the strict mission.json document from a mission, its
// current version payload, and its armory. Shared by export and injection.
func (h *Handler) buildMissionDoc(m *models.Mission) missionJSON {
	payload := json.RawMessage("{}")
	version := "0.0.0"
	if m.CurrentVersionID != nil {
		var v models.MissionVersion
		if err := h.db.First(&v, "id = ?", *m.CurrentVersionID).Error; err == nil {
			payload = json.RawMessage(v.JSONPayload)
			version = v.Semver
		}
	}

	var armory []models.MissionArmory
	h.db.Where("mission_id = ?", m.ID).Order("sort_order ASC").Find(&armory)
	exportArmory := make([]armoryExport, 0, len(armory))
	for _, a := range armory {
		exportArmory = append(exportArmory, armoryExport{
			Faction: a.Faction, Category: a.Category, Item: a.ItemName, Quantity: a.Quantity,
		})
	}

	terrainName := string(m.Terrain)
	if m.Terrain == models.TerrainCustom && m.CustomTerrainName != "" {
		terrainName = m.CustomTerrainName
	}

	return missionJSON{
		SchemaVersion: 1,
		MissionID:     m.ID.String(),
		Title:         m.Title,
		Terrain:       terrainName,
		GameMode:      string(m.GameMode),
		Weather:       string(m.Weather),
		TimeOfDay:     m.TimeOfDay,
		MaxPlayers:    m.MaxPlayers,
		Version:       version,
		Briefing:      m.Briefing,
		Armory:        exportArmory,
		Payload:       payload,
		ExportedAt:    time.Now().UTC(),
	}
}

// ExportMission returns the strict mission.json as a file download.
func (h *Handler) ExportMission(c *gin.Context) {
	m, ok := h.loadMission(c)
	if !ok {
		return
	}
	c.Header("Content-Disposition", `attachment; filename="mission.json"`)
	c.IndentedJSON(http.StatusOK, h.buildMissionDoc(m))
}

// --- shared helpers ---

// loadMission parses :id and loads the mission, writing the error response on
// failure. Returns (mission, true) on success.
func (h *Handler) loadMission(c *gin.Context) (*models.Mission, bool) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return nil, false
	}
	var m models.Mission
	if err := h.db.First(&m, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "mission not found"})
		return nil, false
	}
	return &m, true
}

// parseRange parses a "lo-hi" player-count bucket like "17-32".
func parseRange(s string) (lo, hi int, ok bool) {
	parts := strings.SplitN(s, "-", 2)
	if len(parts) != 2 {
		return 0, 0, false
	}
	lo, err1 := strconv.Atoi(strings.TrimSpace(parts[0]))
	hi, err2 := strconv.Atoi(strings.TrimSpace(parts[1]))
	if err1 != nil || err2 != nil || lo > hi {
		return 0, 0, false
	}
	return lo, hi, true
}
