package config

import (
	"crypto/subtle"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type Config struct {
	Port       string
	BaseURL    string
	Env        string
	Database   DatabaseConfig
	Session    SessionConfig
	Discord    DiscordConfig
	GameServer GameServerConfig
	SchemaDir  string
}

// GameServerConfig controls the server-token authed API that dedicated game
// servers use to fetch missions and post results/telemetry (Phase 0.1 spike).
type GameServerConfig struct {
	Tokens      []string
	MissionsDir string
}

type DatabaseConfig struct {
	URL string
}

type SessionConfig struct {
	Secret string
}

type DiscordConfig struct {
	ClientID        string
	ClientSecret    string
	RedirectURI     string
	GuildID         string
	AdminRoleID     string
	AdminDiscordIDs []string
}

func Load() (*Config, error) {
	cfg := &Config{
		Port:    getEnv("PORT", "8080"),
		BaseURL: getEnv("BASE_URL", "http://localhost:8080"),
		Env:     getEnv("ENV", "development"),
		Database: DatabaseConfig{
			URL: databaseURL(),
		},
		Session: SessionConfig{
			Secret: getEnv("SESSION_SECRET", "dev-secret-change-me"),
		},
		Discord: DiscordConfig{
			ClientID:        os.Getenv("DISCORD_CLIENT_ID"),
			ClientSecret:    os.Getenv("DISCORD_CLIENT_SECRET"),
			RedirectURI:     os.Getenv("DISCORD_REDIRECT_URI"),
			GuildID:         os.Getenv("DISCORD_GUILD_ID"),
			AdminRoleID:     os.Getenv("ADMIN_DISCORD_ROLE_ID"),
			AdminDiscordIDs: parseCSV(os.Getenv("ADMIN_DISCORD_IDS")),
		},
		GameServer: GameServerConfig{
			Tokens:      parseCSV(os.Getenv("GAME_SERVER_TOKENS")),
			MissionsDir: getEnv("MISSIONS_DIR", "missions"),
		},
		SchemaDir: getEnv("SCHEMA_DIR", "../tbd-schema"),
	}

	if cfg.Database.URL == "" {
		return nil, fmt.Errorf("DATABASE_URL or POSTGRES_* variables are required")
	}

	if cfg.Env == "production" && cfg.Session.Secret == "dev-secret-change-me" {
		return nil, fmt.Errorf("SESSION_SECRET must be set in production")
	}

	return cfg, nil
}

func (c *Config) IsProduction() bool {
	return c.Env == "production"
}

func databaseURL() string {
	if url := os.Getenv("DATABASE_URL"); url != "" {
		return url
	}

	host := getEnv("POSTGRES_HOST", "localhost")
	port := getEnv("POSTGRES_PORT", "5432")
	user := getEnv("POSTGRES_USER", "tbdevent")
	password := getEnv("POSTGRES_PASSWORD", "tbdevent")
	db := getEnv("POSTGRES_DB", "tbdevent")

	return fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable", user, password, host, port, db)
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func parseCSV(s string) []string {
	if s == "" {
		return nil
	}
	parts := strings.Split(s, ",")
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p != "" {
			out = append(out, p)
		}
	}
	return out
}

func (d DiscordConfig) OAuthConfigured() bool {
	return d.ClientID != "" && d.ClientSecret != "" && d.RedirectURI != ""
}

func (d DiscordConfig) IsAdminByID(discordID string) bool {
	for _, id := range d.AdminDiscordIDs {
		if id == discordID {
			return true
		}
	}
	return false
}

func (d DiscordConfig) RoleCheckConfigured() bool {
	return d.GuildID != "" && d.AdminRoleID != ""
}

// HasToken reports whether t is one of the configured game-server tokens.
func (g GameServerConfig) HasToken(t string) bool {
	for _, token := range g.Tokens {
		if subtle.ConstantTimeCompare([]byte(token), []byte(t)) == 1 {
			return true
		}
	}
	return false
}

func GetPortInt(port string) (int, error) {
	return strconv.Atoi(port)
}

// Env-only helpers used by the restspike command, which must run without a
// database (and therefore without a full config.Load).

func GameServerTokensFromEnv() []string {
	return parseCSV(os.Getenv("GAME_SERVER_TOKENS"))
}

func MissionsDirFromEnv() string {
	return getEnv("MISSIONS_DIR", "missions")
}

func PortFromEnv() string {
	return getEnv("PORT", "8080")
}
