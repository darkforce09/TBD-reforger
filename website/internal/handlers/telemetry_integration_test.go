package handlers

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/db"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestTelemetryIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	u1 := fmt.Sprintf("itest-tel1-%d", time.Now().UnixNano())
	u2 := fmt.Sprintf("itest-tel2-%d", time.Now().UnixNano())
	arma1 := fmt.Sprintf("steam-%s", u1)
	arma2 := fmt.Sprintf("steam-%s", u2)
	srcMatch := fmt.Sprintf("m-%d", time.Now().UnixNano())

	server := models.Server{Name: "TBD Main", IP: "127.0.0.1", Port: 2001}

	t.Cleanup(func() {
		gdb.Where("arma_id IN ?", []string{arma1, arma2}).Delete(&models.MatchPlayerStat{})
		gdb.Where("source_match_id = ?", srcMatch).Delete(&models.Match{})
		gdb.Where("server_id = ?", server.ID).Delete(&models.ServerStatusHistory{})
		gdb.Where("server_id = ?", server.ID).Delete(&models.ServerStatus{})
		gdb.Where("target_type = ? AND target_id = ?", "server", server.ID.String()).Delete(&models.AuditLog{})
		gdb.Where("id = ?", server.ID).Delete(&models.Server{})
		gdb.Unscoped().Where("discord_id IN ?", []string{u1, u2}).Delete(&models.User{})
		_ = db.RefreshLeaderboard(gdb) // keep MV clean for other runs
	})

	gdb.Create(&models.User{DiscordID: u1, Username: "Ghost Reaper", Role: models.RoleEnlisted, ArmaID: &arma1})
	gdb.Create(&models.User{DiscordID: u2, Username: "Shadow Zero", Role: models.RoleEnlisted, ArmaID: &arma2})
	gdb.Create(&server)
	sid := server.ID.String()

	userTok, _, _ := h.JWT().IssueAccess(u1, "enlisted", false)

	// ===== Server status ingest =====
	// Wrong service token rejected.
	if w := do(r, "POST", "/api/v1/ingest/server-status", reqOpt{service: "nope", body: fmt.Sprintf(`{"server_id":%q,"is_online":true,"player_count":55,"server_fps":44}`, sid)}); w.Code != http.StatusUnauthorized {
		t.Fatalf("ingest bad token = %d, want 401", w.Code)
	}
	// Healthy push.
	if w := do(r, "POST", "/api/v1/ingest/server-status", reqOpt{service: "svc-token", body: fmt.Sprintf(`{"server_id":%q,"is_online":true,"player_count":55,"max_players":64,"server_fps":44,"uptime_seconds":8073,"ingame_time":"14:30 Local","ingame_weather":"Overcast"}`, sid)}); w.Code != http.StatusOK {
		t.Fatalf("ingest status = %d, body=%s", w.Code, w.Body.String())
	}
	w := do(r, "GET", "/api/v1/servers/"+sid+"/status", reqOpt{bearer: userTok})
	var intel struct {
		Status *models.ServerStatus `json:"status"`
	}
	mustJSON(t, w, &intel)
	if intel.Status == nil || intel.Status.PlayerCount != 55 || intel.Status.ServerFPS != 44 {
		t.Fatalf("server status not reflected: %+v", intel.Status)
	}

	// Low-FPS push -> WARN audit row (edge trigger from healthy->unhealthy).
	if w := do(r, "POST", "/api/v1/ingest/server-status", reqOpt{service: "svc-token", body: fmt.Sprintf(`{"server_id":%q,"is_online":true,"player_count":50,"server_fps":15}`, sid)}); w.Code != http.StatusOK {
		t.Fatalf("low fps ingest = %d", w.Code)
	}
	var warnCount int64
	gdb.Model(&models.AuditLog{}).
		Where("action = ? AND target_id = ? AND severity::text = ?", "server.low_fps", sid, "warn").
		Count(&warnCount)
	if warnCount < 1 {
		t.Fatal("expected a WARN audit row for low FPS")
	}

	// ===== SSE live feed (real server, read the snapshot frame) =====
	ts := httptest.NewServer(r)
	defer ts.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	req, _ := http.NewRequestWithContext(ctx, "GET", ts.URL+"/api/v1/servers/"+sid+"/status/stream", nil)
	req.Header.Set("Authorization", "Bearer "+userTok)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("sse connect: %v", err)
	}
	defer resp.Body.Close()
	if ct := resp.Header.Get("Content-Type"); !strings.HasPrefix(ct, "text/event-stream") {
		t.Fatalf("sse content-type = %q", ct)
	}
	line, err := bufio.NewReader(resp.Body).ReadString('\n')
	if err != nil {
		t.Fatalf("sse read: %v", err)
	}
	if !strings.HasPrefix(line, "data: ") || !strings.Contains(line, `"player_count":50`) {
		t.Fatalf("unexpected sse frame: %q", line)
	}

	// ===== Match results ingest + leaderboard =====
	started := time.Now().Add(-time.Hour).UTC().Format(time.RFC3339)
	matchBody := fmt.Sprintf(`{"match":{"source_match_id":%q,"started_at":%q,"outcome":"success","aar_replay_url":"http://aar/1"},
		"players":[
			{"arma_id":%q,"role_played":"Alpha 1-1 (SL)","kills":10,"deaths":2,"longest_kill_m":540,"source_event_id":"se-u1"},
			{"arma_id":%q,"role_played":"Alpha 1-2 (Rifleman)","kills":5,"deaths":5,"source_event_id":"se-u2"}
		]}`, srcMatch, started, arma1, arma2)

	if w := do(r, "POST", "/api/v1/ingest/match-results", reqOpt{service: "svc-token", body: matchBody}); w.Code != http.StatusOK {
		t.Fatalf("match ingest = %d, body=%s", w.Code, w.Body.String())
	}
	// Idempotency: re-post the same batch.
	if w := do(r, "POST", "/api/v1/ingest/match-results", reqOpt{service: "svc-token", body: matchBody}); w.Code != http.StatusOK {
		t.Fatalf("match ingest (2nd) = %d", w.Code)
	}
	var statCount int64
	gdb.Model(&models.MatchPlayerStat{}).Where("arma_id IN ?", []string{arma1, arma2}).Count(&statCount)
	if statCount != 2 {
		t.Fatalf("player stat rows = %d, want 2 (idempotent)", statCount)
	}

	// total_deployments recomputed to 1 for u1.
	var reloaded models.User
	gdb.First(&reloaded, "discord_id = ?", u1)
	if reloaded.TotalDeployments != 1 {
		t.Fatalf("u1 total_deployments = %d, want 1", reloaded.TotalDeployments)
	}

	// Leaderboard (K/D): u1 (5.0) must rank above u2 (1.0).
	w = do(r, "GET", "/api/v1/leaderboards?category=kd&limit=50", reqOpt{bearer: userTok})
	var lb struct {
		Data []leaderboardRow `json:"data"`
	}
	mustJSON(t, w, &lb)
	pos1, pos2 := -1, -1
	for i, row := range lb.Data {
		switch row.DiscordID {
		case u1:
			pos1 = i
			if row.KDRatio != 5 {
				t.Errorf("u1 kd = %v, want 5", row.KDRatio)
			}
		case u2:
			pos2 = i
		}
	}
	if pos1 < 0 || pos2 < 0 || pos1 >= pos2 {
		t.Fatalf("leaderboard order wrong: pos(u1)=%d pos(u2)=%d", pos1, pos2)
	}

	// Individual stat card.
	w = do(r, "GET", "/api/v1/users/"+u1+"/stats", reqOpt{bearer: userTok})
	var statsResp struct {
		Stats           leaderboardRow `json:"stats"`
		TotalOperations int            `json:"total_operations"`
	}
	mustJSON(t, w, &statsResp)
	if statsResp.Stats.Kills != 10 || statsResp.TotalOperations != 1 {
		t.Fatalf("user stats = %+v", statsResp)
	}
}
