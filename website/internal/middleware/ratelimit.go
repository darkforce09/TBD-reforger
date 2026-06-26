package middleware

import (
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/gin-gonic/gin"
	"golang.org/x/time/rate"
)

// IPLimiter holds a per-client-IP token bucket. In-memory and single-instance
// (same caveat as internal/realtime); a multi-instance deployment would back
// this with Redis behind the same Allow() check.
type IPLimiter struct {
	mu      sync.Mutex
	clients map[string]*clientEntry
	rate    rate.Limit
	burst   int
}

type clientEntry struct {
	limiter  *rate.Limiter
	lastSeen time.Time
}

// NewIPLimiter creates a limiter allowing r requests/second with the given burst,
// and starts a background sweeper that evicts idle clients.
func NewIPLimiter(r rate.Limit, burst int) *IPLimiter {
	l := &IPLimiter{clients: make(map[string]*clientEntry), rate: r, burst: burst}
	go l.sweep()
	return l
}

func (l *IPLimiter) get(ip string) *rate.Limiter {
	l.mu.Lock()
	defer l.mu.Unlock()
	e, ok := l.clients[ip]
	if !ok {
		e = &clientEntry{limiter: rate.NewLimiter(l.rate, l.burst)}
		l.clients[ip] = e
	}
	e.lastSeen = time.Now()
	return e.limiter
}

func (l *IPLimiter) sweep() {
	for range time.Tick(time.Minute) {
		l.mu.Lock()
		for ip, e := range l.clients {
			if time.Since(e.lastSeen) > 3*time.Minute {
				delete(l.clients, ip)
			}
		}
		l.mu.Unlock()
	}
}

// RateLimit applies the global limiter, switching to the strict limiter for
// requests whose path matches one of strictPrefixes (e.g. /auth, /ingest).
func RateLimit(global, strict *IPLimiter, strictPrefixes []string) gin.HandlerFunc {
	return func(c *gin.Context) {
		lim := global
		path := c.Request.URL.Path
		for _, p := range strictPrefixes {
			if strings.Contains(path, p) {
				lim = strict
				break
			}
		}
		if !lim.get(c.ClientIP()).Allow() {
			c.AbortWithStatusJSON(http.StatusTooManyRequests, gin.H{"error": "rate limit exceeded"})
			return
		}
		c.Next()
	}
}
