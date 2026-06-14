package handlers_test

import (
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/go-chi/chi/v5"

	"github.com/tbdevent/website/internal/config"
	"github.com/tbdevent/website/internal/handlers"
	appmiddleware "github.com/tbdevent/website/internal/middleware"
)

const testToken = "test-server-token"

func newGameServerRouter(t *testing.T) (http.Handler, string) {
	t.Helper()

	dir := t.TempDir()
	mission := `{"schemaVersion":"1.0","meta":{"name":"Test Mission"}}`
	if err := os.WriteFile(filepath.Join(dir, "msn_test.json"), []byte(mission), 0o600); err != nil {
		t.Fatalf("write mission: %v", err)
	}

	h := handlers.NewGameServerHandler(nil, dir)
	mw := appmiddleware.NewServerTokenMiddleware(config.GameServerConfig{Tokens: []string{testToken}})

	r := chi.NewRouter()
	r.Route("/api", func(api chi.Router) {
		api.Group(func(gs chi.Router) {
			gs.Use(mw.RequireServerToken)
			gs.Get("/missions", h.MissionList)
			gs.Get("/missions/{id}/compiled", h.MissionCompiled)
			gs.Post("/results", h.PostResults)
			gs.Post("/telemetry", h.PostTelemetry)
		})
	})
	return r, mission
}

func do(t *testing.T, r http.Handler, method, path, token, body string) *httptest.ResponseRecorder {
	t.Helper()
	var reader *strings.Reader
	if body != "" {
		reader = strings.NewReader(body)
	} else {
		reader = strings.NewReader("")
	}
	req := httptest.NewRequest(method, path, reader)
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	rec := httptest.NewRecorder()
	r.ServeHTTP(rec, req)
	return rec
}

func TestMissionCompiled_RequiresToken(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions/msn_test/compiled", "", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("want 401, got %d", rec.Code)
	}
}

func TestMissionCompiled_RejectsBadToken(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions/msn_test/compiled", "wrong", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("want 401, got %d", rec.Code)
	}
}

func TestMissionCompiled_Served(t *testing.T) {
	r, mission := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions/msn_test/compiled", testToken, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", rec.Code)
	}
	if strings.TrimSpace(rec.Body.String()) != mission {
		t.Fatalf("body mismatch: %s", rec.Body.String())
	}
	if ct := rec.Header().Get("Content-Type"); ct != "application/json" {
		t.Fatalf("want json content-type, got %q", ct)
	}
}

func TestMissionCompiled_NotFound(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions/msn_missing/compiled", testToken, "")
	if rec.Code != http.StatusNotFound {
		t.Fatalf("want 404, got %d", rec.Code)
	}
}

func TestMissionCompiled_RejectsTraversal(t *testing.T) {
	r, _ := newGameServerRouter(t)
	// chi cleans paths, so an encoded traversal id reaches the handler as a single segment.
	rec := do(t, r, http.MethodGet, "/api/missions/..%2f..%2fsecret/compiled", testToken, "")
	if rec.Code != http.StatusBadRequest && rec.Code != http.StatusNotFound {
		t.Fatalf("want 400 or 404, got %d", rec.Code)
	}
}

func TestMissionList_RequiresToken(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions", "", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("want 401, got %d", rec.Code)
	}
}

func TestMissionList_MergesDiskMissions(t *testing.T) {
	// Nil repo (test harness) -> list falls back to disk, like MissionCompiled.
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodGet, "/api/missions", testToken, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", rec.Code)
	}
	body := rec.Body.String()
	if !strings.Contains(body, "msn_test") || !strings.Contains(body, "Test Mission") {
		t.Fatalf("disk mission not listed: %s", body)
	}
}

func TestPostResults_Accepts(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodPost, "/api/results", testToken, `{"missionId":"msn_test","winner":"blufor"}`)
	if rec.Code != http.StatusAccepted {
		t.Fatalf("want 202, got %d", rec.Code)
	}
}

func TestPostTelemetry_RejectsBadJSON(t *testing.T) {
	r, _ := newGameServerRouter(t)
	rec := do(t, r, http.MethodPost, "/api/telemetry", testToken, `not json`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", rec.Code)
	}
}
