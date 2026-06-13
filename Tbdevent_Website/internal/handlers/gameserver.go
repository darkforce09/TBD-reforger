package handlers

import (
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"regexp"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

// GameServerHandler serves the server-token authed API consumed by dedicated
// game servers: fetch mission JSON, roster, identity link codes, results/telemetry.
type GameServerHandler struct {
	repo        *repository.Repository
	missionsDir string
}

func NewGameServerHandler(repo *repository.Repository, missionsDir string) *GameServerHandler {
	return &GameServerHandler{repo: repo, missionsDir: missionsDir}
}

var missionIDPattern = regexp.MustCompile(`^[A-Za-z0-9_-]+$`)

const maxBodyBytes = 8 << 20 // 8 MiB

// MissionCompiled serves GET /api/missions/{id}/compiled (DB first, then disk).
func (h *GameServerHandler) MissionCompiled(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	if !missionIDPattern.MatchString(id) {
		writeError(w, http.StatusBadRequest, "invalid mission id")
		return
	}

	if h.repo != nil {
		data, err := h.repo.GetMissionCompiled(r.Context(), id)
		if err == nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write(data)
			return
		}
		if !errors.Is(err, repository.ErrNotFound) {
			log.Printf("gameserver: db mission %s: %v", id, err)
			writeError(w, http.StatusInternalServerError, "failed to read mission")
			return
		}
	}

	path := filepath.Join(h.missionsDir, id+".json")
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			writeError(w, http.StatusNotFound, "mission not found")
			return
		}
		log.Printf("gameserver: read mission %s: %v", id, err)
		writeError(w, http.StatusInternalServerError, "failed to read mission")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(data)
}

// PostLink serves POST /api/link — game server registers a 6-digit lobby code.
func (h *GameServerHandler) PostLink(w http.ResponseWriter, r *http.Request) {
	if h.repo == nil {
		writeError(w, http.StatusServiceUnavailable, "database required")
		return
	}

	var input models.RegisterLinkCodeInput
	if err := json.NewDecoder(io.LimitReader(r.Body, 4096)).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.repo.RegisterLinkCode(r.Context(), input); err != nil {
		if errors.Is(err, repository.ErrLinkCodeInvalid) {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to register link code")
		return
	}
	writeJSON(w, http.StatusAccepted, map[string]string{"status": "registered"})
}

// GameRoster serves GET /api/game/events/{id}/roster for game servers (identityId → slotId).
func (h *GameServerHandler) GameRoster(w http.ResponseWriter, r *http.Request) {
	if h.repo == nil {
		writeError(w, http.StatusServiceUnavailable, "database required")
		return
	}

	eventID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}

	roster, err := h.repo.GameRosterForEvent(r.Context(), eventID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to build roster")
		return
	}
	writeJSON(w, http.StatusOK, roster)
}

// PostResults serves POST /api/results (log-only until Phase 2 persistence).
func (h *GameServerHandler) PostResults(w http.ResponseWriter, r *http.Request) {
	payload, ok := decodeGameBody(w, r)
	if !ok {
		return
	}
	log.Printf("gameserver: results received (%d top-level keys)", len(payload))
	writeJSON(w, http.StatusAccepted, map[string]string{"status": "accepted"})
}

// PostTelemetry serves POST /api/telemetry (log-only).
func (h *GameServerHandler) PostTelemetry(w http.ResponseWriter, r *http.Request) {
	payload, ok := decodeGameBody(w, r)
	if !ok {
		return
	}
	log.Printf("gameserver: telemetry batch received (%d top-level keys)", len(payload))
	writeJSON(w, http.StatusAccepted, map[string]string{"status": "accepted"})
}

func decodeGameBody(w http.ResponseWriter, r *http.Request) (map[string]json.RawMessage, bool) {
	defer r.Body.Close()
	var payload map[string]json.RawMessage
	dec := json.NewDecoder(io.LimitReader(r.Body, maxBodyBytes))
	if err := dec.Decode(&payload); err != nil {
		writeError(w, http.StatusBadRequest, "invalid JSON body")
		return nil, false
	}
	return payload, true
}
