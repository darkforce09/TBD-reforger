package handlers

import (
	"encoding/json"
	"errors"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/auth"
	"github.com/tbdevent/website/internal/middleware"
	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type EventsHandler struct {
	repo     *repository.Repository
	sessions *auth.SessionManager
}

func NewEventsHandler(repo *repository.Repository, sessions *auth.SessionManager) *EventsHandler {
	return &EventsHandler{repo: repo, sessions: sessions}
}

func (h *EventsHandler) List(w http.ResponseWriter, r *http.Request) {
	upcoming := r.URL.Query().Get("upcoming") == "true"
	past := r.URL.Query().Get("past") == "true"

	events, err := h.repo.ListEvents(r.Context(), upcoming, past)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list events")
		return
	}
	if events == nil {
		events = []models.EventSummary{}
	}
	writeJSON(w, http.StatusOK, events)
}

func (h *EventsHandler) Next(w http.ResponseWriter, r *http.Request) {
	event, err := h.repo.GetNextEvent(r.Context())
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeJSON(w, http.StatusOK, nil)
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to get next event")
		return
	}
	writeJSON(w, http.StatusOK, event)
}

func (h *EventsHandler) Get(w http.ResponseWriter, r *http.Request) {
	slug := chi.URLParam(r, "slug")
	var userID *uuid.UUID
	if id, ok := h.sessions.GetUserID(r); ok {
		userID = &id
	}

	event, err := h.repo.GetEventBySlug(r.Context(), slug, userID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to get event")
		return
	}
	writeJSON(w, http.StatusOK, event)
}

func (h *EventsHandler) Roster(w http.ResponseWriter, r *http.Request) {
	slug := chi.URLParam(r, "slug")
	event, err := h.repo.GetEventBySlug(r.Context(), slug, nil)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to get event")
		return
	}

	roster, err := h.repo.ListPublicRoster(r.Context(), event.ID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list roster")
		return
	}
	if roster == nil {
		roster = []models.Registration{}
	}
	writeJSON(w, http.StatusOK, roster)
}

func (h *EventsHandler) Register(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	slug := chi.URLParam(r, "slug")
	reg, err := h.repo.RegisterForEvent(r.Context(), slug, user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		if errors.Is(err, repository.ErrSignupsClosed) {
			writeError(w, http.StatusBadRequest, "signups are not open")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to register")
		return
	}
	writeJSON(w, http.StatusCreated, reg)
}

func (h *EventsHandler) CancelRegistration(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	slug := chi.URLParam(r, "slug")
	if err := h.repo.CancelRegistration(r.Context(), slug, user.ID); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "registration not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to cancel")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *EventsHandler) MyRegistrations(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "login required")
		return
	}

	regs, err := h.repo.ListUserRegistrations(r.Context(), user.ID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list registrations")
		return
	}
	if regs == nil {
		regs = []models.Registration{}
	}
	writeJSON(w, http.StatusOK, regs)
}

// Admin handlers

func (h *EventsHandler) AdminList(w http.ResponseWriter, r *http.Request) {
	events, err := h.repo.ListAllEvents(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list events")
		return
	}
	if events == nil {
		events = []models.Event{}
	}
	writeJSON(w, http.StatusOK, events)
}

func (h *EventsHandler) AdminCreate(w http.ResponseWriter, r *http.Request) {
	var input models.CreateEventInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}
	if input.Title == "" || input.Slug == "" {
		writeError(w, http.StatusBadRequest, "title and slug are required")
		return
	}
	if input.Status == "" {
		input.Status = "draft"
	}

	event, err := h.repo.CreateEvent(r.Context(), input)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to create event")
		return
	}
	writeJSON(w, http.StatusCreated, event)
}

func (h *EventsHandler) AdminUpdate(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}

	var input models.UpdateEventInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	event, err := h.repo.UpdateEvent(r.Context(), id, input)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to update event")
		return
	}
	writeJSON(w, http.StatusOK, event)
}

func (h *EventsHandler) AdminDelete(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}
	if err := h.repo.DeleteEvent(r.Context(), id); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "event not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to delete event")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *EventsHandler) AdminListRegistrations(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid event id")
		return
	}

	regs, err := h.repo.ListEventRegistrations(r.Context(), id)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list registrations")
		return
	}
	if regs == nil {
		regs = []models.Registration{}
	}
	writeJSON(w, http.StatusOK, regs)
}

func (h *EventsHandler) AdminUpdateRegistration(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid registration id")
		return
	}

	var input models.UpdateRegistrationInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}
	if input.Status == "" {
		writeError(w, http.StatusBadRequest, "status is required")
		return
	}

	reg, err := h.repo.UpdateRegistrationStatus(r.Context(), id, input.Status)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "registration not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to update registration")
		return
	}
	writeJSON(w, http.StatusOK, reg)
}

func (h *EventsHandler) AdminDeleteRegistration(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid registration id")
		return
	}
	if err := h.repo.DeleteRegistration(r.Context(), id); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "registration not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to delete registration")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
