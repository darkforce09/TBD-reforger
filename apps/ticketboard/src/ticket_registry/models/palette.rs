use super::scope::ScopeLevel;
use ticket_engine::StatusName;
/// Status accent for tree rows, wave chips, and the status filter toggles. The RAW
/// status name stays the label everywhere — color is an accent, never a rename.
pub(crate) fn status_rgb(status: StatusName) -> (u8, u8, u8) {
    match status {
        StatusName::Idea => (150, 150, 150),
        StatusName::Queued => (120, 165, 225),
        StatusName::Ready => (120, 205, 130),
        StatusName::Running => (245, 175, 80),
        StatusName::Review => (195, 150, 235),
        StatusName::Shipped => (105, 150, 115),
        StatusName::Deferred => (180, 150, 110),
        StatusName::Cancelled => (215, 115, 105),
    }
}

/// Muted per-level breadcrumb accents — desaturated hues so the scope
/// path reads as ONE quiet chip trail, distinct from the loud status accents.
pub(crate) fn scope_level_rgb(level: ScopeLevel) -> (u8, u8, u8) {
    match level {
        ScopeLevel::Domain => (170, 190, 215),
        ScopeLevel::Layer => (160, 195, 175),
        ScopeLevel::Component => (205, 185, 150),
        ScopeLevel::Surface => (185, 165, 205),
    }
}
