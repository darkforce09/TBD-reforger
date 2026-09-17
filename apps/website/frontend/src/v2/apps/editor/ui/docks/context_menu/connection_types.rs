//! Connection types.

use super::*;

/// A connection relation offered by the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnKind {
    Sync,
    Group,
    TriggerOwner,
}

impl ConnKind {
    /// Returns the stored vocabulary token for this choice.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Group => "group",
            Self::TriggerOwner => "triggerOwner",
        }
    }

    /// Returns the displayed label for this choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sync => "Sync to",
            Self::Group => "Group to",
            Self::TriggerOwner => "Set Trigger Owner",
        }
    }

    /// Every supported choice in stable menu order.
    pub const ALL: [Self; 3] = [Self::Sync, Self::Group, Self::TriggerOwner];

    /// Parses a stored connection relation token.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.token() == token)
    }
}

/// A formation pattern offered by the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormationKind {
    Column,
    StaggerColumn,
    Wedge,
    EchelonLeft,
    EchelonRight,
    Vee,
    Line,
    File,
    Diamond,
}

impl FormationKind {
    /// Returns the stored vocabulary token for this choice.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Column => "column",
            Self::StaggerColumn => "stagger_column",
            Self::Wedge => "wedge",
            Self::EchelonLeft => "echelon_left",
            Self::EchelonRight => "echelon_right",
            Self::Vee => "vee",
            Self::Line => "line",
            Self::File => "file",
            Self::Diamond => "diamond",
        }
    }

    /// Returns the displayed label for this choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Column => "Column",
            Self::StaggerColumn => "Staggered Column",
            Self::Wedge => "Wedge",
            Self::EchelonLeft => "Echelon Left",
            Self::EchelonRight => "Echelon Right",
            Self::Vee => "Vee",
            Self::Line => "Line",
            Self::File => "File",
            Self::Diamond => "Diamond",
        }
    }

    /// Every supported choice in stable menu order.
    pub const ALL: [Self; 9] = [
        Self::Column,
        Self::StaggerColumn,
        Self::Wedge,
        Self::EchelonLeft,
        Self::EchelonRight,
        Self::Vee,
        Self::Line,
        Self::File,
        Self::Diamond,
    ];
}
