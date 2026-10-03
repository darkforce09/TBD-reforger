//! Business logic behind identity and access: role reconciliation, session minting, account
//! lookup, and refresh-token retention. The Discord HTTP client and the profile it returns live in
//! [`api_discord`]; the session and account authority checks every domain shares live
//! in [`api_caller_identity`].

pub mod account_registration;
pub mod discord_membership_cache;
pub mod discord_membership_enrollment;
pub mod discord_rest_reconciliation;
pub mod discord_role_sync;
pub mod membership_grace_overrides;
pub mod refresh_token_purge;
pub mod session_issuance;
pub mod session_rotation;
pub mod session_storage;
pub mod user_lookup;

pub mod identity_linking;
pub mod link_code_issuance;
