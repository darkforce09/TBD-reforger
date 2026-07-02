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

func TestPushAnnouncementRetriesOn429(t *testing.T) {
	// T-130.2 F3-01: the webhook POST replays its body after a 429 and succeeds.
	attempts := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		attempts++
		if attempts == 1 {
			w.Header().Set("Retry-After", "0")
			http.Error(w, `{"message":"rate limited"}`, http.StatusTooManyRequests)
			return
		}
		var body map[string]any
		_ = json.NewDecoder(r.Body).Decode(&body)
		if _, ok := body["embeds"]; !ok {
			t.Error("retried request lost its body")
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"msg-retry"}`))
	}))
	defer srv.Close()

	wh := NewWebhookService(srv.URL)
	id, err := wh.PushAnnouncement(context.Background(), &models.Announcement{Title: "x", Tag: models.TagEvent})
	if err != nil {
		t.Fatalf("push after 429: %v", err)
	}
	if id != "msg-retry" || attempts != 2 {
		t.Errorf("id = %q attempts = %d, want msg-retry / 2", id, attempts)
	}
}

func TestPushAnnouncementCapsEmbedLimits(t *testing.T) {
	// T-130.2 F3-02: Discord rejects titles > 256 chars — the embed must be capped.
	var gotBody struct {
		Embeds []struct {
			Title  string `json:"title"`
			Footer struct {
				Text string `json:"text"`
			} `json:"footer"`
		} `json:"embeds"`
	}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewDecoder(r.Body).Decode(&gotBody)
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"msg-cap"}`))
	}))
	defer srv.Close()

	wh := NewWebhookService(srv.URL)
	a := &models.Announcement{Title: strings.Repeat("T", 300), Body: "body", Tag: models.TagEvent}
	if _, err := wh.PushAnnouncement(context.Background(), a); err != nil {
		t.Fatalf("push: %v", err)
	}
	if len(gotBody.Embeds) != 1 {
		t.Fatalf("expected 1 embed, got %d", len(gotBody.Embeds))
	}
	if n := len([]rune(gotBody.Embeds[0].Title)); n > 256 {
		t.Errorf("title = %d runes, want <= 256", n)
	}
	if n := len([]rune(gotBody.Embeds[0].Footer.Text)); n > 2048 {
		t.Errorf("footer = %d runes, want <= 2048", n)
	}
}

func TestCapRunes(t *testing.T) {
	if got := capRunes("short", 256); got != "short" {
		t.Errorf("capRunes(short) = %q", got)
	}
	got := capRunes(strings.Repeat("a", 300), 256)
	if n := len([]rune(got)); n != 256 {
		t.Errorf("capped length = %d runes, want exactly 256 (ellipsis included)", n)
	}
	if !strings.HasSuffix(got, "…") {
		t.Errorf("capped string missing ellipsis: %q", got[len(got)-8:])
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
