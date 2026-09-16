//! Role: zones.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// It lives here beside [`ZoneShape`] — the pure, native-tested home — for the same reason `ZoneShape` does: `editor_ops` (wasm-only) branches on it, and keeping it here is what lets a native `cargo test -p website-frontend` prove any pure logic that reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTarget {
    /// Domain representation of zone.
    Zone,

    /// Domain representation of trigger.
    Trigger,
}

impl DrawTarget {
    /// A human word for the target, for the live draw hint ("Drawing a boundary circle" vs "Drawing a presence trigger circle"). Presentation only.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Self::Zone => "zone",
            Self::Trigger => "trigger",
        }
    }
}
