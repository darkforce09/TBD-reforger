//! What the Waves tab asks the application to do.
//!
//! **Role:** `WavePlanEvent`: select or compare a ticket, copy text, toggle the wave 0 list.
//! **Position:** emitted by the desktop application's Waves tab; converted into an `Action` by
//! `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** nothing the tab emits writes the lock or repacks waves.

/// What the Waves tab asks the application to do.
pub enum WavePlanEvent {
    /// Select the ticket at this corpus index.
    Select(usize),
    /// Pick the ticket at this corpus index as the comparison.
    Compare(usize),
    /// Copy this text to the clipboard.
    CopyText(String),
    /// Show or hide the wave 0 id list.
    ToggleWave0,
}
