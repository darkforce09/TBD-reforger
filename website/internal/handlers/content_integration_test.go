package handlers

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestCMSAndContentIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	adminID := fmt.Sprintf("itest-admin-%d", time.Now().UnixNano())
	userID := fmt.Sprintf("itest-user-%d", time.Now().UnixNano())
	slug := fmt.Sprintf("roe-%d", time.Now().UnixNano())

	t.Cleanup(func() {
		gdb.Where("author_id = ?", adminID).Delete(&models.Announcement{})
		gdb.Where("slug = ?", slug).Delete(&models.WikiPage{})
		gdb.Where("action = ? AND target_type = ?", "webhook.push_failed", "announcement").Delete(&models.AuditLog{})
		gdb.Where("actor_id IN ?", []string{adminID, userID}).Delete(&models.AuditLog{})
		gdb.Unscoped().Where("discord_id IN ?", []string{adminID, userID}).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: adminID, Username: "Admin Dave", Role: models.RoleAdmin})
	gdb.Create(&models.User{DiscordID: userID, Username: "Enlisted Joe", Role: models.RoleEnlisted})
	adminTok, _, _ := h.JWT().IssueAccess(adminID, "admin", false)
	userTok, _, _ := h.JWT().IssueAccess(userID, "enlisted", false)

	// --- authz: non-admin cannot use the CMS ---
	if w := do(r, "POST", "/api/v1/cms/announcements", reqOpt{bearer: userTok, body: `{"title":"x","body":"y"}`}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted POST /cms/announcements = %d, want 403", w.Code)
	}

	// --- publish with successful webhook push ---
	var pushedBody bool
	okHook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		pushedBody = true
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"msg-1"}`))
	}))
	defer okHook.Close()
	h.Webhook().SetURL(okHook.URL)

	body := `{"title":"Hardware Upgrade","body":"Migration to high-tickrate complete.","tag":"update","status":"published","push_to_discord":true}`
	w := do(r, "POST", "/api/v1/cms/announcements", reqOpt{bearer: adminTok, body: body})
	if w.Code != http.StatusCreated {
		t.Fatalf("create published announcement = %d, body=%s", w.Code, w.Body.String())
	}
	var created models.Announcement
	mustJSON(t, w, &created)
	if !created.PushedToDiscord || created.DiscordMessageID != "msg-1" {
		t.Fatalf("expected pushed with msg id, got pushed=%v id=%q", created.PushedToDiscord, created.DiscordMessageID)
	}
	if !pushedBody {
		t.Fatal("webhook server was not called")
	}

	// --- it appears in the published feed and by id ---
	w = do(r, "GET", "/api/v1/announcements", reqOpt{bearer: userTok})
	var feed struct {
		Data  []models.Announcement `json:"data"`
		Total int64                 `json:"total"`
	}
	mustJSON(t, w, &feed)
	if feed.Total < 1 {
		t.Fatalf("feed total = %d, want >=1", feed.Total)
	}
	w = do(r, "GET", "/api/v1/announcements/"+created.ID.String(), reqOpt{bearer: userTok})
	if w.Code != http.StatusOK {
		t.Fatalf("GET announcement by id = %d", w.Code)
	}

	// --- webhook failure writes a CRIT audit row but does not fail creation ---
	badHook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		http.Error(w, "boom", http.StatusInternalServerError)
	}))
	defer badHook.Close()
	h.Webhook().SetURL(badHook.URL)

	body2 := `{"title":"Broken Push","body":"This push will fail.","status":"published","push_to_discord":true}`
	w = do(r, "POST", "/api/v1/cms/announcements", reqOpt{bearer: adminTok, body: body2})
	if w.Code != http.StatusCreated {
		t.Fatalf("create (failing push) = %d, body=%s", w.Code, w.Body.String())
	}
	var created2 models.Announcement
	mustJSON(t, w, &created2)
	if created2.PushedToDiscord {
		t.Fatal("expected pushed_to_discord=false after failed webhook")
	}
	var critCount int64
	gdb.Model(&models.AuditLog{}).
		Where("severity::text = ? AND action = ?", "crit", "webhook.push_failed").
		Count(&critCount)
	if critCount < 1 {
		t.Fatalf("expected a CRIT audit row for failed webhook push, found %d", critCount)
	}

	// --- wiki authoring + read ---
	wikiBody := `{"category":"Rules of Engagement (ROE)","title":"ROE","icon":"gavel","body_md":"# ROE\nDo not engage unidentified armor.","nav_order":1}`
	if w := do(r, "PUT", "/api/v1/wiki/"+slug, reqOpt{bearer: adminTok, body: wikiBody}); w.Code != http.StatusOK {
		t.Fatalf("PUT /wiki/%s = %d, body=%s", slug, w.Code, w.Body.String())
	}
	w = do(r, "GET", "/api/v1/wiki/"+slug, reqOpt{bearer: userTok})
	if w.Code != http.StatusOK {
		t.Fatalf("GET /wiki/%s = %d", slug, w.Code)
	}
	var page models.WikiPage
	mustJSON(t, w, &page)
	if page.Title != "ROE" || page.UpdatedBy == nil || *page.UpdatedBy != adminID {
		t.Fatalf("wiki page not saved correctly: %+v", page)
	}

	// --- dashboard renders (empty event data is fine pre-M6) ---
	if w := do(r, "GET", "/api/v1/dashboard", reqOpt{bearer: userTok}); w.Code != http.StatusOK {
		t.Fatalf("GET /dashboard = %d, body=%s", w.Code, w.Body.String())
	}
}
