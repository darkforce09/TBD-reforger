package services

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// mockDiscord returns an httptest server emulating the Discord endpoints we use.
func mockDiscord(t *testing.T) (*DiscordService, *httptest.Server) {
	t.Helper()
	mux := http.NewServeMux()
	mux.HandleFunc("/oauth2/token", func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseForm(); err != nil || r.PostForm.Get("code") != "good-code" {
			http.Error(w, "bad code", http.StatusBadRequest)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"access_token":"tok-abc","token_type":"Bearer","expires_in":604800,"refresh_token":"r","scope":"identify guilds.members.read"}`))
	})
	mux.HandleFunc("/users/@me", func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer tok-abc" {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"765611","username":"AdminDave","global_name":"Admin Dave","discriminator":"0","avatar":"abc123"}`))
	})
	mux.HandleFunc("/users/@me/guilds/guild-1/member", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"nick":"Dave","roles":["role-admin","role-misc"]}`))
	})

	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)

	d := NewDiscordService("client-1", "secret-1", "http://localhost/cb", "guild-1")
	d.SetAPIBase(srv.URL)
	return d, srv
}

func TestExchangeCode(t *testing.T) {
	d, _ := mockDiscord(t)
	tok, err := d.ExchangeCode(context.Background(), "good-code")
	if err != nil {
		t.Fatalf("ExchangeCode: %v", err)
	}
	if tok.AccessToken != "tok-abc" {
		t.Errorf("access token = %q, want tok-abc", tok.AccessToken)
	}
}

func TestExchangeCodeBadCode(t *testing.T) {
	d, _ := mockDiscord(t)
	if _, err := d.ExchangeCode(context.Background(), "wrong"); err == nil {
		t.Fatal("expected error for bad code")
	}
}

func TestFetchUserAndDerivedFields(t *testing.T) {
	d, _ := mockDiscord(t)
	u, err := d.FetchUser(context.Background(), "tok-abc")
	if err != nil {
		t.Fatalf("FetchUser: %v", err)
	}
	if u.ID != "765611" {
		t.Errorf("id = %q", u.ID)
	}
	if got := u.DisplayName(); got != "Admin Dave" {
		t.Errorf("DisplayName = %q, want Admin Dave", got)
	}
	if got := u.Handle(); got != "AdminDave" { // discriminator "0" => no #suffix
		t.Errorf("Handle = %q, want AdminDave", got)
	}
	if got := u.AvatarURL(); !strings.Contains(got, "765611/abc123.png") {
		t.Errorf("AvatarURL = %q", got)
	}
}

func TestFetchGuildMemberRoles(t *testing.T) {
	d, _ := mockDiscord(t)
	m, err := d.FetchGuildMember(context.Background(), "tok-abc")
	if err != nil {
		t.Fatalf("FetchGuildMember: %v", err)
	}
	if m == nil || len(m.Roles) != 2 || m.Roles[0] != "role-admin" {
		t.Fatalf("unexpected member roles: %+v", m)
	}
}

func TestAuthorizeURLContainsParams(t *testing.T) {
	d, _ := mockDiscord(t)
	u := d.AuthorizeURL("state-xyz")
	for _, want := range []string{"client_id=client-1", "state=state-xyz", "response_type=code", "guilds.members.read"} {
		if !strings.Contains(u, want) {
			t.Errorf("authorize url missing %q: %s", want, u)
		}
	}
}
