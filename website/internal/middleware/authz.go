package middleware

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

// roleRank gives an ordering so that a higher role satisfies a lower requirement
// (admin can do anything a mission_maker can, etc.).
func roleRank(r string) int {
	switch r {
	case "admin":
		return 4
	case "mission_maker":
		return 3
	case "leader":
		return 2
	case "enlisted":
		return 1
	default:
		return 0
	}
}

// RequireMinRole aborts with 403 unless the caller's role is at least `min`.
// Must run after RequireAuth. Example: RequireMinRole("admin").
func RequireMinRole(min string) gin.HandlerFunc {
	threshold := roleRank(min)
	return func(c *gin.Context) {
		if roleRank(Role(c)) < threshold {
			c.AbortWithStatusJSON(http.StatusForbidden, gin.H{"error": "insufficient role"})
			return
		}
		c.Next()
	}
}
