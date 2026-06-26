package middleware

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
	"golang.org/x/time/rate"
)

func newEngine(mw ...gin.HandlerFunc) *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(mw...)
	r.GET("/ping", func(c *gin.Context) { c.String(http.StatusOK, "pong") })
	r.POST("/auth/login", func(c *gin.Context) { c.String(http.StatusOK, "ok") })
	return r
}

func TestCORSPreflightAllowedOrigin(t *testing.T) {
	r := newEngine(CORS([]string{"http://localhost:5173"}))
	req := httptest.NewRequest(http.MethodOptions, "/ping", nil)
	req.Header.Set("Origin", "http://localhost:5173")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusNoContent {
		t.Fatalf("preflight status = %d, want 204", w.Code)
	}
	if got := w.Header().Get("Access-Control-Allow-Origin"); got != "http://localhost:5173" {
		t.Errorf("allow-origin = %q", got)
	}
	if w.Header().Get("Access-Control-Allow-Credentials") != "" {
		t.Error("credentials must not be enabled for bearer-token auth")
	}
}

func TestCORSDisallowedOriginNotReflected(t *testing.T) {
	r := newEngine(CORS([]string{"http://localhost:5173"}))
	req := httptest.NewRequest(http.MethodGet, "/ping", nil)
	req.Header.Set("Origin", "https://evil.example")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if got := w.Header().Get("Access-Control-Allow-Origin"); got != "" {
		t.Errorf("disallowed origin reflected: %q", got)
	}
	if w.Code != http.StatusOK {
		t.Errorf("non-preflight request blocked: %d", w.Code)
	}
}

func TestRequestIDEchoed(t *testing.T) {
	r := newEngine(RequestID())
	req := httptest.NewRequest(http.MethodGet, "/ping", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if w.Header().Get("X-Request-ID") == "" {
		t.Error("expected X-Request-ID response header")
	}
}

func TestRequestIDHonorsInbound(t *testing.T) {
	r := newEngine(RequestID())
	req := httptest.NewRequest(http.MethodGet, "/ping", nil)
	req.Header.Set("X-Request-ID", "trace-123")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if got := w.Header().Get("X-Request-ID"); got != "trace-123" {
		t.Errorf("request id = %q, want trace-123", got)
	}
}

func TestRateLimitBlocksBurst(t *testing.T) {
	// Strict limiter: rate 0, burst 2 -> only 2 requests allowed, then 429.
	global := NewIPLimiter(rate.Limit(100), 100)
	strict := NewIPLimiter(rate.Limit(0), 2)
	r := newEngine(RateLimit(global, strict, []string{"/auth/"}))

	codes := make([]int, 0, 3)
	for i := 0; i < 3; i++ {
		req := httptest.NewRequest(http.MethodPost, "/auth/login", nil)
		w := httptest.NewRecorder()
		r.ServeHTTP(w, req)
		codes = append(codes, w.Code)
	}
	if codes[0] != 200 || codes[1] != 200 || codes[2] != http.StatusTooManyRequests {
		t.Fatalf("rate limit codes = %v, want [200 200 429]", codes)
	}
}

func TestRateLimitGlobalPathUsesGlobalBucket(t *testing.T) {
	// Global is generous; /ping is not a strict prefix, so it should pass.
	global := NewIPLimiter(rate.Limit(100), 100)
	strict := NewIPLimiter(rate.Limit(0), 0)
	r := newEngine(RateLimit(global, strict, []string{"/auth/"}))

	req := httptest.NewRequest(http.MethodGet, "/ping", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("global path blocked: %d", w.Code)
	}
}
