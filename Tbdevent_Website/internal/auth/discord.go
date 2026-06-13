package auth

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	"golang.org/x/oauth2"

	"github.com/tbdevent/website/internal/config"
)

const (
	discordAuthURL  = "https://discord.com/api/oauth2/authorize"
	discordTokenURL = "https://discord.com/api/oauth2/token"
	discordAPIBase  = "https://discord.com/api"
)

type DiscordService struct {
	cfg  *config.Config
	oauth *oauth2.Config
}

type DiscordUser struct {
	ID            string `json:"id"`
	Username      string `json:"username"`
	GlobalName    string `json:"global_name"`
	Avatar        string `json:"avatar"`
	Discriminator string `json:"discriminator"`
}

type GuildMember struct {
	Roles []string `json:"roles"`
}

func NewDiscordService(cfg *config.Config) *DiscordService {
	return &DiscordService{
		cfg: cfg,
		oauth: &oauth2.Config{
			ClientID:     cfg.Discord.ClientID,
			ClientSecret: cfg.Discord.ClientSecret,
			RedirectURL:  cfg.Discord.RedirectURI,
			Scopes:       []string{"identify", "guilds.members.read"},
			Endpoint: oauth2.Endpoint{
				AuthURL:  discordAuthURL,
				TokenURL: discordTokenURL,
			},
		},
	}
}

func (d *DiscordService) AuthCodeURL(state string) string {
	return d.oauth.AuthCodeURL(state, oauth2.AccessTypeOnline)
}

func (d *DiscordService) Exchange(ctx context.Context, code string) (*oauth2.Token, error) {
	return d.oauth.Exchange(ctx, code)
}

func (d *DiscordService) FetchUser(ctx context.Context, token *oauth2.Token) (*DiscordUser, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, discordAPIBase+"/users/@me", nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+token.AccessToken)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("fetch discord user: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("discord user api status %d: %s", resp.StatusCode, string(body))
	}

	var user DiscordUser
	if err := json.NewDecoder(resp.Body).Decode(&user); err != nil {
		return nil, fmt.Errorf("decode discord user: %w", err)
	}
	return &user, nil
}

func (d *DiscordService) AvatarURL(user *DiscordUser) string {
	if user.Avatar != "" {
		return fmt.Sprintf("https://cdn.discordapp.com/avatars/%s/%s.png", user.ID, user.Avatar)
	}
	disc := user.Discriminator
	if disc == "0" {
		disc = "0"
	}
	index := 0
	if len(user.ID) > 0 {
		var n int
		fmt.Sscanf(user.ID, "%d", &n)
		index = n % 5
	}
	return fmt.Sprintf("https://cdn.discordapp.com/embed/avatars/%d.png", index)
}

func (d *DiscordService) DisplayName(user *DiscordUser) string {
	if user.GlobalName != "" {
		return user.GlobalName
	}
	return user.Username
}

func (d *DiscordService) IsAdmin(ctx context.Context, discordID string, token *oauth2.Token) bool {
	if d.cfg.Discord.IsAdminByID(discordID) {
		return true
	}

	if !d.cfg.Discord.RoleCheckConfigured() || token == nil {
		return false
	}

	url := fmt.Sprintf("%s/users/@me/guilds/%s/member", discordAPIBase, d.cfg.Discord.GuildID)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return false
	}
	req.Header.Set("Authorization", "Bearer "+token.AccessToken)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return false
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return false
	}

	var member GuildMember
	if err := json.NewDecoder(resp.Body).Decode(&member); err != nil {
		return false
	}

	for _, role := range member.Roles {
		if role == d.cfg.Discord.AdminRoleID {
			return true
		}
	}

	return false
}
