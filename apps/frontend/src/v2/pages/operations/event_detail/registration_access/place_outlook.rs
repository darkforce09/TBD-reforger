//! Whether the viewer can be given a place in the operation right now, and if not, why not.
//!
//! **Role:** reads the dossier's pools, its operation-wide remainder and the viewer's own signups
//! into the one outlook the slotting controls act on.
//! **Position:** consulted by the mission standing, which every mission card and the standalone
//! slotting page build from the operation dossier.
//! **Signals & state:** none; pure over the dossier.
//! **Invariants:** mirrors the backend's choice of pool. One place per participant is shared by
//! every mission of the operation; a new place comes first from the viewer's own pool (member or
//! guest) and then from the open pool, and never beyond the operation-wide limit. When no pool can
//! give a place yet, the earliest opening of a pool with places left is reported, the viewer's own
//! pool winning a tie. A pool state this build does not know reads as closed — the direction that
//! never promises a place the backend would refuse. The outlook only chooses which action to offer;
//! the backend decides every request.

use crate::v2::core::api::dto::EventHub;
use crate::v2::core::utils::utc_timestamp::UtcTimestamp;

/// The viewer's prospect of a place in the operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceOutlook {
    /// The viewer already holds the operation's place, which every mission shares.
    Held,
    /// A pool the viewer draws from grants a place now.
    Available,
    /// No pool grants a place now; the `quota_kind` pool has places left and opens at `opens_at`.
    OpensLater {
        quota_kind: String,
        opens_at: String,
    },
    /// The operation-wide limit, or every pool the viewer draws from, is exhausted.
    Exhausted,
}

/// Whether a reservation state holds the operation's place: an active signup, or one recorded
/// before the pools existed.
pub(crate) fn holds_place(reservation_state: Option<&str>) -> bool {
    matches!(reservation_state, Some("registered" | "legacy_unknown"))
}

/// The viewer's outlook for a new place in the operation `hub` describes.
pub(crate) fn place_outlook(hub: &EventHub) -> PlaceOutlook {
    if hub
        .missions
        .iter()
        .any(|mission| holds_place(mission.my_reservation_state.as_deref()))
    {
        return PlaceOutlook::Held;
    }
    if hub.remaining_event_places.is_some_and(|left| left <= 0) {
        return PlaceOutlook::Exhausted;
    }
    let mut earliest: Option<(Option<UtcTimestamp>, &str, &str)> = None;
    for kind in [hub.viewer_access.quota_class.as_str(), "open"] {
        let Some(pool) = hub
            .reservation_quotas
            .iter()
            .find(|pool| pool.quota_kind == kind)
        else {
            continue;
        };
        match pool.closed_reason.as_deref() {
            None if pool.open => return PlaceOutlook::Available,
            Some("not_yet_open") => {
                let opens = UtcTimestamp::parse(&pool.opens_at);
                let sooner = match &earliest {
                    None => true,
                    Some((current, _, _)) => {
                        opens.is_some() && (current.is_none() || opens < *current)
                    }
                };
                if sooner {
                    earliest = Some((opens, &pool.quota_kind, &pool.opens_at));
                }
            }
            _ => {}
        }
    }
    match earliest {
        Some((_, quota_kind, opens_at)) => PlaceOutlook::OpensLater {
            quota_kind: quota_kind.to_string(),
            opens_at: opens_at.to_string(),
        },
        None => PlaceOutlook::Exhausted,
    }
}
