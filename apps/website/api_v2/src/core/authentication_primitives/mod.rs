//! Credential primitives shared by every authenticated surface: HS256 access tokens, and the
//! hashing and constant-time comparison helpers that keep opaque tokens unusable if the
//! database that stores them leaks.

pub mod jwt_manager;
pub mod token_hashing;

pub use jwt_manager::{Claims, Manager};
pub use token_hashing::{constant_time_equal, hash_token, numeric_code, random_token};
