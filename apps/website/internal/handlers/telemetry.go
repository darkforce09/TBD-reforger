package handlers

import (
	"encoding/json"
	"net/http"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/models"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

const lowFPSThreshold = 20.0

// serverStatusInput is the live telemetry pushed by the game server.
type serverStatusInput struct {
	ServerID       string  `json:"server_id" binding:"required"`
	IsOnline       bool    `json:"is_online"`
	PlayerCount    int     `json:"player_count"`
	MaxPlayers     int     `json:"max_players"`
	ServerFPS      float64 `json:"server_fps"`
	UptimeSeconds  int64   `json:"uptime_seconds"`
	CurrentMatchID *string `json:"current_match_id"`
	IngameTime     string  `json:"ingame_time"`
	IngameWeather  string  `json:"ingame_weather"`
}

// IngestServerStatus upserts the live status row, appends a history sample,
// emits a WARN audit when FPS drops below the threshold, and fans the update
// out to SSE subscribers. Service-token authenticated.
//
// @route POST /api/v1/ingest/server-status
func (h *Handler) IngestServerStatus(c *gin.Context) {
	var in serverStatusInput
	if err := c.ShouldBindJSON(&in); err != nil {
		logHandlerErr(c, "IngestServerStatus", http.StatusBadRequest, "server_id required")
		c.JSON(http.StatusBadRequest, gin.H{"error": "server_id required"})
		return
	}
	serverID, err := uuid.Parse(in.ServerID)
	if err != nil {
		logHandlerErr(c, "IngestServerStatus", http.StatusBadRequest, "invalid server_id")
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid server_id"})
		return
	}

	// Edge-trigger the low-FPS warning: only when crossing below the threshold.
	var prev models.ServerStatus
	prevHealthy := true
	if err := h.db.First(&prev, "server_id = ?", serverID).Error; err == nil {
		prevHealthy = prev.ServerFPS >= lowFPSThreshold
	}

	var matchID *uuid.UUID
	if in.CurrentMatchID != nil && *in.CurrentMatchID != "" {
		if mid, err := uuid.Parse(*in.CurrentMatchID); err == nil {
			matchID = &mid
		}
	}

	status := models.ServerStatus{
		ServerID:       serverID,
		IsOnline:       in.IsOnline,
		PlayerCount:    in.PlayerCount,
		MaxPlayers:     in.MaxPlayers,
		ServerFPS:      in.ServerFPS,
		UptimeSeconds:  in.UptimeSeconds,
		CurrentMatchID: matchID,
		IngameTime:     in.IngameTime,
		IngameWeather:  in.IngameWeather,
		UpdatedAt:      time.Now(),
	}
	if err := h.db.Clauses(clause.OnConflict{
		Columns: []clause.Column{{Name: "server_id"}},
		DoUpdates: clause.AssignmentColumns([]string{
			"is_online", "player_count", "max_players", "server_fps",
			"uptime_seconds", "current_match_id", "ingame_time", "ingame_weather", "updated_at",
		}),
	}).Create(&status).Error; err != nil {
		logHandlerErr(c, "IngestServerStatus", http.StatusInternalServerError, "could not store status")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not store status"})
		return
	}

	// Append a time-series sample.
	h.db.Create(&models.ServerStatusHistory{
		ServerID:    serverID,
		PlayerCount: in.PlayerCount,
		ServerFPS:   in.ServerFPS,
	})

	if prevHealthy && in.ServerFPS < lowFPSThreshold && in.IsOnline {
		//nolint:errcheck // best-effort: audit log is non-blocking; a failed write must not fail the request.
		_ = services.WriteAudit(h.db, models.SeverityWarn, nil, "system",
			"server.low_fps",
			"Active server FPS dropped below 20 (now "+formatFPS(in.ServerFPS)+")",
			"server", serverID.String())
	}

	// Fan out to SSE subscribers.
	if payload, err := json.Marshal(status); err == nil {
		h.hub.Publish("server:"+serverID.String(), payload)
	}

	c.JSON(http.StatusOK, gin.H{"ok": true})
}

// --- Match results ---

type matchInput struct {
	SourceMatchID  *string    `json:"source_match_id"`
	EventID        *string    `json:"event_id"`
	MissionID      *string    `json:"mission_id"`
	Terrain        string     `json:"terrain"`
	StartedAt      *time.Time `json:"started_at"`
	EndedAt        *time.Time `json:"ended_at"`
	Outcome        string     `json:"outcome"`
	WinningFaction string     `json:"winning_faction"`
	AARReplayURL   string     `json:"aar_replay_url"`
}

type playerStatInput struct {
	ArmaID            string `json:"arma_id" binding:"required"`
	RolePlayed        string `json:"role_played"`
	Kills             int    `json:"kills"`
	Deaths            int    `json:"deaths"`
	TeamKills         int    `json:"team_kills"`
	LongestKillM      int    `json:"longest_kill_m"`
	VehiclesDestroyed int    `json:"vehicles_destroyed"`
	IsCommand         bool   `json:"is_command"`
	CommandWin        *bool  `json:"command_win"`
	SourceEventID     string `json:"source_event_id" binding:"required"`
}

type matchResultsInput struct {
	Match   matchInput        `json:"match"`
	Players []playerStatInput `json:"players" binding:"required"`
}

// IngestMatchResults records a completed match and its per-player stats. The
// batch is idempotent: re-posting with the same source_match_id reuses the match
// and upserts stats on (match_id, arma_id, source_event_id). Service-token auth.
//
// @route POST /api/v1/ingest/match-results
func (h *Handler) IngestMatchResults(c *gin.Context) {
	var in matchResultsInput
	if err := c.ShouldBindJSON(&in); err != nil {
		logHandlerErr(c, "IngestMatchResults", http.StatusBadRequest, "match and players are required")
		c.JSON(http.StatusBadRequest, gin.H{"error": "match and players are required"})
		return
	}

	outcome := models.OutcomePending
	switch models.MissionOutcome(in.Match.Outcome) {
	case models.OutcomeSuccess, models.OutcomeFailure, models.OutcomeAborted, models.OutcomePending:
		if in.Match.Outcome != "" {
			outcome = models.MissionOutcome(in.Match.Outcome)
		}
	default:
		logHandlerErr(c, "IngestMatchResults", http.StatusBadRequest, "invalid outcome")
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid outcome"})
		return
	}

	resolvedPlayers := map[string]struct{}{}
	var matchID uuid.UUID

	txErr := h.db.Transaction(func(tx *gorm.DB) error {
		match, err := upsertMatch(tx, in.Match, outcome)
		if err != nil {
			return err
		}
		matchID = match.ID

		for _, p := range in.Players {
			stat := models.MatchPlayerStat{
				MatchID:           match.ID,
				ArmaID:            p.ArmaID,
				RolePlayed:        p.RolePlayed,
				Kills:             p.Kills,
				Deaths:            p.Deaths,
				TeamKills:         p.TeamKills,
				LongestKillM:      p.LongestKillM,
				VehiclesDestroyed: p.VehiclesDestroyed,
				IsCommand:         p.IsCommand,
				CommandWin:        p.CommandWin,
				SourceEventID:     p.SourceEventID,
			}
			// Resolve the Discord identity from the linked Arma ID.
			var u models.User
			if err := tx.Select("discord_id").First(&u, "arma_id = ?", p.ArmaID).Error; err == nil {
				stat.DiscordID = &u.DiscordID
				resolvedPlayers[u.DiscordID] = struct{}{}
			}

			if err := tx.Clauses(clause.OnConflict{
				Columns: []clause.Column{{Name: "match_id"}, {Name: "arma_id"}, {Name: "source_event_id"}},
				DoUpdates: clause.AssignmentColumns([]string{
					"discord_id", "role_played", "kills", "deaths", "team_kills",
					"longest_kill_m", "vehicles_destroyed", "is_command", "command_win",
				}),
			}).Create(&stat).Error; err != nil {
				return err
			}
		}

		// Mark attendance for scheduled operations. event_registrations keys on
		// event_mission_id (event_id was dropped in 02_campaign_refactor.sql), so
		// resolve via the event's missions; return the error so the tx rolls back.
		if match.EventID != nil && len(resolvedPlayers) > 0 {
			ids := keys(resolvedPlayers)
			if err := tx.Model(&models.EventRegistration{}).
				Where("event_mission_id IN (SELECT id FROM event_missions WHERE event_id = ?) AND discord_id IN ?", *match.EventID, ids).
				Update("state", models.RegAttended).Error; err != nil {
				return err
			}
		}
		return nil
	})
	if txErr != nil {
		logHandlerErr(c, "IngestMatchResults", http.StatusInternalServerError, "could not ingest match")
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not ingest match"})
		return
	}

	// Recompute denormalized user stats and refresh the leaderboard view.
	for discordID := range resolvedPlayers {
		h.recomputeUserStats(discordID)
	}
	if err := services.RefreshLeaderboard(h.db); err != nil {
		// Refresh is best-effort: the match ingest already committed, so the handler
		// still returns 200 — but the failure must be surfaced server-side (LOG-3).
		logHandlerErr(c, "IngestMatchResults", http.StatusOK, "leaderboard refresh failed")
		//nolint:errcheck // best-effort: audit log is non-blocking; a failed write must not fail the request.
		_ = services.WriteAudit(h.db, models.SeverityWarn, nil, "system",
			"leaderboard.refresh_failed", "Leaderboard refresh failed after match ingest", "match", matchID.String())
	}

	c.JSON(http.StatusOK, gin.H{"match_id": matchID, "players": len(in.Players)})
}

// upsertMatch finds an existing match by source_match_id or creates a new one,
// updating mutable result fields.
func upsertMatch(tx *gorm.DB, in matchInput, outcome models.MissionOutcome) (*models.Match, error) {
	started := time.Now()
	if in.StartedAt != nil {
		started = *in.StartedAt
	}
	match := models.Match{
		SourceMatchID:  in.SourceMatchID,
		StartedAt:      started,
		EndedAt:        in.EndedAt,
		Outcome:        outcome,
		WinningFaction: in.WinningFaction,
		AARReplayURL:   in.AARReplayURL,
	}
	if eid := parseUUIDPtr(in.EventID); eid != nil {
		match.EventID = eid
	}
	if mid := parseUUIDPtr(in.MissionID); mid != nil {
		match.MissionID = mid
	}
	if t, ok := validTerrain(in.Terrain); ok {
		match.Terrain = &t
	}

	if in.SourceMatchID != nil && *in.SourceMatchID != "" {
		var existing models.Match
		if err := tx.First(&existing, "source_match_id = ?", *in.SourceMatchID).Error; err == nil {
			// Update mutable result fields on the existing match.
			tx.Model(&existing).Updates(map[string]any{
				"ended_at": in.EndedAt, "outcome": outcome,
				"winning_faction": in.WinningFaction, "aar_replay_url": in.AARReplayURL,
			})
			return &existing, nil
		}
	}
	if err := tx.Create(&match).Error; err != nil {
		return nil, err
	}
	return &match, nil
}

// recomputeUserStats refreshes a user's denormalized deployment + attendance
// metrics from telemetry and registrations.
func (h *Handler) recomputeUserStats(discordID string) {
	var deployments int64
	h.db.Model(&models.MatchPlayerStat{}).
		Where("discord_id = ?", discordID).
		Distinct("match_id").Count(&deployments)

	var attended, pastRegistered int64
	h.db.Model(&models.EventRegistration{}).
		Where("discord_id = ? AND state::text = ?", discordID, "attended").Count(&attended)
	h.db.Model(&models.EventRegistration{}).
		Joins("JOIN event_missions ON event_missions.id = event_registrations.event_mission_id").
		Where("event_registrations.discord_id = ? AND event_missions.start_time <= ?", discordID, time.Now()).
		Count(&pastRegistered)

	rate := 0.0
	if pastRegistered > 0 {
		rate = float64(attended) / float64(pastRegistered) * 100
	}
	h.db.Model(&models.User{}).Where("discord_id = ?", discordID).
		Updates(map[string]any{"total_deployments": deployments, "attendance_rate": rate})
}

// --- small helpers ---

func parseUUIDPtr(s *string) *uuid.UUID {
	if s == nil || *s == "" {
		return nil
	}
	id, err := uuid.Parse(*s)
	if err != nil {
		return nil
	}
	return &id
}

func keys(m map[string]struct{}) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	return out
}

func formatFPS(f float64) string {
	return strconv.FormatFloat(f, 'f', 1, 64)
}
