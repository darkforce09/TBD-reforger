//! Business logic behind identity and access: the Discord HTTP client and the profile it
//! returns, role reconciliation, session minting, account lookup, and refresh-token
//! retention.

pub mod discord_client;
pub mod discord_role_sync;
pub mod discord_user_profile;
pub mod refresh_token_purge;
pub mod session_issuance;
pub mod user_lookup;
