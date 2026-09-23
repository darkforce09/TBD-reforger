//! The account tier ladder every permission gate in the application measures against.
//!
//! **Role:** defines the five ranked tiers, the snake_case spelling they travel in, parsing of a
//! route's declared tier, and the two comparison helpers — one for chrome, one for affordances.
//! **Position:** the bottom of the authorisation graph. The session store, the route guard, the
//! navigation registry and every page-level gate read tiers from here.
//! **Signals & state:** none. [`Role`] is a plain `Copy` value and both helpers are pure.
//! **Invariants:** the ladder is total and ordered `Guest < Enlisted < Leader < MissionMaker < Admin`.
//! The snake_case spellings are the wire form the backend sends, so renaming one breaks
//! deserialisation of a stored session as well as the route table's `auth` strings.
//!
//! A gate sees **six** outcomes, not five: "no tier at all" (`None` — anonymous, or a session that
//! has not finished bootstrapping) is its own case, and the two helpers deliberately disagree
//! about it. In tier order, and what each may reach:
//!
//! | Tier | Reaches |
//! |---|---|
//! | `None` (anonymous / pre-bootstrap) | every open route and, in browse mode, the whole sidebar; no affordance that acts on data |
//! | [`Role::Guest`] | open routes and explicitly guest-authorized actions as a signed-in nonmember |
//! | [`Role::Enlisted`] | the same open routes, as a signed-in identity that can be slotted and tracked |
//! | [`Role::Leader`] | everything Enlisted reaches; no route or nav entry asks for this tier yet |
//! | [`Role::MissionMaker`] | additionally the mission editor route and the maker affordances in the mission library |
//! | [`Role::Admin`] | additionally the Administration sidebar section and the six `/admin/*` routes |

/// One account tier on the permission ladder.
///
/// The variants are ordered lowest to highest; comparisons go through [`Role::rank`] rather than
/// derived ordering so the ladder stays explicit. The ladder is the backend's, reproduced whole:
/// `Leader` is carried because a session may deserialise into it, even though no route or
/// navigation entry asks for that tier, which is what the `dead_code` waiver covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Role {
    /// A signed-in account whose verified identity is not a TBD community member.
    Guest,
    /// A signed-in TBD community member.
    Enlisted,
    /// A member trusted to lead an element in the field.
    Leader,
    /// A member allowed to author and edit missions.
    MissionMaker,
    /// Full administrative access, including the `/admin/*` routes.
    Admin,
}

impl Role {
    /// This tier's position on the ladder, `0` for the lowest. Only the ordering is meaningful.
    fn rank(self) -> u8 {
        match self {
            Role::Guest => 0,
            Role::Enlisted => 1,
            Role::Leader => 2,
            Role::MissionMaker => 3,
            Role::Admin => 4,
        }
    }

    /// The wire and display spelling, matching the serde representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Guest => "guest",
            Role::Enlisted => "enlisted",
            Role::Leader => "leader",
            Role::MissionMaker => "mission_maker",
            Role::Admin => "admin",
        }
    }

    /// Parse the tier a route declares. An open route spells this `"none"`.
    /// Unrecognized declarations return `None`, so callers must validate declarations
    /// before treating an absent tier as public access.
    pub fn from_route_auth(auth: &str) -> Option<Role> {
        match auth {
            "none" => None,
            "guest" => Some(Role::Guest),
            "enlisted" => Some(Role::Enlisted),
            "leader" => Some(Role::Leader),
            "mission_maker" => Some(Role::MissionMaker),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// Browse-mode gate: does `user` clear `min` for the purposes of *showing chrome*?
///
/// A viewer with no tier (`None`) clears everything, so a signed-out visitor browses the full
/// navigation instead of an empty sidebar.
///
/// **Chrome only.** Never gate an action affordance with this — a New Mission button or an admin
/// tool would flash as authorised for an anonymous viewer, and between mount and the session
/// landing. Use [`has_min_role_authed`] there.
pub fn has_min_role(user: Option<Role>, min: Role) -> bool {
    match user {
        None => true,
        Some(r) => r.rank() >= min.rank(),
    }
}

/// Affordance gate: does `user` clear `min` for the purposes of *acting*?
///
/// The mirror image of [`has_min_role`] on the `None` case: an anonymous viewer, or a session that has not
/// bootstrapped yet, never clears any tier. That keeps a pre-bootstrap `None` from rendering
/// maker or admin controls and then freezing in the wrong state once the session arrives.
pub fn has_min_role_authed(user: Option<Role>, min: Role) -> bool {
    match user {
        None => false,
        Some(r) => r.rank() >= min.rank(),
    }
}

#[cfg(test)]
#[path = "tests/role.rs"]
mod tests;
