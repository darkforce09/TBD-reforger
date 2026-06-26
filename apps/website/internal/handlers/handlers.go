// Package handlers implements the HTTP layer, grouped by domain. Each milestone
// registers its routes through Handler.Register.
package handlers

import (
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
	"gorm.io/gorm"

	"github.com/tbd-milsim/reforger-backend/internal/auth"
	"github.com/tbd-milsim/reforger-backend/internal/config"
	"github.com/tbd-milsim/reforger-backend/internal/middleware"
	"github.com/tbd-milsim/reforger-backend/internal/realtime"
	"github.com/tbd-milsim/reforger-backend/internal/services"
)

// refreshTTL is how long an opaque refresh token remains valid.
const refreshTTL = 30 * 24 * time.Hour

// linkCodeTTL is how long a 6-digit Arma link code remains valid.
const linkCodeTTL = 10 * time.Minute

// Handler bundles dependencies shared by all HTTP handlers.
type Handler struct {
	db      *gorm.DB
	cfg     *config.Config
	jwt     *auth.Manager
	discord *services.DiscordService
	webhook *services.WebhookService
	hub     *realtime.Hub
}

// New constructs a Handler and its sub-services from config.
func New(db *gorm.DB, cfg *config.Config) *Handler {
	return &Handler{
		db:  db,
		cfg: cfg,
		jwt: auth.NewManager(cfg.JWTSecret, cfg.JWTAccessTTLMin),
		discord: services.NewDiscordService(
			cfg.DiscordClientID,
			cfg.DiscordClientSecret,
			cfg.DiscordRedirectURL,
			cfg.DiscordGuildID,
		),
		webhook: services.NewWebhookService(cfg.DiscordWebhookURL),
		hub:     realtime.NewHub(),
	}
}

// JWT exposes the token manager (used by tests to mint tokens).
func (h *Handler) JWT() *auth.Manager { return h.jwt }

// Discord exposes the Discord service (used by tests to point at a mock server).
func (h *Handler) Discord() *services.DiscordService { return h.discord }

// Webhook exposes the webhook service (used by tests to point at a mock server).
func (h *Handler) Webhook() *services.WebhookService { return h.webhook }

// Register wires all identity routes onto the /api/v1 group. Later milestones
// add their own groups here.
func (h *Handler) Register(rg *gin.RouterGroup) {
	// Public auth flow.
	rg.GET("/auth/discord/login", h.DiscordLogin)
	rg.GET("/auth/discord/callback", h.DiscordCallback)
	rg.POST("/auth/refresh", h.Refresh)
	rg.POST("/auth/logout", h.Logout)

	// Development-only login shortcut (mints a session without Discord). The
	// handler also guards on env, so this is a no-op surface in production.
	if h.cfg.Env == "development" {
		rg.GET("/auth/dev-login", h.DevLogin)
	}

	// Authenticated self-service + content read paths (members-only).
	authed := rg.Group("", middleware.RequireAuth(h.jwt))
	authed.GET("/me", h.GetMe)
	authed.PATCH("/me", h.UpdateMe)
	authed.POST("/me/link", h.CreateLinkCode)
	authed.GET("/me/link/status", h.LinkStatus)
	authed.DELETE("/me/link", h.Unlink)

	// Content read paths (M4).
	authed.GET("/announcements", h.ListAnnouncements)
	authed.GET("/announcements/:id", h.GetAnnouncement)
	authed.GET("/dashboard", h.GetDashboard)
	authed.GET("/wiki", h.ListWiki)
	authed.GET("/wiki/:slug", h.GetWikiPage)
	authed.GET("/vehicle-database", h.ListVehicles)
	authed.GET("/modpacks", h.ListModpacks)
	authed.GET("/modpacks/current", h.GetCurrentModpack)
	authed.GET("/servers", h.ListServers)
	authed.GET("/servers/:id/status", h.GetServerStatus)

	// Telemetry reads (M7): leaderboards, individual stats, live SSE feed.
	authed.GET("/servers/:id/status/stream", h.StreamServerStatus)
	authed.GET("/leaderboards", h.GetLeaderboards)
	authed.GET("/users/:discordId/stats", h.GetUserStats)

	// Missions (M5): library reads + per-mission actions (author/admin checks
	// happen inside the handlers since ownership is row-dependent).
	authed.GET("/missions", h.ListMissions)
	authed.GET("/missions/:id", h.GetMission)
	authed.PATCH("/missions/:id", h.UpdateMission)
	authed.POST("/missions/:id/submit", h.SubmitMission)
	// Mission versions carry the full compiled payload (hundreds of MB at scale), so
	// this route gets its own high body cap; the global limiter skips it (see middleware.BodyLimit).
	authed.POST("/missions/:id/versions", middleware.BodyLimit(h.cfg.MissionVersionBodyLimit()), h.CreateVersion)
	authed.GET("/missions/:id/versions/:vid", h.GetVersion)
	authed.GET("/missions/:id/armory", h.GetArmory)
	authed.PUT("/missions/:id/armory", h.SetArmory)
	authed.POST("/missions/:id/bookmark", h.BookmarkMission)
	authed.DELETE("/missions/:id/bookmark", h.RemoveBookmark)

	// Events & ORBAT (M6): event hub reads, then per-mission ORBAT + registration
	// keyed on event_mission_id (a campaign event holds many missions).
	authed.GET("/events", h.ListEvents)
	authed.GET("/events/:id", h.GetEvent)
	authed.GET("/event-missions/:emid/orbat", h.GetOrbat)
	authed.POST("/event-missions/:emid/register", h.RegisterForEventMission)
	authed.DELETE("/event-missions/:emid/register", h.WithdrawFromEventMission)
	authed.GET("/me/deployments", h.GetMyDeployments)
	authed.POST("/me/leave-requests", h.SubmitLeave)
	authed.GET("/me/leave-requests", h.ListMyLeave)

	// Field tools (M9): mortar calculator + saved fire missions.
	authed.POST("/fire-missions/solve", h.SolveFire)
	authed.POST("/fire-missions", h.SaveFire)
	authed.GET("/events/:id/fire-missions", h.ListEventFireMissions)

	// Squad leadership (M6): reserve/hold a squad, fill it, and look up members.
	// Available to leader and above (mission_maker/admin inherit).
	leader := rg.Group("", middleware.RequireAuth(h.jwt), middleware.RequireMinRole("leader"))
	leader.POST("/event-missions/:emid/squads/reserve", h.ReserveSquad)
	leader.POST("/event-missions/:emid/squads/release", h.ReleaseSquad)
	leader.PUT("/event-missions/:emid/slots/:slotId/assign", h.AssignSlot)
	leader.DELETE("/event-missions/:emid/slots/:slotId/assign", h.ClearSlot)
	leader.GET("/members", h.SearchMembers)

	// Mission authoring + export require at least mission_maker.
	mm := rg.Group("", middleware.RequireAuth(h.jwt), middleware.RequireMinRole("mission_maker"))
	mm.POST("/missions", h.CreateMission)
	mm.GET("/missions/:id/export", h.ExportMission)
	mm.POST("/missions/:id/inject", h.InjectMission)

	// Admin CMS + wiki authoring (M4) + mission approvals (M5).
	admin := rg.Group("", middleware.RequireAuth(h.jwt), middleware.RequireMinRole("admin"))
	admin.POST("/cms/announcements", h.CreateAnnouncement)
	admin.PATCH("/cms/announcements/:id", h.UpdateAnnouncement)
	admin.DELETE("/cms/announcements/:id", h.DeleteAnnouncement)
	admin.POST("/cms/announcements/:id/push-discord", h.PushAnnouncementDiscord)
	admin.POST("/cms/uploads", h.UploadImage)
	admin.PUT("/wiki/:slug", h.UpsertWikiPage)
	admin.GET("/approvals", h.ListApprovals)
	admin.POST("/approvals/:id/approve", h.ApproveMission)
	admin.POST("/approvals/:id/reject", h.RejectMission)

	// Event management + ORBAT slot assignment + LOA review (M6, admin).
	admin.POST("/events", h.CreateEvent)
	admin.PATCH("/events/:id", h.UpdateEvent)
	admin.DELETE("/events/:id", h.DeleteEvent)
	admin.POST("/events/:id/missions", h.AddEventMission)
	admin.DELETE("/events/:id/missions/:emid", h.RemoveEventMission)
	admin.GET("/admin/leave-requests", h.ListAllLeave)
	admin.PATCH("/admin/leave-requests/:id", h.ReviewLeave)

	// Administration (M8): personnel, bans/warnings, role sync, RCON, audit logs.
	admin.GET("/admin/users", h.ListUsers)
	admin.PATCH("/admin/users/:discordId", h.UpdateUser)
	admin.POST("/admin/users/:discordId/ban", h.BanUser)
	admin.DELETE("/admin/users/:discordId/ban", h.UnbanUser)
	admin.POST("/admin/users/:discordId/warnings", h.IssueWarning)
	admin.POST("/admin/roles/sync", h.ResyncRoles)
	admin.POST("/admin/servers/:id/rcon", h.SendRCON)
	admin.GET("/admin/audit-logs", h.ListAuditLogs)
	admin.GET("/admin/audit-logs/stream", h.StreamAuditLogs)
	admin.GET("/admin/audit-logs/export.csv", h.ExportAuditLogsCSV)

	// Game-server ingest (service-token auth): link confirmation + telemetry (M7).
	ingest := rg.Group("/ingest", middleware.RequireServiceToken(h.cfg.ServiceToken))
	ingest.POST("/link-confirm", h.IngestLinkConfirm)
	ingest.POST("/server-status", h.IngestServerStatus)
	ingest.POST("/match-results", h.IngestMatchResults)
}

// uploadDir is the local directory for CMS thumbnail uploads, served at /uploads.
// TODO: swap for S3/MinIO in production.
const uploadDir = "uploads"

// parsePage reads ?limit= and ?offset= with sane bounds for list endpoints.
func parsePage(c *gin.Context) (limit, offset int) {
	limit, offset = 20, 0
	if v := c.Query("limit"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 100 {
			limit = n
		}
	}
	if v := c.Query("offset"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n >= 0 {
			offset = n
		}
	}
	return limit, offset
}
