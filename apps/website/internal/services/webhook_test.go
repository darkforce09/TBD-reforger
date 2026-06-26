package services

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

func TestPushAnnouncementSuccess(t *testing.T) {
	var gotBody map[string]any
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Query().Get("wait") != "true" {
			t.Errorf("expected wait=true, got %q", r.URL.RawQuery)
		}
		_ = json.NewDecoder(r.Body).Decode(&gotBody)
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"msg-987"}`))
	}))
	defer srv.Close()

	wh := NewWebhookService("")
	wh.SetURL(srv.URL)
	if !wh.Enabled() {
		t.Fatal("expected enabled")
	}

	a := &models.Announcement{Title: "Modpack v2.1", Body: "Sync before Tuesday.", Tag: models.TagModpackUpdate}
	id, err := wh.PushAnnouncement(context.Background(), a)
	if err != nil {
		t.Fatalf("push: %v", err)
	}
	if id != "msg-987" {
		t.Errorf("message id = %q, want msg-987", id)
	}
	embeds, ok := gotBody["embeds"].([]any)
	if !ok || len(embeds) != 1 {
		t.Fatalf("expected 1 embed, got %v", gotBody["embeds"])
	}
}

func TestPushAnnouncementDisabled(t *testing.T) {
	wh := NewWebhookService("")
	if _, err := wh.PushAnnouncement(context.Background(), &models.Announcement{Title: "x"}); err == nil {
		t.Fatal("expected error when webhook URL not configured")
	}
}

func TestPushAnnouncementServerError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "boom", http.StatusInternalServerError)
	}))
	defer srv.Close()
	wh := NewWebhookService(srv.URL)
	if _, err := wh.PushAnnouncement(context.Background(), &models.Announcement{Title: "x"}); err == nil {
		t.Fatal("expected error on 500 response")
	}
}

func TestSnippet(t *testing.T) {
	got := Snippet("  hello   world\n\tfoo  ", 100)
	if got != "hello world foo" {
		t.Errorf("Snippet = %q", got)
	}
	if got := Snippet(strings.Repeat("a", 50), 10); len([]rune(got)) != 11 { // 10 + ellipsis
		t.Errorf("truncated snippet length = %d", len([]rune(got)))
	}
}
