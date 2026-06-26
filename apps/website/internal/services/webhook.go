package services

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// WebhookService pushes announcement embeds to the Discord #announcements channel.
type WebhookService struct {
	url  string
	http *http.Client
}

// NewWebhookService constructs the service with the configured webhook URL
// (empty disables pushing).
func NewWebhookService(url string) *WebhookService {
	return &WebhookService{url: url, http: &http.Client{Timeout: 10 * time.Second}}
}

// SetURL overrides the webhook URL (used by tests with httptest).
func (w *WebhookService) SetURL(u string) { w.url = u }

// Enabled reports whether a webhook URL is configured.
func (w *WebhookService) Enabled() bool { return w.url != "" }

type embedFooter struct {
	Text string `json:"text"`
}

type embed struct {
	Title       string       `json:"title"`
	Description string       `json:"description,omitempty"`
	Color       int          `json:"color,omitempty"`
	Timestamp   string       `json:"timestamp,omitempty"`
	Footer      *embedFooter `json:"footer,omitempty"`
}

type webhookPayload struct {
	Username string  `json:"username,omitempty"`
	Embeds   []embed `json:"embeds"`
}

type webhookResponse struct {
	ID string `json:"id"`
}

// tagColor maps an announcement tag to an embed sidebar color.
func tagColor(tag models.AnnouncementTag) int {
	switch tag {
	case models.TagImportant:
		return 0xF87171
	case models.TagEvent:
		return 0x4D8EFF
	case models.TagModpackUpdate:
		return 0x7BD0FF
	default:
		return 0xADC6FF
	}
}

// PushAnnouncement posts the announcement as an embed and returns the created
// Discord message ID (via ?wait=true). Errors are returned for the caller to
// log as a CRIT audit event.
func (w *WebhookService) PushAnnouncement(ctx context.Context, a *models.Announcement) (string, error) {
	if !w.Enabled() {
		return "", fmt.Errorf("webhook not configured")
	}
	desc := a.Snippet
	if desc == "" {
		desc = truncate(a.Body, 500)
	}
	payload := webhookPayload{
		Username: "TBD Operations",
		Embeds: []embed{{
			Title:       a.Title,
			Description: desc,
			Color:       tagColor(a.Tag),
			Timestamp:   time.Now().UTC().Format(time.RFC3339),
			Footer:      &embedFooter{Text: "Category: " + string(a.Tag)},
		}},
	}
	buf, err := json.Marshal(payload)
	if err != nil {
		return "", err
	}

	url := w.url
	if strings.Contains(url, "?") {
		url += "&wait=true"
	} else {
		url += "?wait=true"
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(buf))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := w.http.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<12))
		return "", fmt.Errorf("webhook push: status %d: %s", resp.StatusCode, string(body))
	}
	var out webhookResponse
	_ = json.NewDecoder(resp.Body).Decode(&out)
	return out.ID, nil
}

// truncate shortens s to at most n runes, appending an ellipsis when cut.
func truncate(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	return string(r[:n]) + "…"
}
