package handlers

import (
	"encoding/json"
	"errors"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type AnnouncementsHandler struct {
	repo *repository.Repository
}

func NewAnnouncementsHandler(repo *repository.Repository) *AnnouncementsHandler {
	return &AnnouncementsHandler{repo: repo}
}

func (h *AnnouncementsHandler) List(w http.ResponseWriter, r *http.Request) {
	limit := 50
	if l := r.URL.Query().Get("limit"); l != "" {
		if n, err := strconv.Atoi(l); err == nil {
			limit = n
		}
	}

	items, err := h.repo.ListAnnouncements(r.Context(), limit)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list announcements")
		return
	}
	if items == nil {
		items = []models.Announcement{}
	}
	writeJSON(w, http.StatusOK, items)
}

func (h *AnnouncementsHandler) AdminList(w http.ResponseWriter, r *http.Request) {
	items, err := h.repo.ListAllAnnouncements(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list announcements")
		return
	}
	if items == nil {
		items = []models.Announcement{}
	}
	writeJSON(w, http.StatusOK, items)
}

func (h *AnnouncementsHandler) AdminCreate(w http.ResponseWriter, r *http.Request) {
	var input models.CreateAnnouncementInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}
	if input.Title == "" {
		writeError(w, http.StatusBadRequest, "title is required")
		return
	}

	item, err := h.repo.CreateAnnouncement(r.Context(), input)
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to create announcement")
		return
	}
	writeJSON(w, http.StatusCreated, item)
}

func (h *AnnouncementsHandler) AdminUpdate(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid id")
		return
	}

	var input models.UpdateAnnouncementInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	item, err := h.repo.UpdateAnnouncement(r.Context(), id, input)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "announcement not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to update announcement")
		return
	}
	writeJSON(w, http.StatusOK, item)
}

func (h *AnnouncementsHandler) AdminDelete(w http.ResponseWriter, r *http.Request) {
	id, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid id")
		return
	}
	if err := h.repo.DeleteAnnouncement(r.Context(), id); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "announcement not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to delete announcement")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
