package middleware

import (
	"context"
	"encoding/json"
	"net/http"

	"github.com/tbdevent/website/internal/auth"
	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type contextKey string

const userContextKey contextKey = "user"

type AuthMiddleware struct {
	sessions *auth.SessionManager
	repo     *repository.Repository
}

func NewAuthMiddleware(sessions *auth.SessionManager, repo *repository.Repository) *AuthMiddleware {
	return &AuthMiddleware{sessions: sessions, repo: repo}
}

func (m *AuthMiddleware) RequireLogin(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user, ok := m.userFromRequest(r)
		if !ok {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "unauthorized"})
			return
		}
		ctx := context.WithValue(r.Context(), userContextKey, user)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

func UserFromContext(ctx context.Context) (*models.User, bool) {
	user, ok := ctx.Value(userContextKey).(*models.User)
	return user, ok
}

func (m *AuthMiddleware) userFromRequest(r *http.Request) (*models.User, bool) {
	userID, ok := m.sessions.GetUserID(r)
	if !ok {
		return nil, false
	}

	user, err := m.repo.GetUserByID(r.Context(), userID)
	if err != nil {
		return nil, false
	}

	return user, true
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}
