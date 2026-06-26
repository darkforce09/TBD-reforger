package middleware

import (
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"
)

// CORS reflects an allow-listed Origin and answers preflight requests. The API
// authenticates with a bearer token (not cookies), so credentials are NOT
// enabled and the origin is reflected explicitly rather than using "*".
// In development the SPA talks through the Vite proxy (same-origin), so this is
// primarily for production where the SPA and API are on different origins.
func CORS(allowedOrigins []string) gin.HandlerFunc {
	allowed := make(map[string]bool, len(allowedOrigins))
	for _, o := range allowedOrigins {
		allowed[strings.TrimRight(o, "/")] = true
	}

	const (
		allowMethods = "GET, POST, PUT, PATCH, DELETE, OPTIONS"
		allowHeaders = "Authorization, Content-Type, X-Service-Token, X-Request-ID"
	)

	return func(c *gin.Context) {
		origin := c.GetHeader("Origin")
		if origin != "" && allowed[strings.TrimRight(origin, "/")] {
			h := c.Writer.Header()
			h.Set("Access-Control-Allow-Origin", origin)
			h.Set("Vary", "Origin")
			h.Set("Access-Control-Allow-Methods", allowMethods)
			h.Set("Access-Control-Allow-Headers", allowHeaders)
			h.Set("Access-Control-Max-Age", "600")
		}
		if c.Request.Method == http.MethodOptions {
			c.AbortWithStatus(http.StatusNoContent)
			return
		}
		c.Next()
	}
}
