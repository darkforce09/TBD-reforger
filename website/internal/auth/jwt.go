// Package auth handles JWT issuance/verification and token helpers used by the
// identity flow (Discord OAuth2 + the 6-digit Arma link codes).
package auth

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

const issuer = "tbd-reforger"

// Claims is the access-token payload: subject is the Discord ID, plus the cached
// web role and whether an Arma identity is linked (so the frontend TopBar can
// render the "Linked" pill without an extra round-trip).
type Claims struct {
	Role       string `json:"role"`
	ArmaLinked bool   `json:"arma_linked"`
	jwt.RegisteredClaims
}

// Manager signs and verifies HS256 access tokens.
type Manager struct {
	secret    []byte
	accessTTL time.Duration
}

// NewManager builds a Manager with the given secret and access-token TTL (minutes).
func NewManager(secret string, accessTTLMin int) *Manager {
	if accessTTLMin <= 0 {
		accessTTLMin = 15
	}
	return &Manager{
		secret:    []byte(secret),
		accessTTL: time.Duration(accessTTLMin) * time.Minute,
	}
}

// IssueAccess mints a signed access token and returns it with its expiry.
func (m *Manager) IssueAccess(discordID, role string, armaLinked bool) (string, time.Time, error) {
	now := time.Now()
	exp := now.Add(m.accessTTL)
	claims := Claims{
		Role:       role,
		ArmaLinked: armaLinked,
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   discordID,
			Issuer:    issuer,
			IssuedAt:  jwt.NewNumericDate(now),
			ExpiresAt: jwt.NewNumericDate(exp),
		},
	}
	signed, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString(m.secret)
	if err != nil {
		return "", time.Time{}, err
	}
	return signed, exp, nil
}

// Parse verifies a token's signature and expiry and returns its claims.
func (m *Manager) Parse(tokenStr string) (*Claims, error) {
	claims := &Claims{}
	tok, err := jwt.ParseWithClaims(tokenStr, claims, func(t *jwt.Token) (any, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, errors.New("unexpected signing method")
		}
		return m.secret, nil
	})
	if err != nil {
		return nil, err
	}
	if !tok.Valid {
		return nil, errors.New("invalid token")
	}
	return claims, nil
}
