package auth

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/hex"
	"math/big"
)

// RandomToken returns a cryptographically random hex string of nBytes entropy.
// Used for OAuth state and opaque refresh tokens.
func RandomToken(nBytes int) (string, error) {
	b := make([]byte, nBytes)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

// HashToken returns the hex SHA-256 of a token. Refresh tokens are stored hashed
// so a database leak does not expose usable credentials.
func HashToken(token string) string {
	sum := sha256.Sum256([]byte(token))
	return hex.EncodeToString(sum[:])
}

// ConstantTimeEqual compares two strings without leaking timing information.
func ConstantTimeEqual(a, b string) bool {
	return subtle.ConstantTimeCompare([]byte(a), []byte(b)) == 1
}

// NumericCode returns a zero-padded random decimal code of the given length,
// e.g. NumericCode(6) -> "042199". Backs the Arma identity link code.
func NumericCode(digits int) (string, error) {
	upper := big.NewInt(1)
	ten := big.NewInt(10)
	for i := 0; i < digits; i++ {
		upper.Mul(upper, ten)
	}
	n, err := rand.Int(rand.Reader, upper)
	if err != nil {
		return "", err
	}
	s := n.String()
	for len(s) < digits {
		s = "0" + s
	}
	return s, nil
}
