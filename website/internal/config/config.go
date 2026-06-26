// Package config loads runtime configuration from environment variables.
package config

import (
	"fmt"
	"os"
	"strings"

	"github.com/joho/godotenv"
)

// Config holds all runtime settings for the API.
type Config struct {
	// Server
	Port string
	Env  string // "development" | "production"

	// Frontend integration
	FrontendURL    string   // SPA base URL for the OAuth redirect, e.g. http://localhost:5173
	AllowedOrigins []string // CORS allow-list (production; dev uses the Vite proxy)

	// Database
	DatabaseURL string // postgres DSN, e.g. postgres://user:pass@host:5432/db?sslmode=disable

	// Mission editor
	// Body cap for POST /missions/:id/versions only (compiled missions are large at
	// scale). Default 256 MB; all other JSON routes keep the 1 MB middleware default.
	MissionVersionMaxBodyBytes int

	// Auth
	JWTSecret       string
	JWTAccessTTLMin int

	// Discord OAuth2 + role sync
	DiscordClientID     string
	DiscordClientSecret string
	DiscordRedirectURL  string
	DiscordGuildID      string
	DiscordBotToken     string
	DiscordWebhookURL   string

	// Game-server ingest authentication
	ServiceToken string
}

// Load reads configuration from the environment, applying defaults for local
// development. A .env file is loaded if present but is optional.
func Load() (*Config, error) {
	_ = godotenv.Load() // optional; ignore if absent

	frontendURL := getEnv("FRONTEND_URL", "http://localhost:5173")

	cfg := &Config{
		Port:                       getEnv("PORT", "8080"),
		Env:                        getEnv("APP_ENV", "development"),
		FrontendURL:                frontendURL,
		AllowedOrigins:             splitCSV(getEnv("ALLOWED_ORIGINS", frontendURL)),
		DatabaseURL:                os.Getenv("DATABASE_URL"),
		MissionVersionMaxBodyBytes: getEnvInt("MISSION_VERSION_MAX_BODY_BYTES", 256<<20),
		JWTSecret:                  os.Getenv("JWT_SECRET"),
		JWTAccessTTLMin:            getEnvInt("JWT_ACCESS_TTL_MIN", 15),
		DiscordClientID:            os.Getenv("DISCORD_CLIENT_ID"),
		DiscordClientSecret:        os.Getenv("DISCORD_CLIENT_SECRET"),
		DiscordRedirectURL:         os.Getenv("DISCORD_REDIRECT_URL"),
		DiscordGuildID:             os.Getenv("DISCORD_GUILD_ID"),
		DiscordBotToken:            os.Getenv("DISCORD_BOT_TOKEN"),
		DiscordWebhookURL:          os.Getenv("DISCORD_WEBHOOK_URL"),
		ServiceToken:               os.Getenv("SERVICE_TOKEN"),
	}

	if cfg.DatabaseURL == "" {
		return nil, fmt.Errorf("DATABASE_URL is required")
	}
	if cfg.JWTSecret == "" {
		return nil, fmt.Errorf("JWT_SECRET is required")
	}
	return cfg, nil
}

// MissionVersionBodyLimit is the body cap (bytes) for POST /missions/:id/versions,
// falling back to 256 MB when unset — e.g. a Config built directly (tests) rather
// than via Load(). Keeps the large-payload route working without every construction
// site having to remember the field.
func (c *Config) MissionVersionBodyLimit() int64 {
	if c.MissionVersionMaxBodyBytes > 0 {
		return int64(c.MissionVersionMaxBodyBytes)
	}
	return 256 << 20
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// splitCSV parses a comma-separated env value into a trimmed, non-empty slice.
func splitCSV(s string) []string {
	parts := strings.Split(s, ",")
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		if t := strings.TrimSpace(p); t != "" {
			out = append(out, t)
		}
	}
	return out
}

func getEnvInt(key string, fallback int) int {
	if v := os.Getenv(key); v != "" {
		var n int
		if _, err := fmt.Sscanf(v, "%d", &n); err == nil {
			return n
		}
	}
	return fallback
}
