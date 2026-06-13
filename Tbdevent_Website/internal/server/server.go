package server

import (
	"io/fs"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"

	"github.com/tbdevent/website/internal/auth"
	"github.com/tbdevent/website/internal/handlers"
	appmiddleware "github.com/tbdevent/website/internal/middleware"
)

type Dependencies struct {
	PagesHandler         *handlers.PagesHandler
	AdminHandler         *handlers.AdminHandler
	AuthHandler          *handlers.AuthHandler
	EventsHandler        *handlers.EventsHandler
	AnnouncementsHandler *handlers.AnnouncementsHandler
	GameServerHandler    *handlers.GameServerHandler
	MissionsHandler      *handlers.MissionsHandler
	AuthMW               *appmiddleware.AuthMiddleware
	ServerTokenMW        *appmiddleware.ServerTokenMiddleware
	Sessions             *auth.SessionManager
	StaticFS             fs.FS
}

func NewRouter(deps Dependencies) http.Handler {
	r := chi.NewRouter()

	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)
	r.Use(deps.Sessions.LoadAndSave)
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://localhost:5173", "http://127.0.0.1:5173"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type"},
		AllowCredentials: true,
		MaxAge:           300,
	}))

	r.Route("/api", func(api chi.Router) {
		api.Get("/pages", deps.PagesHandler.List)
		api.Get("/pages/{slug}", deps.PagesHandler.Get)

		api.Get("/events", deps.EventsHandler.List)
		api.Get("/events/next", deps.EventsHandler.Next)
		api.Get("/events/{slug}", deps.EventsHandler.Get)
		api.Get("/events/{slug}/roster", deps.EventsHandler.Roster)

		api.Get("/announcements", deps.AnnouncementsHandler.List)

		api.Route("/auth", func(authRoutes chi.Router) {
			authRoutes.Get("/me", deps.AuthHandler.Me)
			authRoutes.Post("/logout", deps.AuthHandler.Logout)
		})

		// Server-token authed API for dedicated game servers (Phase 0.1+).
		if deps.GameServerHandler != nil && deps.ServerTokenMW != nil {
			api.Group(func(gs chi.Router) {
				gs.Use(deps.ServerTokenMW.RequireServerToken)
				gs.Get("/missions/{id}/compiled", deps.GameServerHandler.MissionCompiled)
				gs.Get("/game/events/{id}/roster", deps.GameServerHandler.GameRoster)
				gs.Post("/link", deps.GameServerHandler.PostLink)
				gs.Post("/results", deps.GameServerHandler.PostResults)
				gs.Post("/telemetry", deps.GameServerHandler.PostTelemetry)
			})
		}

		if deps.MissionsHandler != nil {
			api.Route("/me", func(me chi.Router) {
				me.Use(deps.AuthMW.RequireLogin)
				me.Get("/registrations", deps.EventsHandler.MyRegistrations)
				me.Get("/game-identity", deps.MissionsHandler.MyGameIdentity)
				me.Post("/link", deps.MissionsHandler.LinkGameAccount)
			})

			api.With(deps.AuthMW.RequireLogin, deps.AuthHandler.RequireAdmin).
				Post("/missions", deps.MissionsHandler.Publish)
		} else {
			api.Route("/me", func(me chi.Router) {
				me.Use(deps.AuthMW.RequireLogin)
				me.Get("/registrations", deps.EventsHandler.MyRegistrations)
			})
		}

		api.With(deps.AuthMW.RequireLogin).Post("/events/{slug}/register", deps.EventsHandler.Register)
		api.With(deps.AuthMW.RequireLogin).Delete("/events/{slug}/register", deps.EventsHandler.CancelRegistration)

		api.Route("/admin", func(admin chi.Router) {
			admin.Use(deps.AuthMW.RequireLogin)
			admin.Use(deps.AuthHandler.RequireAdmin)

			admin.Get("/pages/{slug}", deps.AdminHandler.GetPage)
			admin.Put("/pages/{slug}", deps.AdminHandler.UpdatePage)
			admin.Put("/pages/{slug}/sections", deps.AdminHandler.UpsertSections)
			admin.Post("/pages/{slug}/sections", deps.AdminHandler.CreateSection)
			admin.Delete("/sections/{id}", deps.AdminHandler.DeleteSection)

			admin.Get("/events", deps.EventsHandler.AdminList)
			admin.Post("/events", deps.EventsHandler.AdminCreate)
			admin.Put("/events/{id}", deps.EventsHandler.AdminUpdate)
			admin.Delete("/events/{id}", deps.EventsHandler.AdminDelete)
			admin.Get("/events/{id}/registrations", deps.EventsHandler.AdminListRegistrations)

			admin.Get("/announcements", deps.AnnouncementsHandler.AdminList)
			admin.Post("/announcements", deps.AnnouncementsHandler.AdminCreate)
			admin.Put("/announcements/{id}", deps.AnnouncementsHandler.AdminUpdate)
			admin.Delete("/announcements/{id}", deps.AnnouncementsHandler.AdminDelete)

			admin.Put("/registrations/{id}", deps.EventsHandler.AdminUpdateRegistration)
			admin.Delete("/registrations/{id}", deps.EventsHandler.AdminDeleteRegistration)

			if deps.MissionsHandler != nil {
				admin.Get("/events/{id}/slots", deps.MissionsHandler.AdminListSlots)
				admin.Put("/events/{id}/slots/{slotId}", deps.MissionsHandler.AdminAssignSlot)
			}
		})
	})

	r.Get("/auth/discord", deps.AuthHandler.DiscordLogin)
	r.Get("/auth/discord/callback", deps.AuthHandler.DiscordCallback)

	if deps.StaticFS != nil {
		static := newStaticHandler(deps.StaticFS)
		r.Get("/*", static.ServeHTTP)
	}

	return r
}
