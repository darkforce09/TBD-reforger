package handlers

import (
	"encoding/json"
	"errors"
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/tbdevent/website/internal/models"
	"github.com/tbdevent/website/internal/repository"
)

type PagesHandler struct {
	repo *repository.Repository
}

func NewPagesHandler(repo *repository.Repository) *PagesHandler {
	return &PagesHandler{repo: repo}
}

func (h *PagesHandler) List(w http.ResponseWriter, r *http.Request) {
	pages, err := h.repo.ListPublishedPages(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to list pages")
		return
	}
	if pages == nil {
		pages = []models.PageSummary{}
	}
	writeJSON(w, http.StatusOK, pages)
}

func (h *PagesHandler) Get(w http.ResponseWriter, r *http.Request) {
	slug := chi.URLParam(r, "slug")
	page, err := h.repo.GetPageBySlug(r.Context(), slug, false)
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

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}
