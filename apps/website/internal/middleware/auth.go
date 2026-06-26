// Package middleware holds gin middleware for authentication and authorization.
package middleware

import (
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
)

// Context keys set by RequireAuth and read by handlers.
const (
	ctxDiscordID  = "discord_id"
	ctxRole       = "role"
	ctxArmaLinked = "arma_linked"
)

// RequireAuth validates the Bearer access token and populates the request
// context with the caller's identity.
func RequireAuth(jm *auth.Manager) gin.HandlerFunc {
	return func(c *gin.Context) {
		h := c.GetHeader("Authorization")
		if !strings.HasPrefix(h, "Bearer ") {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "missing bearer token"})
			return
		}
		claims, err := jm.Parse(strings.TrimSpace(strings.TrimPrefix(h, "Bearer ")))
		if err != nil {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid or expired token"})
			return
		}
		c.Set(ctxDiscordID, claims.Subject)
		c.Set(ctxRole, claims.Role)
		c.Set(ctxArmaLinked, claims.ArmaLinked)
		c.Next()
	}
}

// RequireServiceToken guards game-server ingest routes with a shared secret in
// the X-Service-Token header, compared in constant time.
func RequireServiceToken(token string) gin.HandlerFunc {
	return func(c *gin.Context) {
		got := c.GetHeader("X-Service-Token")
		if token == "" || !auth.ConstantTimeEqual(got, token) {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid service token"})
			return
		}
		c.Next()
	}
}

// DiscordID returns the authenticated caller's Discord ID, or "" if unset.
func DiscordID(c *gin.Context) string {
	v, _ := c.Get(ctxDiscordID)
	s, _ := v.(string)
	return s
}

// Role returns the authenticated caller's web role, or "" if unset.
func Role(c *gin.Context) string {
	v, _ := c.Get(ctxRole)
	s, _ := v.(string)
	return s
}
