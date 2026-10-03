//! Who may read and who may change a mission row.
//!
//! A mission is visible to everyone once it is `Live`; before that it belongs to its author, and
//! an admin stands in for the author everywhere.

use crate::models::mission::{Mission, MissionStatus};
use api_http_layer::middleware::AuthUser;

pub(crate) fn can_edit(u: &AuthUser, m: &Mission) -> bool {
    m.author_id == u.discord_id || u.role == "admin"
}

pub(crate) fn can_view(u: &AuthUser, m: &Mission) -> bool {
    m.status == MissionStatus::Live || can_edit(u, m)
}
