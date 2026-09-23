//! Business logic behind identity and access: the Discord HTTP client and the profile it
//! returns, role reconciliation, session minting, account lookup, and refresh-token
//! retention.

pub mod account_authority;
pub mod cached_membership_permissions;
pub mod discord_client;
pub mod discord_membership_cache;
pub mod discord_membership_enrollment;
pub mod discord_rest_reconciliation;
pub mod discord_role_sync;
pub mod discord_user_profile;
pub mod membership_grace_overrides;
pub mod refresh_token_purge;
pub mod session_authorization;
pub mod session_issuance;
pub mod session_rotation;
pub mod session_storage;
pub mod user_lookup;

pub mod identity_linking;
pub mod identity_ownership;
pub mod link_code_issuance;
