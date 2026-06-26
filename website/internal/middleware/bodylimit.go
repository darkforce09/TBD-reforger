package middleware

import (
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"
)

// Body size caps. JSON routes are held to a small default to defend against
// oversized payloads; multipart uploads get a larger cap (the per-file 5MB limit
// is enforced in the upload handler). The mission-version POST is the one route
// that legitimately carries a large body (a compiled mission can be hundreds of
// MB at scale) and is given its own much higher cap at registration — see
// MissionVersionRoute, BodyLimit, and cmd/api/main.go.
const (
	MaxJSONBody      = 1 << 20 // 1 MB — default JSON routes
	MaxMultipartBody = 6 << 20 // 6 MB — multipart uploads (per-file 5MB enforced in-handler)
)

// MissionVersionRoute is the Gin route pattern (suffix) for the large
// mission-version POST. GlobalBodyLimit skips it so the route-specific
// BodyLimit registered on the route is the only MaxBytesReader wrapping the body.
const MissionVersionRoute = "/missions/:id/versions"

// isMissionVersionPOST reports whether the request is the large mission-version
// POST, which GlobalBodyLimit must skip so the route's own (much higher) BodyLimit
// is the only MaxBytesReader on the body. It matches both the Gin route pattern via
// FullPath() AND the concrete request path (/…/missions/<id>/versions) as a fallback
// — so a 1 MB wrap can never silently apply to a 100MB+ save even if FullPath() is
// empty/unexpected at the point the global middleware runs (T-060.1.4).
func isMissionVersionPOST(c *gin.Context) bool {
	if c.Request.Method != http.MethodPost {
		return false
	}
	if strings.HasSuffix(c.FullPath(), MissionVersionRoute) {
		return true
	}
	// Fallback on the concrete URL path: /…/missions/<id>/versions.
	p := strings.TrimSuffix(c.Request.URL.Path, "/")
	if !strings.HasSuffix(p, "/versions") {
		return false
	}
	p = strings.TrimSuffix(p, "/versions")
	i := strings.LastIndex(p, "/")
	return i > 0 && strings.HasSuffix(p[:i], "/missions")
}

// GlobalBodyLimit caps every request body to `limit` bytes (multipart bumped to
// MaxMultipartBody) EXCEPT the mission-version POST, which it leaves untouched.
// That route is wrapped instead by its own BodyLimit at a far higher cap: a
// global MaxBytesReader(1 MB) wrapped first cannot be loosened by a later
// route-level limiter (the innermost limit wins), so the global must opt out.
func GlobalBodyLimit(limit int64) gin.HandlerFunc {
	return func(c *gin.Context) {
		if isMissionVersionPOST(c) {
			c.Next()
			return
		}
		max := limit
		if strings.HasPrefix(c.GetHeader("Content-Type"), "multipart/form-data") {
			max = MaxMultipartBody
		}
		c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, max)
		c.Next()
	}
}

// BodyLimit wraps the request body in http.MaxBytesReader(limit) unconditionally.
// Used as route-specific middleware (e.g. the mission-version POST) where the
// global cap is skipped. Reading past the limit makes the bind fail with
// *http.MaxBytesError, which the handler maps to 413.
func BodyLimit(limit int64) gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, limit)
		c.Next()
	}
}
