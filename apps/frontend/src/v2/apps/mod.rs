//! The standalone full-screen workspaces the SPA hosts beside its document pages.
//!
//! **Role:** groups the four applications that mount their own canvas and drive the map and
//! graphics engines directly — the scenario creator CAD workspace, the tactical planning
//! whiteboard, the after-action telemetry replay player, and the engine diagnostics testbenches.
//! Each owns its entire surface: its docks, toolbelts, modals, inspectors and canvas mounting.
//! **Position:** above `core`, below `pages`. A route in `pages` opens a workspace full screen;
//! a workspace never reaches back into a page.
//! **Signals & state:** none at this level. Each workspace owns its own session state, document
//! store and render heartbeat.
//! **Invariants:** a workspace imports from `core` and from the engine crates, never from
//! `pages` and never from a sibling workspace. Modules appear here as each workspace lands, and
//! a `pub mod` line carries the same `cfg` gate as the code it declares.

/// The diagnostics testbenches: URL-only benches that drive one engine subsystem in isolation.
pub mod debug;
/// The scenario creator: the 2D/3D CAD workspace that authors a mission document.
pub mod editor;
