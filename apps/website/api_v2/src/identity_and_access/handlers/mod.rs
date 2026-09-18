//! HTTP handlers for identity and access: Discord sign-in, session token rotation, the
//! caller's own profile, and the two halves of the Arma account-link handshake.

pub mod arma_link_codes;
pub mod arma_link_confirmation;
pub mod developer_login;
pub mod discord_oauth;
pub mod member_profile;
pub mod oauth_host_guard;
pub mod session_tokens;
