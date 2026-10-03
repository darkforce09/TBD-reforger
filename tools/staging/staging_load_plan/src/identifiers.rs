//! The identifiers a load plan and its report name: request templates, member accounts, and the
//! fixture events, missions and slots the accounts write to.
//!
//! - **Role:** one newtype per kind of id, so a template id never stands where an event id is
//!   expected and no public field or parameter is a bare string.
//! - **Position:** [`crate::workload_plan`] names templates and fixture events with them,
//!   [`crate::request_catalog::AccountBinding`] binds them to placeholders, and
//!   [`crate::load_report::TemplateSummary`] reports per template.
//! - **Signals & state:** none; plain values.
//! - **Invariants:** each id serialises as its bare string, so a plan and a report cross the
//!   process boundary in the same JSON as before the types existed, and compares equal to the
//!   `str` it spells.

newtype_ids::string_id! {
    /// The name of one request template of the mix, such as `events`: lowercase letters, digits
    /// and `_`, checked when the catalog compiles.
    pub struct TemplateId;
}

newtype_ids::string_id! {
    /// The Discord id of one member account in the account file: a decimal number.
    pub struct DiscordId;
}

newtype_ids::string_id! {
    /// The id of one seeded fixture event.
    pub struct EventId;
}

newtype_ids::string_id! {
    /// The id of a fixture event's active mission attachment, which registrations name.
    pub struct EventMissionId;
}

newtype_ids::string_id! {
    /// The id of the mission attached to a fixture event, which bookmarks name.
    pub struct MissionId;
}

newtype_ids::string_id! {
    /// The id of one slot of a fixture event's mission attachment.
    pub struct SlotId;
}
