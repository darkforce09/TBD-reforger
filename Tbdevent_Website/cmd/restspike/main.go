// Command restspike runs the game-server API (missions/results/telemetry) in
// isolation, without Postgres, for the Phase 0.1 REST spike. It reuses the exact
// same handlers and server-token middleware as the production server so the spike
// exercises the real contract a dedicated Reforger server will hit.
//
// Usage:
//
//	GAME_SERVER_TOKENS=spike-token MISSIONS_DIR=missions PORT=8099 go run ./cmd/restspike
package main

import (
	"log"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"

	"github.com/tbdevent/website/internal/config"
	"github.com/tbdevent/website/internal/handlers"
	appmiddleware "github.com/tbdevent/website/internal/middleware"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		// The spike does not need a database; ignore DB-related load errors by
		// reading the game-server settings directly if Load fails on DATABASE_URL.
		log.Printf("config.Load: %v (continuing with game-server env only)", err)
	}

	var gsCfg config.GameServerConfig
	if cfg != nil {
		gsCfg = cfg.GameServer
	} else {
		gsCfg = config.GameServerConfig{
			Tokens:      config.GameServerTokensFromEnv(),
			MissionsDir: config.MissionsDirFromEnv(),
		}
	}

	if len(gsCfg.Tokens) == 0 {
		log.Fatal("GAME_SERVER_TOKENS must be set for the spike")
	}

	h := handlers.NewGameServerHandler(nil, gsCfg.MissionsDir)
	mw := appmiddleware.NewServerTokenMiddleware(gsCfg)

	r := chi.NewRouter()
	r.Use(middleware.Logger)
	r.Route("/api", func(api chi.Router) {
		api.Group(func(gs chi.Router) {
			gs.Use(mw.RequireServerToken)
			gs.Get("/missions/{id}/compiled", h.MissionCompiled)
			gs.Post("/results", h.PostResults)
			gs.Post("/telemetry", h.PostTelemetry)
		})
	})

	port := config.PortFromEnv()
	addr := ":" + port
	log.Printf("restspike listening on %s (missions dir: %s)", addr, gsCfg.MissionsDir)
	srv := &http.Server{Addr: addr, Handler: r, ReadHeaderTimeout: 10 * time.Second}
	log.Fatal(srv.ListenAndServe())
}
