// Command api is the entrypoint for the TBD Reforger Platform backend.
package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"golang.org/x/time/rate"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/config"
	"github.com/tbd-milsim/reforger-backend/internal/db"
	"github.com/tbd-milsim/reforger-backend/internal/handlers"
	"github.com/tbd-milsim/reforger-backend/internal/middleware"
)

const (
	maxMultipart  = 8 << 20 // in-memory multipart buffer
	shutdownGrace = 10 * time.Second
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("config: %v", err)
	}

	gdb, err := db.Open(cfg.DatabaseURL, cfg.Env == "development")
	if err != nil {
		log.Fatalf("database: %v", err)
	}
	if err := db.Migrate(gdb); err != nil {
		log.Fatalf("migrate: %v", err)
	}
	log.Println("database connected and migrated")

	if cfg.Env != "development" {
		gin.SetMode(gin.ReleaseMode)
	}
	r := gin.New()
	r.MaxMultipartMemory = maxMultipart

	// Cross-cutting middleware chain (outermost first).
	globalLimiter := middleware.NewIPLimiter(rate.Limit(20), 40) // ~20 req/s/IP
	authLimiter := middleware.NewIPLimiter(rate.Limit(1), 10)    // tighter on auth/ingest
	r.Use(
		middleware.RequestID(),
		middleware.Logger(),
		gin.Recovery(),
		middleware.CORS(cfg.AllowedOrigins),
		// 1 MB cap on all JSON routes (multipart bumped); the mission-version POST is
		// skipped here and wrapped at its own higher cap where it's registered.
		middleware.GlobalBodyLimit(middleware.MaxJSONBody),
		middleware.RateLimit(globalLimiter, authLimiter, []string{"/auth/", "/ingest/"}),
	)

	// Liveness/readiness.
	r.GET("/healthz", func(c *gin.Context) {
		sqlDB, err := gdb.DB()
		if err == nil {
			err = sqlDB.Ping()
		}
		if err != nil {
			c.JSON(http.StatusServiceUnavailable, gin.H{"status": "down", "error": err.Error()})
			return
		}
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})

	// Static serving for CMS thumbnail uploads (local-disk for now).
	r.Static("/uploads", "uploads")

	// Versioned API surface.
	v1 := r.Group("/api/v1")
	registerRoutes(v1, gdb, cfg)

	// Serve with graceful shutdown so in-flight requests and SSE streams drain.
	srv := &http.Server{Addr: ":" + cfg.Port, Handler: r}
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	go func() {
		log.Printf("listening on :%s (env=%s)", cfg.Port, cfg.Env)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("server: %v", err)
		}
	}()

	<-ctx.Done()
	log.Println("shutting down...")
	shutCtx, cancel := context.WithTimeout(context.Background(), shutdownGrace)
	defer cancel()
	if err := srv.Shutdown(shutCtx); err != nil {
		log.Printf("graceful shutdown failed: %v", err)
	}
	log.Println("stopped")
}

// registerRoutes wires handler groups onto the /api/v1 router group.
func registerRoutes(rg *gin.RouterGroup, gdb *gorm.DB, cfg *config.Config) {
	handlers.New(gdb, cfg).Register(rg)
}
