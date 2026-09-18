# Auth Primitives (`identity_and_access/auth_primitives/`)

Cryptographic token generators, signing algorithms, and compile-time role hierarchy definitions.

---

## 1. Modules

### `jwt_tokens.rs` (<280 LOC)
- **Purpose**: Encodes and verifies JSON Web Tokens (JWT).
- **Key Functions**:
  - `encode_jwt(claims: &JwtClaims, secret: &[u8]) -> Result<String, JwtError>`: Signs claims with expiration and issuer headers.
  - `decode_jwt(token: &str, secret: &[u8]) -> Result<JwtClaims, JwtError>`: Parses and cryptographically validates token integrity and timestamps.
- **Invariants**:
  - Strict validation of `exp` and `iat`. Rejects expired or future-dated tokens.

### `role_hierarchy.rs` (<220 LOC)
- **Purpose**: Implements the total ordering and capability matrix for platform roles.
- **Role Hierarchy**:
  ```text
  UserRole::Enlisted < UserRole::Leader < UserRole::MissionMaker < UserRole::Admin
  ```
- **Key Functions**:
  - `satisfies_minimum_role(actual: UserRole, required: UserRole) -> bool`: Fast comparison for authorization guards.
  - `can_manage_target_role(actor: UserRole, target: UserRole) -> bool`: Prevents lower roles from mutating higher or equal roles.
