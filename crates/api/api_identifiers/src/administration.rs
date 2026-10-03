//! The identifiers of the administration domain: moderation warnings and audit lines.
//!
//! **Role:** declares one typed id per administration table key, and the free-form target key an
//! audit line names.
//! **Position:** declared here, below every API crate; administration owns the tables, and every
//! domain that writes an audit line names its target with [`AuditTargetId`].
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner value
//! (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those of the
//! bare `Uuid`, `i64` or `String`.

newtype_ids::uuid_id! {
    sqlx,
    /// A moderation warning issued to a member: the key of `warnings`.
    pub struct WarningId;
}

newtype_ids::integer_id! {
    sqlx,
    /// One audit line: the `bigint` sequence key of `audit_logs`.
    pub struct AuditLogEntryId(i64);
}

newtype_ids::string_id! {
    sqlx,
    /// The record an audit line is about, as text (`audit_logs.target_id`): a row key, a Discord
    /// user id or a composite such as `<catalog>/<version>`, read together with the line's
    /// `target_type`; empty when the line names no target.
    #[derive(Default)]
    pub struct AuditTargetId;
}

impl AuditTargetId {
    /// True when the audit line names no target (the stored text is empty).
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

/// An audit target built from a formatted key, such as `&format!("{catalog}/{version}")` or
/// `&row_id.to_string()`, which every audit writer accepts as `impl Into<AuditTargetId>`.
impl From<&String> for AuditTargetId {
    fn from(text: &String) -> Self {
        Self::new(text.as_str())
    }
}

/// The target of an audit line about a member account: the member's Discord user id.
impl From<&crate::DiscordUserId> for AuditTargetId {
    fn from(member: &crate::DiscordUserId) -> Self {
        Self::new(member.as_str())
    }
}
