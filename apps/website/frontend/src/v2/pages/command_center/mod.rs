//! The command centre: the three read-only screens that report the unit's current situation.
//!
//! **Role:** groups the landing dashboard, the live game-server panel and the announcement
//! board. None of them writes platform data; each renders one fetched payload.
//! **Position:** the first hub in the navigation registry, rendered inside the frame.
//! **Signals & state:** each page owns its own resource and signals; nothing is shared here.
//! **Invariants:** every page in this hub sits behind the sign-in gate and fetches only from
//! the browser build — a native compile resolves its request to `None`.

pub mod announcements;
pub mod dashboard;
pub mod server_intel;

#[allow(unused_imports)]
pub use announcements::AnnouncementsPage;
#[allow(unused_imports)]
pub use dashboard::DashboardPage;
#[allow(unused_imports)]
pub use server_intel::ServerIntelPage;
