//! Integration tests of identity and access: Discord sign-in and its HTTP clients, session
//! issuance and refresh, the Arma link handshake, Discord membership authority and grace, the
//! dev login, and the administration roster and approvals.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../router_boot_support/mod.rs"]
mod router_boot_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod admin_approvals_cms_field_tools;
mod arma_id_whitespace;
mod auth_refresh;
mod authentication_session_transactions;
mod dev_login_runtime_identity;
mod discord_embed_sanitisation;
mod discord_http_clients;
mod discord_membership_authority;
mod discord_membership_http;
mod identity_deleted_owner_reclamation;
mod identity_link;
mod identity_link_transactions;
mod membership_grace_transactions;
mod personnel_pagination;
