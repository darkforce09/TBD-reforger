//! Auth — HS256 JWT + token helpers (Rust port of `internal/auth`).

pub mod jwt;
pub mod tokens;

pub use jwt::{Claims, Manager};
pub use tokens::{constant_time_equal, hash_token, numeric_code, random_token};
