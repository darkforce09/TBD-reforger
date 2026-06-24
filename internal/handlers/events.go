package handlers

import (
	"errors"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// Sentinel errors used to map registration transaction failures to HTTP codes.
var (
	errBadSlot       = errors.New("invalid slot id")
	errSlotNotFound  = errors.New("slot not found")
	errSlotTaken     = errors.New("slot taken")
	errSquadReserved = errors.New("squad reserved")
)

func validEventStatus(s string) (models.EventStatus, bool) {
	switch models.EventStatus(s) {
	case models.EventScheduled, models.EventOpen, models.EventLocked,
		models.EventLive, models.EventCompleted, models.EventCancelled:
		return models.EventStatus(s), true
	case "":
		return models.EventScheduled, true
	default:
		return "", false
	}
}

// canRegisterStatus reports whether registration is permitted for a status.
func canRegisterStatus(s models.EventStatus) bool {
	return s == models.EventScheduled || s == models.EventOpen
}

// The ORBAT template types live in the services package (shared with the payload
// parser/deriver); aliased here so the rest of this file is unchanged.
type orbatSlotTemplate = services.OrbatSlotTemplate
type orbatSquadTemplate = services.OrbatSquadTemplate

// parseOrbatTemplate resolves the ORBAT squad list from a mission version payload —
// an explicit top-level "orbat" wins, otherwise it's derived from the editor graph
// (Save Version omits the redundant orbat as of T-062.1.1). Thin wrapper over the
// shared services parser.
func parseOrbatTemplate(payload []byte) []orbatSquadTemplate {
	return services.ParseOrbatTemplate(payload)
}

// materializeSlots expands the parsed squads into OrbatSlot records for one
// mission within an event. slot_index is the slot's 0-based position within its
// squad, preserving the authored order and satisfying the unique constraint.
func materializeSlots(tx *gorm.DB, eventMissionID uuid.UUID, squads []orbatSquadTemplate) error {
	rows := make([]models.OrbatSlot, 0)
	for _, sq := range squads {
		for i, sl := range sq.Slots {
			rows = append(rows, models.OrbatSlot{
				EventMissionID: eventMissionID,
				Faction:        sq.Faction,
				Callsign:       sq.Callsign,
				Squad:          sq.Squad,
				Role:           sl.Role,
				Loadout:        sl.Loadout,
				Tag:            sl.Tag,
				SlotIndex:      i,
			})
		}
	}
	if len(rows) == 0 {
		return nil
	}
	return tx.Create(&rows).Error
}

// orbatTemplateForMission resolves a mission's ORBAT template from its current
// published version payload.
func (h *Handler) orbatTemplateForMission(m *models.Mission) []orbatSquadTemplate {
	if m.CurrentVersionID == nil {
		return nil
	}
	var v models.MissionVersion
	if err := h.db.First(&v, "id = ?", *m.CurrentVersionID).Error; err != nil {
		return nil
	}
	return parseOrbatTemplate(v.JSONPayload)
}

// --- Event container CRUD ---

// createEventInput is the Event Manager "Schedule Operation" body. An event is
// now a container; missions are attached separately via AddEventMission.
type createEventInput struct {
	StartTime          time.Time `json:"start_time" binding:"required"`
	NameOverride       string    `json:"name_override"`
	Briefing           string    `json:"briefing"`
	BannerImageURL     string    `json:"banner_image_url"`
	MaxSlots           int       `json:"max_slots"`
	RegistrationLocked bool      `json:"registration_locked"`
	Status             string    `json:"status"`
}

// CreateEvent schedules an operation container (admin only). ORBAT slots are
// materialized per mission when missions are attached.
func (h *Handler) CreateEvent(c *gin.Context) {
	var in createEventInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "start_time is required"})
		return
	}
	status, ok := validEventStatus(in.Status)
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid status"})
		return
	}

	event := models.Event{
		NameOverride:       in.NameOverride,
		StartTime:          in.StartTime,
		Briefing:           in.Briefing,
		BannerImageURL:     in.BannerImageURL,
		Status:             status,
		RegistrationLocked: in.RegistrationLocked,
		MaxSlots:           in.MaxSlots,
		CreatedBy:          middleware.DiscordID(c),
	}
	if err := h.db.Create(&event).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not create event"})
		return
	}
	c.JSON(http.StatusCreated, event)
}

// addMissionInput attaches a mission to an event with its own start time. An
// explicit orbat overrides the mission payload; otherwise the payload is parsed.
type addMissionInput struct {
	MissionID string               `json:"mission_id" binding:"required"`
	StartTime time.Time            `json:"start_time" binding:"required"`
	Orbat     []orbatSquadTemplate `json:"orbat"`
}

// AddEventMission attaches a mission to an event and auto-materializes its ORBAT
// from the mission.json payload (admin only).
func (h *Handler) AddEventMission(c *gin.Context) {
	ev, ok := h.loadEvent(c)
	if !ok {
		return
	}
	var in addMissionInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "mission_id and start_time are required"})
		return
	}
	missionID, err := uuid.Parse(in.MissionID)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid mission_id"})
		return
	}
	var mission models.Mission
	if err := h.db.First(&mission, "id = ?", missionID).Error; err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "mission not found"})
		return
	}

	// Resolve the ORBAT template: explicit input wins, else the mission payload.
	template := in.Orbat
	if len(template) == 0 {
		template = h.orbatTemplateForMission(&mission)
	}

	em := models.EventMission{
		EventID:   ev.ID,
		MissionID: missionID,
		StartTime: in.StartTime,
	}
	err = h.db.Transaction(func(tx *gorm.DB) error {
		if err := tx.Create(&em).Error; err != nil {
			return err
		}
		return materializeSlots(tx, em.ID, template)
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not attach mission"})
		return
	}
	c.JSON(http.StatusCreated, em)
}

// RemoveEventMission detaches a mission from an event, removing its ORBAT slots
// and registrations (admin only).
func (h *Handler) RemoveEventMission(c *gin.Context) {
	ev, ok := h.loadEvent(c)
	if !ok {
		return
	}
	emID, err := uuid.Parse(c.Param("emid"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid mission id"})
		return
	}
	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		var em models.EventMission
		if err := tx.First(&em, "id = ? AND event_id = ?", emID, ev.ID).Error; err != nil {
			return gorm.ErrRecordNotFound
		}
		tx.Where("event_mission_id = ?", emID).Delete(&models.EventRegistration{})
		tx.Where("event_mission_id = ?", emID).Delete(&models.OrbatSlot{})
		return tx.Delete(&em).Error
	})
	if txErr == gorm.ErrRecordNotFound {
		c.JSON(http.StatusNotFound, gin.H{"error": "mission not found in event"})
		return
	}
	if txErr != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not remove mission"})
		return
	}
	c.Status(http.StatusNoContent)
}

// --- Event lists ---

// eventListItem is an Upcoming Operations row. An event no longer maps to a
// single mission, so it carries an aggregate mission count and fill stats.
type eventListItem struct {
	models.Event
	MissionCount int   `json:"mission_count"`
	Registered   int64 `json:"registered"`
	Filled       int   `json:"filled"`
	TotalSlots   int   `json:"total_slots"`
	Percent      int   `json:"percent"`
}

// ListEvents returns operations for the Upcoming/Calendar views.
// Query: ?scope=upcoming|past|all
func (h *Handler) ListEvents(c *gin.Context) {
	limit, offset := parsePage(c)
	now := time.Now()

	q := h.db.Model(&models.Event{})
	switch c.DefaultQuery("scope", "upcoming") {
	case "past":
		q = q.Where("start_time <= ?", now).Order("start_time DESC")
	case "all":
		q = q.Order("start_time ASC")
	default:
		q = q.Where("start_time > ? OR status::text = ?", now, "live").Order("start_time ASC")
	}

	var total int64
	q.Count(&total)

	var events []models.Event
	if err := q.Limit(limit).Offset(offset).Find(&events).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not list events"})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"data":   h.decorateEvents(events),
		"total":  total,
		"limit":  limit,
		"offset": offset,
	})
}

// decorateEvents batch-loads per-event mission counts, registration counts, and
// ORBAT fill totals aggregated across each event's missions.
func (h *Handler) decorateEvents(events []models.Event) []eventListItem {
	eventIDs := make([]uuid.UUID, 0, len(events))
	for _, e := range events {
		eventIDs = append(eventIDs, e.ID)
	}

	// Map each event to its event_mission ids.
	type emRow struct {
		ID      uuid.UUID
		EventID uuid.UUID
	}
	missionsByEvent := map[uuid.UUID][]uuid.UUID{}
	emToEvent := map[uuid.UUID]uuid.UUID{}
	if len(eventIDs) > 0 {
		var ems []emRow
		h.db.Model(&models.EventMission{}).Select("id, event_id").
			Where("event_id IN ?", eventIDs).Scan(&ems)
		for _, r := range ems {
			missionsByEvent[r.EventID] = append(missionsByEvent[r.EventID], r.ID)
			emToEvent[r.ID] = r.EventID
		}
	}

	emIDs := make([]uuid.UUID, 0, len(emToEvent))
	for id := range emToEvent {
		emIDs = append(emIDs, id)
	}

	// Registration counts per event_mission, summed into the parent event.
	regByEvent := map[uuid.UUID]int64{}
	slotTotalByEvent := map[uuid.UUID]int{}
	slotFilledByEvent := map[uuid.UUID]int{}
	if len(emIDs) > 0 {
		type cntRow struct {
			EventMissionID uuid.UUID
			N              int64
		}
		var regRows []cntRow
		h.db.Model(&models.EventRegistration{}).
			Select("event_mission_id, count(*) as n").
			Where("event_mission_id IN ? AND state::text = ?", emIDs, "registered").
			Group("event_mission_id").Scan(&regRows)
		for _, r := range regRows {
			regByEvent[emToEvent[r.EventMissionID]] += r.N
		}

		type slotRow struct {
			EventMissionID uuid.UUID
			Total          int
			Filled         int
		}
		var slotRows []slotRow
		h.db.Model(&models.OrbatSlot{}).
			Select("event_mission_id, count(*) as total, count(assigned_to) as filled").
			Where("event_mission_id IN ?", emIDs).
			Group("event_mission_id").Scan(&slotRows)
		for _, r := range slotRows {
			slotTotalByEvent[emToEvent[r.EventMissionID]] += r.Total
			slotFilledByEvent[emToEvent[r.EventMissionID]] += r.Filled
		}
	}

	out := make([]eventListItem, 0, len(events))
	for _, e := range events {
		total := slotTotalByEvent[e.ID]
		filled := slotFilledByEvent[e.ID]
		percent := 0
		if total > 0 {
			percent = filled * 100 / total
		}
		out = append(out, eventListItem{
			Event:        e,
			MissionCount: len(missionsByEvent[e.ID]),
			Registered:   regByEvent[e.ID],
			Filled:       filled,
			TotalSlots:   total,
			Percent:      percent,
		})
	}
	return out
}

// --- Event Hub (nested missions) ---

type armoryFactionDTO struct {
	Faction string                 `json:"faction"`
	Items   []models.MissionArmory `json:"items"`
}

// eventMissionDossier is one mission's "dossier" inside the Event Hub.
type eventMissionDossier struct {
	EventMissionID string             `json:"event_mission_id"`
	MissionID      string             `json:"mission_id"`
	Title          string             `json:"title"`
	Terrain        string             `json:"terrain"`
	GameMode       string             `json:"game_mode"`
	Briefing       string             `json:"briefing,omitempty"`
	ThumbnailURL   string             `json:"thumbnail_url,omitempty"`
	StartTime      time.Time          `json:"start_time"`
	Factions       []string           `json:"factions"`
	Armory         []armoryFactionDTO `json:"armory_by_faction"`
	Filled         int                `json:"filled"`
	Total          int                `json:"total"`
	MyState        string             `json:"my_state,omitempty"`
	MySlotID       *string            `json:"my_slot_id,omitempty"`
}

type eventHubDTO struct {
	models.Event
	Missions []eventMissionDossier `json:"missions"`
}

// armoryByFaction groups a mission's armory rows by faction, preserving order.
func (h *Handler) armoryByFaction(missionID uuid.UUID) []armoryFactionDTO {
	var items []models.MissionArmory
	h.db.Where("mission_id = ?", missionID).Order("sort_order ASC").Find(&items)
	order := make([]string, 0)
	groups := map[string][]models.MissionArmory{}
	for _, it := range items {
		if _, ok := groups[it.Faction]; !ok {
			order = append(order, it.Faction)
		}
		groups[it.Faction] = append(groups[it.Faction], it)
	}
	out := make([]armoryFactionDTO, 0, len(order))
	for _, f := range order {
		out = append(out, armoryFactionDTO{Faction: f, Items: groups[f]})
	}
	return out
}

// GetEvent returns the Event Hub: event fields plus its nested mission dossiers
// (briefing, assets per faction, fill counts, and the caller's registration).
func (h *Handler) GetEvent(c *gin.Context) {
	ev, ok := h.loadEvent(c)
	if !ok {
		return
	}
	me := middleware.DiscordID(c)

	var ems []models.EventMission
	h.db.Where("event_id = ?", ev.ID).Order("start_time ASC").Find(&ems)

	missions := make([]eventMissionDossier, 0, len(ems))
	for _, em := range ems {
		var m models.Mission
		if err := h.db.First(&m, "id = ?", em.MissionID).Error; err != nil {
			continue
		}

		// Slot fill counts + distinct factions for this mission.
		var slots []models.OrbatSlot
		h.db.Where("event_mission_id = ?", em.ID).Find(&slots)
		filled := 0
		factionSeen := map[string]bool{}
		factions := make([]string, 0)
		for _, s := range slots {
			if s.AssignedTo != nil {
				filled++
			}
			if !factionSeen[s.Faction] {
				factionSeen[s.Faction] = true
				factions = append(factions, s.Faction)
			}
		}

		d := eventMissionDossier{
			EventMissionID: em.ID.String(),
			MissionID:      m.ID.String(),
			Title:          m.Title,
			Terrain:        string(m.Terrain),
			GameMode:       string(m.GameMode),
			Briefing:       m.Briefing,
			ThumbnailURL:   m.ThumbnailURL,
			StartTime:      em.StartTime,
			Factions:       factions,
			Armory:         h.armoryByFaction(m.ID),
			Filled:         filled,
			Total:          len(slots),
		}

		// Caller's registration state for this mission.
		var reg models.EventRegistration
		if err := h.db.First(&reg, "event_mission_id = ? AND discord_id = ?", em.ID, me).Error; err == nil {
			d.MyState = string(reg.State)
			if reg.SlotID != nil {
				s := reg.SlotID.String()
				d.MySlotID = &s
			}
		}
		missions = append(missions, d)
	}

	c.JSON(http.StatusOK, eventHubDTO{Event: *ev, Missions: missions})
}

// patchEventInput edits schedule/lock/status and event lore.
type patchEventInput struct {
	StartTime          *time.Time `json:"start_time"`
	MaxSlots           *int       `json:"max_slots"`
	NameOverride       *string    `json:"name_override"`
	Briefing           *string    `json:"briefing"`
	BannerImageURL     *string    `json:"banner_image_url"`
	RegistrationLocked *bool      `json:"registration_locked"`
	Status             *string    `json:"status"`
}

// UpdateEvent edits an event (admin only).
func (h *Handler) UpdateEvent(c *gin.Context) {
	ev, ok := h.loadEvent(c)
	if !ok {
		return
	}
	var in patchEventInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid body"})
		return
	}
	updates := map[string]any{}
	if in.StartTime != nil {
		updates["start_time"] = *in.StartTime
	}
	if in.MaxSlots != nil {
		updates["max_slots"] = *in.MaxSlots
	}
	if in.NameOverride != nil {
		updates["name_override"] = *in.NameOverride
	}
	if in.Briefing != nil {
		updates["briefing"] = *in.Briefing
	}
	if in.BannerImageURL != nil {
		updates["banner_image_url"] = *in.BannerImageURL
	}
	if in.RegistrationLocked != nil {
		updates["registration_locked"] = *in.RegistrationLocked
	}
	if in.Status != nil {
		st, ok := validEventStatus(*in.Status)
		if !ok {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid status"})
			return
		}
		updates["status"] = st
	}
	if len(updates) > 0 {
		if err := h.db.Model(ev).Updates(updates).Error; err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "could not update event"})
			return
		}
	}
	_ = h.db.First(ev, "id = ?", ev.ID).Error
	c.JSON(http.StatusOK, ev)
}

// DeleteEvent cancels/removes an event (admin only, soft delete).
func (h *Handler) DeleteEvent(c *gin.Context) {
	ev, ok := h.loadEvent(c)
	if !ok {
		return
	}
	if err := h.db.Delete(ev).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not delete event"})
		return
	}
	c.Status(http.StatusNoContent)
}

// --- ORBAT (per event_mission) ---

type orbatSlotDTO struct {
	ID           string  `json:"id"`
	Number       int     `json:"number"` // 1-based position within the squad
	Role         string  `json:"role"`
	Loadout      string  `json:"loadout,omitempty"`
	Tag          string  `json:"tag,omitempty"`
	SlotIndex    int     `json:"slot_index"`
	AssignedTo   *string `json:"assigned_to"`
	AssignedName string  `json:"assigned_name,omitempty"`
}

type orbatSquadDTO struct {
	Faction        string         `json:"faction"`
	Callsign       string         `json:"callsign,omitempty"`
	Squad          string         `json:"squad"`
	Filled         int            `json:"filled"`
	Total          int            `json:"total"`
	ReservedBy     string         `json:"reserved_by,omitempty"`
	ReservedByName string         `json:"reserved_by_name,omitempty"`
	Slots          []orbatSlotDTO `json:"slots"`
}

// GetOrbat returns a mission's ORBAT grouped by squad with filled/total counts,
// per-slot loadout/tag, and any leader squad reservations.
func (h *Handler) GetOrbat(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	var slots []models.OrbatSlot
	h.db.Where("event_mission_id = ?", em.ID).
		Order("faction ASC").Order("squad ASC").Order("slot_index ASC").
		Find(&slots)

	// Squad reservations for this mission, keyed by squad.
	var reservations []models.OrbatReservation
	h.db.Where("event_mission_id = ?", em.ID).Find(&reservations)
	reservedBy := map[string]string{}
	for _, r := range reservations {
		reservedBy[r.Squad] = r.ReservedBy
	}

	// Resolve display names for assignees + reservers in one lookup.
	idSet := map[string]struct{}{}
	for _, s := range slots {
		if s.AssignedTo != nil {
			idSet[*s.AssignedTo] = struct{}{}
		}
	}
	for _, who := range reservedBy {
		idSet[who] = struct{}{}
	}
	names := map[string]string{}
	if len(idSet) > 0 {
		var us []models.User
		h.db.Where("discord_id IN ?", keys(idSet)).Find(&us)
		for _, u := range us {
			names[u.DiscordID] = u.Username
		}
	}

	// Group by squad, preserving query order.
	order := make([]string, 0)
	groups := map[string]*orbatSquadDTO{}
	for _, s := range slots {
		g, exists := groups[s.Squad]
		if !exists {
			g = &orbatSquadDTO{Faction: s.Faction, Callsign: s.Callsign, Squad: s.Squad}
			if who, ok := reservedBy[s.Squad]; ok {
				g.ReservedBy = who
				g.ReservedByName = names[who]
			}
			groups[s.Squad] = g
			order = append(order, s.Squad)
		}
		dto := orbatSlotDTO{
			ID:         s.ID.String(),
			Number:     s.SlotIndex + 1,
			Role:       s.Role,
			Loadout:    s.Loadout,
			Tag:        s.Tag,
			SlotIndex:  s.SlotIndex,
			AssignedTo: s.AssignedTo,
		}
		if s.AssignedTo != nil {
			dto.AssignedName = names[*s.AssignedTo]
			g.Filled++
		}
		g.Total++
		g.Slots = append(g.Slots, dto)
	}

	out := make([]orbatSquadDTO, 0, len(order))
	for _, sq := range order {
		out = append(out, *groups[sq])
	}
	c.JSON(http.StatusOK, gin.H{"data": out})
}

// --- Registration (per event_mission) ---

type registerBody struct {
	SlotID string `json:"slot_id"`
}

// RegisterForEventMission signs the caller up for a specific mission within an
// event, claiming a slot if provided, otherwise granting a confirmed spot or a
// waitlist place based on the mission's slot capacity.
func (h *Handler) RegisterForEventMission(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	var ev models.Event
	if err := h.db.First(&ev, "id = ?", em.EventID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "event not found"})
		return
	}
	me := middleware.DiscordID(c)
	if !canRegisterStatus(ev.Status) {
		c.JSON(http.StatusConflict, gin.H{"error": "registration is closed for this operation"})
		return
	}
	if ev.RegistrationLocked && middleware.Role(c) != "admin" {
		c.JSON(http.StatusForbidden, gin.H{"error": "registration is locked; an admin must assign you"})
		return
	}

	var body registerBody
	_ = c.ShouldBindJSON(&body)

	var result models.EventRegistration
	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		var capacity int64
		tx.Model(&models.OrbatSlot{}).Where("event_mission_id = ?", em.ID).Count(&capacity)
		var registered int64
		tx.Model(&models.EventRegistration{}).
			Where("event_mission_id = ? AND state::text = ? AND discord_id <> ?", em.ID, "registered", me).
			Count(&registered)

		state := models.RegRegistered
		var slotID *uuid.UUID

		if body.SlotID != "" {
			sid, err := uuid.Parse(body.SlotID)
			if err != nil {
				return errBadSlot
			}
			var slot models.OrbatSlot
			if err := tx.First(&slot, "id = ? AND event_mission_id = ?", sid, em.ID).Error; err != nil {
				return errSlotNotFound
			}
			if slot.AssignedTo != nil && *slot.AssignedTo != me {
				return errSlotTaken
			}
			// A reserved squad is held for its leader: only the reserver (or an
			// admin) may self-claim slots inside it.
			if middleware.Role(c) != "admin" {
				var res models.OrbatReservation
				if err := tx.First(&res, "event_mission_id = ? AND squad = ?", em.ID, slot.Squad).Error; err == nil {
					if res.ReservedBy != me {
						return errSquadReserved
					}
				}
			}
			now := time.Now()
			if err := tx.Model(&models.OrbatSlot{}).Where("id = ?", sid).
				Updates(map[string]any{"assigned_to": me, "assigned_at": now}).Error; err != nil {
				return err
			}
			slotID = &sid
		} else if capacity > 0 && registered >= capacity {
			state = models.RegWaitlisted
		}

		reg := models.EventRegistration{
			EventMissionID: em.ID,
			DiscordID:      me,
			SlotID:         slotID,
			State:          state,
		}
		if err := tx.Clauses(clause.OnConflict{
			Columns:   []clause.Column{{Name: "event_mission_id"}, {Name: "discord_id"}},
			DoUpdates: clause.AssignmentColumns([]string{"slot_id", "state"}),
		}).Create(&reg).Error; err != nil {
			return err
		}
		result = reg
		return nil
	})

	switch txErr {
	case nil:
		c.JSON(http.StatusOK, gin.H{"state": result.State, "slot_id": result.SlotID})
	case errBadSlot, errSlotNotFound:
		c.JSON(http.StatusNotFound, gin.H{"error": "slot not found"})
	case errSlotTaken:
		c.JSON(http.StatusConflict, gin.H{"error": "slot already taken"})
	case errSquadReserved:
		c.JSON(http.StatusConflict, gin.H{"error": "squad is reserved by a leader"})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not register"})
	}
}

// WithdrawFromEventMission removes the caller's registration for a mission and
// promotes the oldest waitlisted member if a confirmed spot was freed.
func (h *Handler) WithdrawFromEventMission(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	me := middleware.DiscordID(c)

	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		var reg models.EventRegistration
		if err := tx.First(&reg, "event_mission_id = ? AND discord_id = ?", em.ID, me).Error; err != nil {
			return gorm.ErrRecordNotFound
		}
		// Free any claimed slot.
		if reg.SlotID != nil {
			tx.Model(&models.OrbatSlot{}).Where("id = ?", *reg.SlotID).
				Updates(map[string]any{"assigned_to": nil, "assigned_at": nil})
		}
		wasRegistered := reg.State == models.RegRegistered
		if err := tx.Delete(&reg).Error; err != nil {
			return err
		}
		// Promote the oldest waitlisted member into the freed spot.
		if wasRegistered {
			var next models.EventRegistration
			if err := tx.Where("event_mission_id = ? AND state::text = ?", em.ID, "waitlisted").
				Order("registered_at ASC").First(&next).Error; err == nil {
				tx.Model(&models.EventRegistration{}).Where("id = ?", next.ID).
					Update("state", models.RegRegistered)
			}
		}
		return nil
	})
	if txErr == gorm.ErrRecordNotFound {
		c.JSON(http.StatusNotFound, gin.H{"error": "not registered"})
		return
	}
	if txErr != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not withdraw"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"withdrawn": true})
}

// --- Slot assignment (admin, per event_mission) ---

type assignSlotInput struct {
	DiscordID string `json:"discord_id" binding:"required"`
}

// canManageSquad reports whether the caller may fill/assign a squad's slots: an
// admin always can; a leader can only manage a squad they have reserved.
func (h *Handler) canManageSquad(c *gin.Context, emID uuid.UUID, squad string) bool {
	if middleware.Role(c) == "admin" {
		return true
	}
	var res models.OrbatReservation
	if err := h.db.First(&res, "event_mission_id = ? AND squad = ?", emID, squad).Error; err != nil {
		return false
	}
	return res.ReservedBy == middleware.DiscordID(c)
}

// AssignSlot assigns or reassigns a user to an ORBAT slot and ensures they have a
// confirmed registration for that mission. Allowed for an admin, or the leader who
// reserved the slot's squad.
func (h *Handler) AssignSlot(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	slotID, err := uuid.Parse(c.Param("slotId"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid slot id"})
		return
	}
	var in assignSlotInput
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "discord_id required"})
		return
	}
	var target models.User
	if err := h.db.First(&target, "discord_id = ?", in.DiscordID).Error; err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "user not found"})
		return
	}
	var slot models.OrbatSlot
	if err := h.db.First(&slot, "id = ? AND event_mission_id = ?", slotID, em.ID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "slot not found"})
		return
	}
	if !h.canManageSquad(c, em.ID, slot.Squad) {
		c.JSON(http.StatusForbidden, gin.H{"error": "reserve this squad to assign its slots"})
		return
	}

	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		now := time.Now()
		if err := tx.Model(&models.OrbatSlot{}).Where("id = ?", slotID).
			Updates(map[string]any{"assigned_to": in.DiscordID, "assigned_at": now}).Error; err != nil {
			return err
		}
		reg := models.EventRegistration{
			EventMissionID: em.ID,
			DiscordID:      in.DiscordID,
			SlotID:         &slotID,
			State:          models.RegRegistered,
		}
		return tx.Clauses(clause.OnConflict{
			Columns:   []clause.Column{{Name: "event_mission_id"}, {Name: "discord_id"}},
			DoUpdates: clause.AssignmentColumns([]string{"slot_id", "state"}),
		}).Create(&reg).Error
	})
	if txErr != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not assign slot"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"assigned_to": in.DiscordID})
}

// ClearSlot unassigns an ORBAT slot. Allowed for an admin, or the leader who
// reserved the slot's squad.
func (h *Handler) ClearSlot(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	slotID, err := uuid.Parse(c.Param("slotId"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid slot id"})
		return
	}
	var slot models.OrbatSlot
	if err := h.db.First(&slot, "id = ? AND event_mission_id = ?", slotID, em.ID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "slot not found"})
		return
	}
	if !h.canManageSquad(c, em.ID, slot.Squad) {
		c.JSON(http.StatusForbidden, gin.H{"error": "reserve this squad to manage its slots"})
		return
	}
	h.db.Transaction(func(tx *gorm.DB) error {
		tx.Model(&models.OrbatSlot{}).Where("id = ? AND event_mission_id = ?", slotID, em.ID).
			Updates(map[string]any{"assigned_to": nil, "assigned_at": nil})
		tx.Model(&models.EventRegistration{}).Where("event_mission_id = ? AND slot_id = ?", em.ID, slotID).
			Update("slot_id", nil)
		return nil
	})
	c.JSON(http.StatusOK, gin.H{"cleared": true})
}

// --- Squad reservation (leader: hold an entire squad in one click) ---

type squadBody struct {
	Squad string `json:"squad" binding:"required"`
}

// ReserveSquad places a one-click hold on a whole squad for the calling leader.
// While held, only the reserver (or an admin) may fill its slots.
func (h *Handler) ReserveSquad(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	var in squadBody
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "squad is required"})
		return
	}
	me := middleware.DiscordID(c)

	// The squad must exist in this mission's ORBAT.
	var n int64
	h.db.Model(&models.OrbatSlot{}).Where("event_mission_id = ? AND squad = ?", em.ID, in.Squad).Count(&n)
	if n == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "squad not found in this ORBAT"})
		return
	}

	// Reject if already reserved by someone else.
	var existing models.OrbatReservation
	if err := h.db.First(&existing, "event_mission_id = ? AND squad = ?", em.ID, in.Squad).Error; err == nil {
		if existing.ReservedBy != me {
			c.JSON(http.StatusConflict, gin.H{"error": "squad is already reserved"})
			return
		}
		c.JSON(http.StatusOK, existing) // idempotent: already yours
		return
	}

	res := models.OrbatReservation{EventMissionID: em.ID, Squad: in.Squad, ReservedBy: me}
	if err := h.db.Create(&res).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not reserve squad"})
		return
	}
	c.JSON(http.StatusCreated, res)
}

// ReleaseSquad lifts a squad hold. Only the reserver or an admin may release.
func (h *Handler) ReleaseSquad(c *gin.Context) {
	em, ok := h.loadEventMission(c)
	if !ok {
		return
	}
	var in squadBody
	if err := c.ShouldBindJSON(&in); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "squad is required"})
		return
	}
	var res models.OrbatReservation
	if err := h.db.First(&res, "event_mission_id = ? AND squad = ?", em.ID, in.Squad).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "squad is not reserved"})
		return
	}
	if res.ReservedBy != middleware.DiscordID(c) && middleware.Role(c) != "admin" {
		c.JSON(http.StatusForbidden, gin.H{"error": "only the reserver or an admin can release this squad"})
		return
	}
	if err := h.db.Delete(&res).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not release squad"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"released": true})
}

// --- Member directory (leader: pick assignees for a reserved squad) ---

type memberDTO struct {
	DiscordID string `json:"discord_id"`
	Username  string `json:"username"`
	AvatarURL string `json:"avatar_url,omitempty"`
}

// SearchMembers returns a slim member list for leaders filling a reserved squad.
// Query: ?q= matches username/handle (case-insensitive). Excludes banned users.
func (h *Handler) SearchMembers(c *gin.Context) {
	q := c.Query("q")
	db := h.db.Model(&models.User{}).Where("is_banned = ?", false)
	if q != "" {
		like := "%" + q + "%"
		db = db.Where("username ILIKE ? OR discord_handle ILIKE ?", like, like)
	}
	var users []models.User
	db.Order("username ASC").Limit(20).Find(&users)
	out := make([]memberDTO, 0, len(users))
	for _, u := range users {
		out = append(out, memberDTO{DiscordID: u.DiscordID, Username: u.Username, AvatarURL: u.AvatarURL})
	}
	c.JSON(http.StatusOK, gin.H{"data": out})
}

// --- helpers ---

func (h *Handler) loadEvent(c *gin.Context) (*models.Event, bool) {
	id, err := uuid.Parse(c.Param("id"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return nil, false
	}
	var ev models.Event
	if err := h.db.First(&ev, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "event not found"})
		return nil, false
	}
	return &ev, true
}

func (h *Handler) loadEventMission(c *gin.Context) (*models.EventMission, bool) {
	id, err := uuid.Parse(c.Param("emid"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid id"})
		return nil, false
	}
	var em models.EventMission
	if err := h.db.First(&em, "id = ?", id).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "mission not found"})
		return nil, false
	}
	return &em, true
}
