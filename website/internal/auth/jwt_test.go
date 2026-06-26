package auth

import (
	"testing"
	"time"
)

func TestIssueAndParseRoundTrip(t *testing.T) {
	m := NewManager("test-secret", 15)
	tok, exp, err := m.IssueAccess("123456789", "admin", true)
	if err != nil {
		t.Fatalf("issue: %v", err)
	}
	if exp.Before(time.Now()) {
		t.Fatalf("expiry in the past: %v", exp)
	}
	claims, err := m.Parse(tok)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	if claims.Subject != "123456789" {
		t.Errorf("subject = %q, want 123456789", claims.Subject)
	}
	if claims.Role != "admin" {
		t.Errorf("role = %q, want admin", claims.Role)
	}
	if !claims.ArmaLinked {
		t.Errorf("arma_linked = false, want true")
	}
}

func TestParseRejectsWrongSecret(t *testing.T) {
	signer := NewManager("secret-a", 15)
	verifier := NewManager("secret-b", 15)
	tok, _, err := signer.IssueAccess("1", "enlisted", false)
	if err != nil {
		t.Fatalf("issue: %v", err)
	}
	if _, err := verifier.Parse(tok); err == nil {
		t.Fatal("expected parse to fail with wrong secret")
	}
}

func TestParseRejectsExpired(t *testing.T) {
	m := NewManager("test-secret", 15)
	// Force a negative TTL so the token is already expired.
	m.accessTTL = -time.Minute
	tok, _, err := m.IssueAccess("1", "enlisted", false)
	if err != nil {
		t.Fatalf("issue: %v", err)
	}
	if _, err := m.Parse(tok); err == nil {
		t.Fatal("expected parse to fail for expired token")
	}
}

func TestNumericCodeFormat(t *testing.T) {
	for i := 0; i < 100; i++ {
		code, err := NumericCode(6)
		if err != nil {
			t.Fatalf("NumericCode: %v", err)
		}
		if len(code) != 6 {
			t.Fatalf("code %q length = %d, want 6", code, len(code))
		}
		for _, r := range code {
			if r < '0' || r > '9' {
				t.Fatalf("code %q has non-digit %q", code, r)
			}
		}
	}
}

func TestConstantTimeEqual(t *testing.T) {
	if !ConstantTimeEqual("abc", "abc") {
		t.Error("equal strings reported unequal")
	}
	if ConstantTimeEqual("abc", "abd") {
		t.Error("unequal strings reported equal")
	}
}
