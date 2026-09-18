# Identity & Access Tests (`identity_and_access/tests/`)

Sibling unit and security regression test specifications for OAuth callbacks, JWT verification, session rotation, and role hierarchy bounds.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `oauth_flow.rs`
- **Coverage**: OAuth state generation entropy, state tampering detection, code exchange mock failures, and role resolution from mock Discord responses.

### `session_lifecycle.rs`
- **Coverage**: Refresh token issuance, valid rotation, token reuse detection (family invalidation), and logout cleanup.

### `user_profile.rs`
- **Coverage**: Profile serialization, unauthenticated rejection (401), and preferences update validation.

### `dev_auth.rs`
- **Coverage**: Verification that dev-login returns 404 in non-development environments, and correctly mints specified roles in development mode.

### `jwt_tokens.rs`
- **Coverage**: Signature verification, expired token rejection, tampered claims detection, and malformed header handling.

### `role_hierarchy.rs`
- **Coverage**: Monotonic role hierarchy comparisons, permission boundary invariants, and self-demotion prevention rules.
