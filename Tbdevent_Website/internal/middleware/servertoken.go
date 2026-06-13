package middleware

import (
	"net/http"
	"strings"

	"github.com/tbdevent/website/internal/config"
)

// ServerTokenMiddleware authenticates dedicated game servers via a bearer token.
// Used for the machine-to-machine API (missions, results, telemetry) — never a
// user session.
type ServerTokenMiddleware struct {
	cfg config.GameServerConfig
}

func NewServerTokenMiddleware(cfg config.GameServerConfig) *ServerTokenMiddleware {
	return &ServerTokenMiddleware{cfg: cfg}
}

func (m *ServerTokenMiddleware) RequireServerToken(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		token := bearerToken(r)
		if token == "" || !m.cfg.HasToken(token) {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "invalid server token"})
			return
		}
		next.ServeHTTP(w, r)
	})
}

func bearerToken(r *http.Request) string {
	h := r.Header.Get("Authorization")
	const prefix = "Bearer "
	if len(h) > len(prefix) && strings.EqualFold(h[:len(prefix)], prefix) {
		return strings.TrimSpace(h[len(prefix):])
	}
	return ""
}
