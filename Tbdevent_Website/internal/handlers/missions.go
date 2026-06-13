package handlers

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"os"
	"path/filepath"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/middleware"
	"github.com/tbdevent/website/internal/missionvalidate"
	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type MissionsHandler struct {
	repo        *repository.Repository
	schemaDir   string
	missionsDir string
}

func NewMissionsHandler(repo *repository.Repository, schemaDir, missionsDir string) *MissionsHandler {
	return &MissionsHandler{repo: repo, schemaDir: schemaDir, missionsDir: missionsDir}
}

// Publish serves POST /api/missions — admin uploads validated Mission JSON.
func (h *MissionsHandler) Publish(w http.ResponseWriter, r *http.Request) {
	raw, err := io.ReadAll(io.LimitReader(r.Body, maxBodyBytes))
	if err != nil {
		writeError(w, http.StatusBadRequest, "failed to read body")
		return
	}

	if err := missionvalidate.Validate(h.schemaDir, raw); err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	mission, err := h.repo.PublishMission(r.Context(), raw, &user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrMissionExists) {
			writeError(w, http.StatusConflict, "mission id already published")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to publish mission")
		return
	}

	if err := os.MkdirAll(h.missionsDir, 0o755); err == nil {
		path := filepath.Join(h.missionsDir, mission.ID+".json")
		_ = os.WriteFile(path, raw, 0o600)
	}

	writeJSON(w, http.StatusCreated, mission)
}

// LinkGameAccount serves POST /api/me/link — user consumes a 6-digit code from in-game lobby.
func (h *MissionsHandler) LinkGameAccount(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	var input models.ConsumeLinkCodeInput
	if err := json.NewDecoder(io.LimitReader(r.Body, 4096)).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	gi, err := h.repo.ConsumeLinkCode(r.Context(), user.ID, input.Code)
	if err != nil {
		switch {
		case errors.Is(err, repository.ErrLinkCodeInvalid):
			writeError(w, http.StatusBadRequest, "invalid or expired code")
		case errors.Is(err, repository.ErrLinkCodeUsed):
			writeError(w, http.StatusConflict, "code already used")
		case errors.Is(err, repository.ErrAlreadyLinked):
			writeError(w, http.StatusConflict, "account already linked")
		default:
			writeError(w, http.StatusInternalServerError, "failed to link account")
		}
		return
	}
	writeJSON(w, http.StatusOK, gi)
}

// MyGameIdentity serves GET /api/me/game-identity.
func (h *MissionsHandler) MyGameIdentity(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	gi, err := h.repo.GetGameIdentityByUser(r.Context(), user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeJSON(w, http.StatusOK, nil)
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to get game identity")
		return
	}
	writeJSON(w, http.StatusOK, gi)
}

// AdminAssignSlot serves PUT /api/admin/events/{id}/slots/{slotId}.
func (h *MissionsHandler) AdminAssignSlot(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	eventID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}
	slotID := chi.URLParam(r, "slotId")
	if slotID == "" {
		writeError(w, http.StatusBadRequest, "slot id required")
		return
	}

	var input models.AssignSlotInput
	if err := json.NewDecoder(io.LimitReader(r.Body, 4096)).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	a, err := h.repo.AssignEventSlot(r.Context(), eventID, slotID, input.UserID, user.ID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to assign slot")
		return
	}
	writeJSON(w, http.StatusOK, a)
}

// AdminListSlots serves GET /api/admin/events/{id}/slots.
func (h *MissionsHandler) AdminListSlots(w http.ResponseWriter, r *http.Request) {
	eventID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}

	slots, err := h.repo.ListEventSlotAssignments(r.Context(), eventID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list slots")
		return
	}
	if slots == nil {
		slots = []models.EventSlotAssignment{}
	}
	writeJSON(w, http.StatusOK, slots)
}
