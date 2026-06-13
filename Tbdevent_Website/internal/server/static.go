package server

import (
	"io/fs"
	"net/http"
	"path"
	"strings"
)

func newStaticHandler(dist fs.FS) http.Handler {
	fileServer := http.FileServer(http.FS(dist))

	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet && r.Method != http.MethodHead {
			http.NotFound(w, r)
			return
		}

		cleanPath := path.Clean(r.URL.Path)
		if cleanPath == "/" {
			serveSPA(w, r, dist)
			return
		}

		// Try to serve a real file (assets, etc.)
		if _, err := fs.Stat(dist, strings.TrimPrefix(cleanPath, "/")); err == nil {
			fileServer.ServeHTTP(w, r)
			return
		}

		serveSPA(w, r, dist)
	})
}

func serveSPA(w http.ResponseWriter, r *http.Request, dist fs.FS) {
	data, err := fs.ReadFile(dist, "index.html")
	if err != nil {
		http.NotFound(w, r)
		return
	}
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(data)
}
