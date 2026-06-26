package services

import (
	"strings"

	"github.com/microcosm-cc/bluemonday"
)

// htmlPolicy sanitizes user-authored rich text (e.g. announcement bodies) down
// to a safe subset of HTML — scripts, event handlers, javascript: URLs, and
// other XSS vectors are stripped. UGCPolicy is bluemonday's standard policy for
// untrusted user-generated content. The policy is immutable and safe for
// concurrent use, so we build it once.
var htmlPolicy = bluemonday.UGCPolicy()

// SanitizeHTML returns body with all unsafe HTML removed, leaving only the
// formatting tags permitted by the UGC policy. The result is safe to persist
// and to render verbatim on the client.
func SanitizeHTML(body string) string {
	return htmlPolicy.Sanitize(body)
}

// Snippet produces a short plain-ish preview from body content: it collapses
// whitespace and truncates to at most n runes (appending an ellipsis if cut).
func Snippet(body string, n int) string {
	collapsed := strings.Join(strings.Fields(body), " ")
	return truncate(collapsed, n)
}
