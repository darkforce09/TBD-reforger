package middleware

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

// TestIsMissionVersionPOST_RoutePattern verifies the skip matches the real Gin route
// (FullPath is set to the route pattern when the engine dispatches), and that the body
// is NOT wrapped at the 1 MB global cap on the version POST. Regression guard: a stale
// or mis-mounted build that let the global 1 MB cap reach this route caused the
// T-060.1.4 mid-upload reset on 100MB+ saves.
func TestIsMissionVersionPOST_RoutePattern(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()

	var capped bool // did GlobalBodyLimit wrap (cap) this request's body?
	r.Use(func(c *gin.Context) {
		capped = !isMissionVersionPOST(c)
		c.Next()
	})
	r.Use(GlobalBodyLimit(MaxJSONBody))

	v1 := r.Group("/api/v1")
	v1.POST("/missions/:id/versions", BodyLimit(256<<20), func(c *gin.Context) {
		// A 2 MB read must succeed — i.e. the global 1 MB cap was skipped. A MaxBytesReader
		// trip surfaces as a read error here.
		b, err := io.ReadAll(c.Request.Body)
		if err != nil {
			c.JSON(http.StatusRequestEntityTooLarge, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusOK, gin.H{"read": len(b)})
	})
	v1.POST("/missions", func(c *gin.Context) { c.Status(http.StatusOK) })

	// Version POST: skipped → 2 MB body reads fully.
	body := strings.NewReader(strings.Repeat("x", 2<<20))
	req := httptest.NewRequest(http.MethodPost, "/api/v1/missions/abc-123/versions", body)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if capped {
		t.Fatal("version POST was capped by GlobalBodyLimit — the route must be skipped")
	}
	if w.Code != http.StatusOK {
		t.Fatalf("version POST = %d, want 200 (2 MB read should not trip a cap)", w.Code)
	}

	// A non-version POST is capped.
	req = httptest.NewRequest(http.MethodPost, "/api/v1/missions", strings.NewReader("{}"))
	w = httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if !capped {
		t.Fatal("non-version POST should be capped by GlobalBodyLimit")
	}
}

// TestIsMissionVersionPOST_PathFallback verifies the concrete-URL-path fallback fires
// when FullPath() is empty (e.g. the predicate is consulted outside a matched route, or
// a future mount leaves FullPath unset).
func TestIsMissionVersionPOST_PathFallback(t *testing.T) {
	cases := []struct {
		method, path string
		want         bool
	}{
		{http.MethodPost, "/api/v1/missions/8513de64-4c3c/versions", true},
		{http.MethodPost, "/missions/abc/versions", true},
		{http.MethodPost, "/api/v1/missions/abc/versions/", true}, // trailing slash tolerated
		{http.MethodGet, "/api/v1/missions/abc/versions", false},  // wrong method
		{http.MethodPost, "/api/v1/missions/abc/versions/xyz", false},
		{http.MethodPost, "/api/v1/missions", false},
		{http.MethodPost, "/api/v1/events/abc/versions", false}, // not /missions/
	}
	for _, tc := range cases {
		c, _ := gin.CreateTestContext(httptest.NewRecorder())
		c.Request = httptest.NewRequest(tc.method, tc.path, nil)
		// CreateTestContext leaves FullPath empty (no route matched) → exercises the fallback.
		if got := isMissionVersionPOST(c); got != tc.want {
			t.Errorf("isMissionVersionPOST(%s %s) = %v, want %v", tc.method, tc.path, got, tc.want)
		}
	}
}
