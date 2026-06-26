package handlers

import (
	"fmt"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestAdminIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	adminID := fmt.Sprintf("itest-admin8-%d", time.Now().UnixNano())
	targetID := fmt.Sprintf("itest-tgt8-%d", time.Now().UnixNano())
	roleID := fmt.Sprintf("r-mm-%d", time.Now().UnixNano())
	mmRole := models.RoleMissionMaker

	t.Cleanup(func() {
		gdb.Where("discord_id IN ?", []string{adminID, targetID}).Delete(&models.Warning{})
		gdb.Where("discord_id IN ?", []string{adminID, targetID}).Delete(&models.UserDiscordRole{})
		gdb.Where("discord_role_id = ?", roleID).Delete(&models.DiscordRole{})
		gdb.Where("discord_id IN ?", []string{adminID, targetID}).Delete(&models.RefreshToken{})
		gdb.Where("actor_id IN ?", []string{adminID}).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id IN ?", []string{adminID, targetID}).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Sarah", Role: models.RoleAdmin})
	gdb.Create(&models.User{DiscordID: targetID, DiscordHandle: "RandomGuy#9999", Username: "RandomGuy", Role: models.RoleEnlisted})
	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	targetTok, _, _ := h.JWT().IssueAccess(targetID, "enlisted", false)

	// --- authz: non-admin blocked from roster ---
	if w := do(r, "GET", "/api/v1/admin/users", reqOpt{bearer: targetTok}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted roster = %d, want 403", w.Code)
	}

	// --- roster shows the target with zero warnings ---
	w := do(r, "GET", "/api/v1/admin/users?q=RandomGuy", reqOpt{bearer: adminTok})
	var roster struct {
		Data []rosterRow `json:"data"`
	}
	mustJSON(t, w, &roster)
	if len(roster.Data) != 1 || roster.Data[0].Warnings != 0 {
		t.Fatalf("roster = %+v", roster.Data)
	}

	// --- issue a warning -> count reflects ---
	if w := do(r, "POST", "/api/v1/admin/users/"+targetID+"/warnings", reqOpt{bearer: adminTok, body: `{"reason":"Intentional Teamkilling"}`}); w.Code != http.StatusCreated {
		t.Fatalf("warn = %d, body=%s", w.Code, w.Body.String())
	}
	w = do(r, "GET", "/api/v1/admin/users?q=RandomGuy", reqOpt{bearer: adminTok})
	mustJSON(t, w, &roster)
	if roster.Data[0].Warnings != 1 {
		t.Fatalf("warnings = %d, want 1", roster.Data[0].Warnings)
	}

	// --- change role ---
	if w := do(r, "PATCH", "/api/v1/admin/users/"+targetID, reqOpt{bearer: adminTok, body: `{"role":"mission_maker"}`}); w.Code != http.StatusOK {
		t.Fatalf("role change = %d", w.Code)
	}

	// --- ban: marks banned + revokes refresh tokens ---
	rawRefresh, _ := auth.RandomToken(32)
	gdb.Create(&models.RefreshToken{DiscordID: targetID, TokenHash: auth.HashToken(rawRefresh), ExpiresAt: time.Now().Add(time.Hour)})
	if w := do(r, "POST", "/api/v1/admin/users/"+targetID+"/ban", reqOpt{bearer: adminTok, body: `{"reason":"Intentional Teamkilling"}`}); w.Code != http.StatusOK {
		t.Fatalf("ban = %d", w.Code)
	}
	var banned models.User
	gdb.First(&banned, "discord_id = ?", targetID)
	if !banned.IsBanned {
		t.Fatal("user not marked banned")
	}
	// The previously valid refresh token must now be rejected.
	if w := do(r, "POST", "/api/v1/auth/refresh", reqOpt{body: fmt.Sprintf(`{"refresh_token":%q}`, rawRefresh)}); w.Code != http.StatusUnauthorized {
		t.Fatalf("refresh after ban = %d, want 401", w.Code)
	}
	// Ban audit row exists.
	var banAudit int64
	gdb.Model(&models.AuditLog{}).Where("action = ? AND target_id = ?", "user.ban", targetID).Count(&banAudit)
	if banAudit < 1 {
		t.Fatal("missing ban audit row")
	}

	// --- unban ---
	if w := do(r, "DELETE", "/api/v1/admin/users/"+targetID+"/ban", reqOpt{bearer: adminTok}); w.Code != http.StatusOK {
		t.Fatalf("unban = %d", w.Code)
	}

	// --- role resync: map a discord role to mission_maker, attach to target, resync ---
	gdb.Create(&models.DiscordRole{DiscordRoleID: roleID, Name: "MissionMaker", MappedRole: &mmRole, Priority: 5})
	gdb.Create(&models.UserDiscordRole{DiscordID: targetID, DiscordRoleID: roleID, SyncedAt: time.Now()})
	// First force role back to enlisted to observe the change.
	gdb.Model(&models.User{}).Where("discord_id = ?", targetID).Update("role", models.RoleEnlisted)
	w = do(r, "POST", "/api/v1/admin/roles/sync", reqOpt{bearer: adminTok})
	if w.Code != http.StatusOK {
		t.Fatalf("resync = %d", w.Code)
	}
	var resynced models.User
	gdb.First(&resynced, "discord_id = ?", targetID)
	if resynced.Role != models.RoleMissionMaker {
		t.Fatalf("resync role = %q, want mission_maker", resynced.Role)
	}

	// --- RCON: needs a server ---
	server := models.Server{Name: "RCON Box", IP: "127.0.0.1", Port: 2001}
	gdb.Create(&server)
	defer gdb.Where("id = ?", server.ID).Delete(&models.Server{})
	defer gdb.Where("target_id = ? AND action = ?", server.ID.String(), "server.rcon").Delete(&models.AuditLog{})
	if w := do(r, "POST", "/api/v1/admin/servers/"+server.ID.String()+"/rcon", reqOpt{bearer: adminTok, body: `{"action":"change_map","map":"Arland"}`}); w.Code != http.StatusAccepted {
		t.Fatalf("rcon = %d, body=%s", w.Code, w.Body.String())
	}

	// --- audit log console: list + severity filter ---
	w = do(r, "GET", "/api/v1/admin/audit-logs?severity=warn", reqOpt{bearer: adminTok})
	var logs struct {
		Data []models.AuditLog `json:"data"`
	}
	mustJSON(t, w, &logs)
	sawBan := false
	for _, l := range logs.Data {
		if l.Severity != models.SeverityWarn {
			t.Fatalf("severity filter leaked %q", l.Severity)
		}
		if l.Action == "user.ban" {
			sawBan = true
		}
	}
	if !sawBan {
		t.Fatal("expected ban entry in WARN audit feed")
	}

	// --- CSV export ---
	w = do(r, "GET", "/api/v1/admin/audit-logs/export.csv", reqOpt{bearer: adminTok})
	if w.Code != http.StatusOK {
		t.Fatalf("csv = %d", w.Code)
	}
	if ct := w.Header().Get("Content-Type"); !strings.HasPrefix(ct, "text/csv") {
		t.Fatalf("csv content-type = %q", ct)
	}
	if !strings.HasPrefix(w.Body.String(), "timestamp,severity,actor,action,message") {
		t.Fatalf("csv header wrong: %q", w.Body.String()[:60])
	}
}
