// ---- work-ticket class ----

/// The closed class set, mirrored from [`ticket_engine::CLASS_VALUES`] (parity is
/// test-pinned). An enum so the chip accent match below is TOTAL — a 6th class
/// fails compile here before it can ever render unstyled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Bug,
    Feature,
    Chore,
    Audit,
    Docs,
}

impl Class {
    pub const ALL: [Class; 5] = [
        Class::Bug,
        Class::Feature,
        Class::Chore,
        Class::Audit,
        Class::Docs,
    ];

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
