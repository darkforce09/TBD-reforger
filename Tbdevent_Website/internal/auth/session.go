package auth

import (
	"net/http"
	"time"

	"github.com/alexedwards/scs/v2"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/config"
)

const sessionUserIDKey = "userID"

type SessionManager struct {
	manager *scs.SessionManager
}

func NewSessionManager(cfg *config.Config) *SessionManager {
	m := scs.New()
	m.Lifetime = 24 * time.Hour
	m.Cookie.HttpOnly = true
	m.Cookie.SameSite = http.SameSiteLaxMode
	m.Cookie.Secure = cfg.IsProduction()
	m.Cookie.Path = "/"

	if cfg.IsProduction() {
		m.Cookie.Name = "tbdevent_session"
	}

	return &SessionManager{manager: m}
}

func (s *SessionManager) LoadAndSave(next http.Handler) http.Handler {
	return s.manager.LoadAndSave(next)
}

func (s *SessionManager) PutUserID(r *http.Request, userID uuid.UUID) error {
	s.manager.Put(r.Context(), sessionUserIDKey, userID.String())
	return nil
}

func (s *SessionManager) GetUserID(r *http.Request) (uuid.UUID, bool) {
	v := s.manager.GetString(r.Context(), sessionUserIDKey)
	if v == "" {
		return uuid.Nil, false
	}
	id, err := uuid.Parse(v)
	if err != nil {
		return uuid.Nil, false
	}
	return id, true
}

func (s *SessionManager) Clear(r *http.Request) error {
	return s.manager.Destroy(r.Context())
}

func (s *SessionManager) RenewToken(r *http.Request) error {
	return s.manager.RenewToken(r.Context())
}
