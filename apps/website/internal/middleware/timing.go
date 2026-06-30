package middleware

import (
	"time"

	"github.com/gin-gonic/gin"
)

// RequestStartKey is the gin context key under which Timing stores the request
// start time. Handlers read it (via logHandlerErr) to report handling duration
// in LOG-3 error logs.
const RequestStartKey = "reqStart"

// Timing records the request start time on the context so handlers can include a
// duration in their consequential-error logs (LOG-3). Mount once on the API
// group; it never writes to the response.
func Timing() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Set(RequestStartKey, time.Now())
		c.Next()
	}
}
