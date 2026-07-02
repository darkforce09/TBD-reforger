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
// requests whose path starts with one of strictPrefixes. Prefixes are full rooted
// paths (e.g. /api/v1/auth/) matched with HasPrefix — a substring match would also
// catch unrelated routes that merely contain the fragment, such as a path parameter
// spelling "auth" (T-130.1 F2B-11).
func RateLimit(global, strict *IPLimiter, strictPrefixes []string) gin.HandlerFunc {
	return func(c *gin.Context) {
		lim := global
		path := c.Request.URL.Path
		for _, p := range strictPrefixes {
			if strings.HasPrefix(path, p) {
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
