package handlers

import (
	"crypto/rand"
	"encoding/base64"
	"errors"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	"golang.org/x/oauth2"

	"github.com/tbdevent/website/internal/auth"
	"github.com/tbdevent/website/internal/config"
	"github.com/tbdevent/website/internal/middleware"
	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type AuthHandler struct {
	cfg      *config.Config
	discord  *auth.DiscordService
	sessions *auth.SessionManager
	repo     *repository.Repository
	states   *oauthStateStore
	tokens   *tokenStore
}

func NewAuthHandler(cfg *config.Config, discord *auth.DiscordService, sessions *auth.SessionManager, repo *repository.Repository) *AuthHandler {
	return &AuthHandler{
		cfg:      cfg,
		discord:  discord,
		sessions: sessions,
		repo:     repo,
		states:   newOAuthStateStore(),
		tokens:   newTokenStore(),
	}
}

func (h *AuthHandler) DiscordLogin(w http.ResponseWriter, r *http.Request) {
	if !h.cfg.Discord.OAuthConfigured() {
		writeError(w, http.StatusServiceUnavailable, "discord oauth is not configured")
		return
	}

	state, err := randomState()
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to create oauth state")
		return
	}
	returnTo := sanitizeReturnTo(r.URL.Query().Get("returnTo"))
	h.states.set(state, oauthStateEntry{
		expires:  time.Now().Add(10 * time.Minute),
		returnTo: returnTo,
	})

	http.Redirect(w, r, h.discord.AuthCodeURL(state), http.StatusTemporaryRedirect)
}

func (h *AuthHandler) DiscordCallback(w http.ResponseWriter, r *http.Request) {
	if !h.cfg.Discord.OAuthConfigured() {
		writeError(w, http.StatusServiceUnavailable, "discord oauth is not configured")
		return
	}

	state := r.URL.Query().Get("state")
	entry, ok := h.states.consume(state)
	if !ok {
		writeError(w, http.StatusBadRequest, "invalid oauth state")
		return
	}

	code := r.URL.Query().Get("code")
	if code == "" {
		writeError(w, http.StatusBadRequest, "missing oauth code")
		return
	}

	token, err := h.discord.Exchange(r.Context(), code)
	if err != nil {
		writeError(w, http.StatusBadGateway, "failed to exchange oauth code")
		return
	}

	discordUser, err := h.discord.FetchUser(r.Context(), token)
	if err != nil {
		writeError(w, http.StatusBadGateway, "failed to fetch discord user")
		return
	}

	user, err := h.repo.UpsertUser(
		r.Context(),
		discordUser.ID,
		h.discord.DisplayName(discordUser),
		h.discord.AvatarURL(discordUser),
	)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to persist user")
		return
	}

	if err := h.sessions.PutUserID(r, user.ID); err != nil {
		writeError(w, http.StatusInternalServerError, "failed to create session")
		return
	}

	h.tokens.set(user.ID.String(), token)

	redirectTo := h.cfg.BaseURL + "/"
	if entry.returnTo != "" {
		redirectTo = h.cfg.BaseURL + entry.returnTo
	}
	http.Redirect(w, r, redirectTo, http.StatusTemporaryRedirect)
}

func (h *AuthHandler) Me(w http.ResponseWriter, r *http.Request) {
	userID, ok := h.sessions.GetUserID(r)
	if !ok {
		writeError(w, http.StatusUnauthorized, "unauthorized")
		return
	}

	user, err := h.repo.GetUserByID(r.Context(), userID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusUnauthorized, "unauthorized")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to load user")
		return
	}

	token, _ := h.tokens.get(user.ID.String())
	isAdmin := h.discord.IsAdmin(r.Context(), user.DiscordID, token)

	writeJSON(w, http.StatusOK, models.AuthMeResponse{
		User:    *user,
		IsAdmin: isAdmin,
	})
}

func (h *AuthHandler) Logout(w http.ResponseWriter, r *http.Request) {
	if userID, ok := h.sessions.GetUserID(r); ok {
		h.tokens.delete(userID.String())
	}
	if err := h.sessions.Clear(r); err != nil {
		writeError(w, http.StatusInternalServerError, "failed to logout")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *AuthHandler) RequireAdmin(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user, ok := middleware.UserFromContext(r.Context())
		if !ok {
			writeError(w, http.StatusUnauthorized, "unauthorized")
			return
		}

		token, _ := h.tokens.get(user.ID.String())
		if !h.discord.IsAdmin(r.Context(), user.DiscordID, token) {
			writeError(w, http.StatusForbidden, "admin access required")
			return
		}

		next.ServeHTTP(w, r)
	})
}

func randomState() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}

type oauthStateEntry struct {
	expires  time.Time
	returnTo string
}

type oauthStateStore struct {
	mu     sync.Mutex
	states map[string]oauthStateEntry
}

func newOAuthStateStore() *oauthStateStore {
	return &oauthStateStore{states: make(map[string]oauthStateEntry)}
}

func (s *oauthStateStore) set(state string, entry oauthStateEntry) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.states[state] = entry
}

func (s *oauthStateStore) consume(state string) (oauthStateEntry, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	entry, ok := s.states[state]
	if !ok || time.Now().After(entry.expires) {
		delete(s.states, state)
		return oauthStateEntry{}, false
	}
	delete(s.states, state)
	return entry, true
}

func sanitizeReturnTo(returnTo string) string {
	if returnTo == "" {
		return ""
	}
	if !strings.HasPrefix(returnTo, "/") || strings.HasPrefix(returnTo, "//") {
		return ""
	}
	if _, err := url.Parse(returnTo); err != nil {
		return ""
	}
	return returnTo
}

type tokenStore struct {
	mu     sync.Mutex
	tokens map[string]*oauth2.Token
}

func newTokenStore() *tokenStore {
	return &tokenStore{tokens: make(map[string]*oauth2.Token)}
}

func (s *tokenStore) set(userID string, token *oauth2.Token) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.tokens[userID] = token
}

func (s *tokenStore) get(userID string) (*oauth2.Token, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	token, ok := s.tokens[userID]
	return token, ok
}

func (s *tokenStore) delete(userID string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.tokens, userID)
}
