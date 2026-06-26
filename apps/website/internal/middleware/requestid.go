package middleware

import (
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

const ctxRequestID = "request_id"

// RequestID assigns a request ID (honoring an inbound X-Request-ID) and echoes
// it on the response so logs and clients can correlate a request.
func RequestID() gin.HandlerFunc {
	return func(c *gin.Context) {
		id := c.GetHeader("X-Request-ID")
		if id == "" {
			id = uuid.NewString()
		}
		c.Set(ctxRequestID, id)
		c.Writer.Header().Set("X-Request-ID", id)
		c.Next()
	}
}

// GetRequestID returns the current request's ID, or "" if unset.
func GetRequestID(c *gin.Context) string {
	v, _ := c.Get(ctxRequestID)
	s, _ := v.(string)
	return s
}

// Logger logs one structured line per request including the request ID, status,
// latency, method, path, and client IP.
func Logger() gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		path := c.Request.URL.Path
		if raw := c.Request.URL.RawQuery; raw != "" {
			path += "?" + raw
		}
		c.Next()
		gin.DefaultWriter.Write([]byte(
			start.Format("2006/01/02 15:04:05") + " [" + GetRequestID(c) + "] " +
				c.Request.Method + " " + path + " " +
				itoa(c.Writer.Status()) + " " +
				time.Since(start).String() + " " +
				c.ClientIP() + "\n",
		))
	}
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	var b [4]byte
	i := len(b)
	for n > 0 {
		i--
		b[i] = byte('0' + n%10)
		n /= 10
	}
	return string(b[i:])
}
