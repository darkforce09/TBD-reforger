package handlers

import (
	"encoding/json"
	"errors"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/tbdevent/website/internal/middleware"
	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type AdminHandler struct {
	repo *repository.Repository
}

func NewAdminHandler(repo *repository.Repository) *AdminHandler {
	return &AdminHandler{repo: repo}
}

func (h *AdminHandler) GetPage(w http.ResponseWriter, r *http.Request) {
	slug := chi.URLParam(r, "slug")
	page, err := h.repo.GetPageBySlug(r.Context(), slug, true)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "page not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to get page")
		return
	}
	writeJSON(w, http.StatusOK, page)
}

func (h *AdminHandler) UpdatePage(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "unauthorized")
		return
	}

	slug := chi.URLParam(r, "slug")
	var input models.UpdatePageInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	page, err := h.repo.UpdatePage(r.Context(), slug, input, user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "page not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to update page")
		return
	}
	writeJSON(w, http.StatusOK, page)
}

func (h *AdminHandler) UpsertSections(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "unauthorized")
		return
	}

	slug := chi.URLParam(r, "slug")
	var sections []models.UpsertSectionInput
	if err := json.NewDecoder(r.Body).Decode(&sections); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	page, err := h.repo.UpsertSections(r.Context(), slug, sections, user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "page not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to save sections")
		return
	}
	writeJSON(w, http.StatusOK, page)
}

func (h *AdminHandler) CreateSection(w http.ResponseWriter, r *http.Request) {
	user, ok := middleware.UserFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "unauthorized")
		return
	}

	slug := chi.URLParam(r, "slug")
	var input models.CreateSectionInput
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}
	if input.SectionKey == "" {
		writeError(w, http.StatusBadRequest, "sectionKey is required")
		return
	}

	section, err := h.repo.CreateSection(r.Context(), slug, input, user.ID)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "page not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to create section")
		return
	}
	writeJSON(w, http.StatusCreated, section)
}

func (h *AdminHandler) DeleteSection(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid section id")
		return
	}

	if err := h.repo.DeleteSection(r.Context(), id); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeError(w, http.StatusNotFound, "section not found")
			return
		}
		writeError(w, http.StatusInternalServerError, "failed to delete section")
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
