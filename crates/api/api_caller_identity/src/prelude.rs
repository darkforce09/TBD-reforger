//! The names a caller of the caller identity checks imports with
//! `use api_caller_identity::prelude::*;`.

pub use crate::UserRole;
pub use crate::account_authority::{
    AccountAuthority, holds_administrator_authority, load_account_authority, lock_account,
};
pub use crate::arma_identity_link::arma_id_is_linked;
pub use crate::cached_membership_permissions::{
    CachedMembershipPermissionDecision, evaluate_cached_membership_permissions,
};
pub use crate::identity_ownership::{lock_accounts, lock_identities};
pub use crate::machine_caller::{MachineCaller, authenticate_machine};
pub use crate::session_authorization::{
    DatabaseSessionAuthority, authorize_on_connection, authorize_session,
};
