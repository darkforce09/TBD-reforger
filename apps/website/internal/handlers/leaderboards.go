package handlers

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// leaderboardOrder whitelists category -> ORDER BY clause to avoid injection.
var leaderboardOrder = map[string]string{
	"kd":           "lt.kd_ratio DESC NULLS LAST",
	"command_win":  "lt.command_win_rate DESC NULLS LAST",
	"missions":     "lt.missions_played DESC",
	"longest_kill": "lt.longest_kill_m DESC",
	"team_kills":   "lt.team_kills DESC", // Wall of Shame
}

// leaderboardRow is one ranked entry joined with the user's display info.
type leaderboardRow struct {
	DiscordID         string  `json:"discord_id" gorm:"column:discord_id"`
	Username          string  `json:"username" gorm:"column:username"`
	AvatarURL         string  `json:"avatar_url" gorm:"column:avatar_url"`
	Kills             int     `json:"kills" gorm:"column:kills"`
	Deaths            int     `json:"deaths" gorm:"column:deaths"`
	KDRatio           float64 `json:"kd_ratio" gorm:"column:kd_ratio"`
	TeamKills         int     `json:"team_kills" gorm:"column:team_kills"`
	LongestKillM      int     `json:"longest_kill_m" gorm:"column:longest_kill_m"`
	VehiclesDestroyed int     `json:"vehicles_destroyed" gorm:"column:vehicles_destroyed"`
	MissionsPlayed    int     `json:"missions_played" gorm:"column:missions_played"`
	CommandWins       int     `json:"command_wins" gorm:"column:command_wins"`
	CommandWinRate    float64 `json:"command_win_rate" gorm:"column:command_win_rate"`
	Rank              int     `json:"rank" gorm:"-"`
}

// GetLeaderboards returns a ranked board for the selected category, searchable
// by player name. The frontend splits the first three into the podium.
// Query: ?category=kd|command_win|missions|longest_kill|team_kills &q= &limit=
func (h *Handler) GetLeaderboards(c *gin.Context) {
	category := c.DefaultQuery("category", "kd")
	order, ok := leaderboardOrder[category]
	if !ok {
		c.JSON(http.StatusBadRequest, gin.H{"error": "unknown category"})
		return
	}
	limit, offset := parsePage(c)
	if limit > 50 {
		limit = 50
	}

	q := h.db.Table("leaderboard_totals lt").
		Select("lt.*, u.username, u.avatar_url").
		Joins("JOIN users u ON u.discord_id = lt.discord_id AND u.deleted_at IS NULL")
	if search := strings.TrimSpace(c.Query("q")); search != "" {
		q = q.Where("u.username ILIKE ?", "%"+search+"%")
	}

	rows := []leaderboardRow{}
	if err := q.Order(order).Limit(limit).Offset(offset).Scan(&rows).Error; err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load leaderboard"})
		return
	}
	for i := range rows {
		rows[i].Rank = offset + i + 1
	}

	c.JSON(http.StatusOK, gin.H{"category": category, "data": rows})
}

// GetUserStats returns one player's aggregate stat card.
func (h *Handler) GetUserStats(c *gin.Context) {
	discordID := c.Param("discordId")

	var user models.User
	if err := h.db.First(&user, "discord_id = ?", discordID).Error; err != nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
		return
	}

	var row leaderboardRow
	err := h.db.Table("leaderboard_totals lt").
		Select("lt.*, u.username, u.avatar_url").
		Joins("JOIN users u ON u.discord_id = lt.discord_id").
		Where("lt.discord_id = ?", discordID).
		Scan(&row).Error
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "could not load stats"})
		return
	}
	// No telemetry yet: return zeroed stats with the user's identity.
	if row.DiscordID == "" {
		row.DiscordID = user.DiscordID
		row.Username = user.Username
		row.AvatarURL = user.AvatarURL
	}

	c.JSON(http.StatusOK, gin.H{
		"stats":            row,
		"total_operations": user.TotalDeployments,
		"attendance_rate":  user.AttendanceRate,
	})
}

// StreamServerStatus is an SSE endpoint that pushes live server-status updates.
func (h *Handler) StreamServerStatus(c *gin.Context) {
	serverID := c.Param("id")

	c.Writer.Header().Set("Content-Type", "text/event-stream")
	c.Writer.Header().Set("Cache-Control", "no-cache")
	c.Writer.Header().Set("Connection", "keep-alive")
	c.Writer.Header().Set("X-Accel-Buffering", "no")

	flusher, ok := c.Writer.(interface{ Flush() })
	if !ok {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "streaming unsupported"})
		return
	}

	ch, cancel := h.hub.Subscribe("server:" + serverID)
	defer cancel()

	// Send the current snapshot immediately so the client renders without delay.
	var current models.ServerStatus
	if err := h.db.First(&current, "server_id = ?", serverID).Error; err == nil {
		if b, err := json.Marshal(current); err == nil {
			writeSSE(c.Writer, b)
			flusher.Flush()
		}
	}

	ctx := c.Request.Context()
	for {
		select {
		case <-ctx.Done():
			return
		case msg, open := <-ch:
			if !open {
				return
			}
			writeSSE(c.Writer, msg)
			flusher.Flush()
		}
	}
}

// writeSSE writes a single Server-Sent Event data frame.
func writeSSE(w interface{ Write([]byte) (int, error) }, data []byte) {
	_, _ = w.Write([]byte("data: ")) //nolint:errcheck // best-effort: SSE frame; a disconnected client is reaped by the stream loop.
	_, _ = w.Write(data)             //nolint:errcheck // best-effort: SSE frame; a disconnected client is reaped by the stream loop.
	_, _ = w.Write([]byte("\n\n"))   //nolint:errcheck // best-effort: SSE frame; a disconnected client is reaped by the stream loop.
}
