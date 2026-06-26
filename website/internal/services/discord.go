// Package services holds cross-cutting integrations: Discord OAuth2/role sync,
// audit logging, and (in later milestones) webhook push and leaderboard refresh.
package services

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// DefaultDiscordAPI is the production Discord API base. Overridable for tests.
const DefaultDiscordAPI = "https://discord.com/api"

// oauthScopes: identify gets the profile; guilds.members.read gets the caller's
// roles within our guild so the backend can map them to web permissions.
const oauthScopes = "identify guilds.members.read"

// DiscordService is a thin client for the OAuth2 + member-roles endpoints.
type DiscordService struct {
	clientID     string
	clientSecret string
	redirectURL  string
	guildID      string
	apiBase      string
	http         *http.Client
}

// NewDiscordService constructs the client with production defaults.
func NewDiscordService(clientID, clientSecret, redirectURL, guildID string) *DiscordService {
	return &DiscordService{
		clientID:     clientID,
		clientSecret: clientSecret,
		redirectURL:  redirectURL,
		guildID:      guildID,
		apiBase:      DefaultDiscordAPI,
		http:         &http.Client{Timeout: 10 * time.Second},
	}
}

// SetAPIBase overrides the Discord API base URL (used by tests with httptest).
func (d *DiscordService) SetAPIBase(base string) { d.apiBase = strings.TrimRight(base, "/") }

// AuthorizeURL builds the consent URL the browser is redirected to.
func (d *DiscordService) AuthorizeURL(state string) string {
	q := url.Values{}
	q.Set("client_id", d.clientID)
	q.Set("redirect_uri", d.redirectURL)
	q.Set("response_type", "code")
	q.Set("scope", oauthScopes)
	q.Set("state", state)
	return d.apiBase + "/oauth2/authorize?" + q.Encode()
}

// TokenResponse is the OAuth2 token-exchange payload.
type TokenResponse struct {
	AccessToken  string `json:"access_token"`
	TokenType    string `json:"token_type"`
	ExpiresIn    int    `json:"expires_in"`
	RefreshToken string `json:"refresh_token"`
	Scope        string `json:"scope"`
}

// ExchangeCode swaps an authorization code for an access token.
func (d *DiscordService) ExchangeCode(ctx context.Context, code string) (*TokenResponse, error) {
	form := url.Values{}
	form.Set("client_id", d.clientID)
	form.Set("client_secret", d.clientSecret)
	form.Set("grant_type", "authorization_code")
	form.Set("code", code)
	form.Set("redirect_uri", d.redirectURL)

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, d.apiBase+"/oauth2/token", strings.NewReader(form.Encode()))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	var out TokenResponse
	if err := d.do(req, &out); err != nil {
		return nil, err
	}
	if out.AccessToken == "" {
		return nil, fmt.Errorf("discord: empty access token")
	}
	return &out, nil
}

// DiscordUser is the subset of /users/@me we use.
type DiscordUser struct {
	ID            string `json:"id"`
	Username      string `json:"username"`
	GlobalName    string `json:"global_name"`
	Discriminator string `json:"discriminator"`
	Avatar        string `json:"avatar"`
}

// DisplayName prefers the new global display name, falling back to username.
func (u *DiscordUser) DisplayName() string {
	if u.GlobalName != "" {
		return u.GlobalName
	}
	return u.Username
}

// Handle renders the classic "name#1234" handle, or just the username for users
// migrated to the new unique-username system (discriminator "0").
func (u *DiscordUser) Handle() string {
	if u.Discriminator == "" || u.Discriminator == "0" {
		return u.Username
	}
	return u.Username + "#" + u.Discriminator
}

// AvatarURL builds the CDN avatar URL, or "" if the user has no custom avatar.
func (u *DiscordUser) AvatarURL() string {
	if u.Avatar == "" {
		return ""
	}
	return fmt.Sprintf("https://cdn.discordapp.com/avatars/%s/%s.png", u.ID, u.Avatar)
}

// FetchUser retrieves the authenticated user's profile.
func (d *DiscordService) FetchUser(ctx context.Context, accessToken string) (*DiscordUser, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, d.apiBase+"/users/@me", nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	var out DiscordUser
	if err := d.do(req, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// GuildMember is the subset of the member object we use (role snowflakes + nick).
type GuildMember struct {
	Nick  string   `json:"nick"`
	Roles []string `json:"roles"`
}

// FetchGuildMember retrieves the caller's membership (and roles) in our guild.
// Returns a nil member with no error if the user is not in the guild (404),
// so login still succeeds for users who haven't joined yet.
func (d *DiscordService) FetchGuildMember(ctx context.Context, accessToken string) (*GuildMember, error) {
	endpoint := fmt.Sprintf("%s/users/@me/guilds/%s/member", d.apiBase, d.guildID)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)

	resp, err := d.http.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode == http.StatusNotFound {
		return nil, nil // not a guild member
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<12))
		return nil, fmt.Errorf("discord member fetch: status %d: %s", resp.StatusCode, string(body))
	}
	var out GuildMember
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	return &out, nil
}

// do executes a request expecting a 2xx JSON response decoded into v.
func (d *DiscordService) do(req *http.Request, v any) error {
	resp, err := d.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<12))
		return fmt.Errorf("discord: status %d: %s", resp.StatusCode, string(body))
	}
	return json.NewDecoder(resp.Body).Decode(v)
}
