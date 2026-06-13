package main

import (
	"context"
	"fmt"
	"io/fs"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	_ "github.com/jackc/pgx/v5/stdlib"

	website "github.com/tbdevent/website"
	"github.com/tbdevent/website/internal/auth"
	"github.com/tbdevent/website/internal/config"
	"github.com/tbdevent/website/internal/db"
	"github.com/tbdevent/website/internal/handlers"
	appmiddleware "github.com/tbdevent/website/internal/middleware"
	"github.com/tbdevent/website/internal/migrate"
	"github.com/tbdevent/website/internal/repository"
	"github.com/tbdevent/website/internal/server"
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}

	ctx := context.Background()

	if err := migrate.Up(ctx, cfg.Database.URL); err != nil {
		return fmt.Errorf("migrations: %w", err)
	}

	pool, err := db.Connect(ctx, cfg.Database.URL)
	if err != nil {
		return err
	}
	defer pool.Close()

	repo := repository.New(pool)
	discord := auth.NewDiscordService(cfg)
	sessions := auth.NewSessionManager(cfg)
	authHandler := handlers.NewAuthHandler(cfg, discord, sessions, repo)
	authMW := appmiddleware.NewAuthMiddleware(sessions, repo)

	staticFS, err := fs.Sub(website.WebDist, "web/dist")
	if err != nil {
		log.Printf("warning: static assets not embedded (%v); API-only mode", err)
		staticFS = nil
	}

	router := server.NewRouter(server.Dependencies{
		PagesHandler:         handlers.NewPagesHandler(repo),
		AdminHandler:         handlers.NewAdminHandler(repo),
		AuthHandler:          authHandler,
		EventsHandler:        handlers.NewEventsHandler(repo, sessions),
		AnnouncementsHandler: handlers.NewAnnouncementsHandler(repo),
		GameServerHandler:    handlers.NewGameServerHandler(repo, cfg.GameServer.MissionsDir),
		MissionsHandler:      handlers.NewMissionsHandler(repo, cfg.SchemaDir, cfg.GameServer.MissionsDir),
		AuthMW:               authMW,
		ServerTokenMW:        appmiddleware.NewServerTokenMiddleware(cfg.GameServer),
		Sessions:             sessions,
		StaticFS:             staticFS,
	})

	addr := ":" + cfg.Port
	httpServer := &http.Server{
		Addr:              addr,
		Handler:           router,
		ReadHeaderTimeout: 10 * time.Second,
	}

	errCh := make(chan error, 1)
	go func() {
		log.Printf("listening on %s", addr)
		errCh <- httpServer.ListenAndServe()
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-errCh:
		if err != nil && err != http.ErrServerClosed {
			return err
		}
	case <-stop:
	}

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	return httpServer.Shutdown(shutdownCtx)
}
