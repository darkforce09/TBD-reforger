//! The work-ticket class vocabulary.
//!
//! **Role:** `Class`, the five work-ticket classes, with their names and accent colours.
//! **Position:** read through `projection` by the browser, the ticket actions, the estimates and the
//! desktop application's cards and details.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** `Class::ALL` equals `ticket_model::CLASS_VALUES`; every match lists each variant,
//! so a new class fails to compile until it has a colour.

// ---- work-ticket class ----

/// The closed class set, mirrored from [`ticket_model::CLASS_VALUES`] (parity is
/// test-pinned). An enum so the chip accent match below is TOTAL — a 6th class
/// fails compile here before it can ever render unstyled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A defect fix.
    Bug,
    /// New behaviour.
    Feature,
    /// Maintenance with no behaviour change.
    Chore,
    /// A review or audit.
    Audit,
    /// Documentation.
    Docs,
}

impl Class {
    /// Every class, in display order.
    pub const ALL: [Class; 5] = [
        Class::Bug,
        Class::Feature,
        Class::Chore,
        Class::Audit,
        Class::Docs,
    ];

    /// The class named `s`, `None` for any other word.
    pub fn parse(s: &str) -> Option<Class> {
        Some(match s {
            "bug" => Class::Bug,
            "feature" => Class::Feature,
            "chore" => Class::Chore,
            "audit" => Class::Audit,
            "docs" => Class::Docs,
            _ => return None,
        })
    }

    #[deny(clippy::wildcard_enum_match_arm)]
    /// The class name as written in a ticket.
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Bug => "bug",
            Class::Feature => "feature",
            Class::Chore => "chore",
            Class::Audit => "audit",
            Class::Docs => "docs",
        }
    }

    /// Class accents share the status palette: bugs are red, features blue, chores
    /// gray, audits purple, and documentation green.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn accent_rgb(self) -> (u8, u8, u8) {
        match self {
            Class::Bug => (215, 115, 105),
            Class::Feature => (120, 165, 225),
            Class::Chore => (150, 150, 150),
            Class::Audit => (195, 150, 235),
            Class::Docs => (105, 150, 115),
        }
    }
}
