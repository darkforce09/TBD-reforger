//! The crate's error: why an inspector or dialog edit was refused.
//!
//! **Role:** the one failure type of the crate's fallible public functions: the row edits of the
//! authored blocks the inspectors write (audio emitters and music cues, weather keyframes, radio
//! nets, spawn modules, tasks, win conditions) and the ORBAT manager's update of a faction
//! template from a side.
//! **Position:** built where an edit's own rule or the authored block's schema refuses the
//! result, before the edit can become an undo step; read by the inspector views, which show the
//! text under the control, and by the ORBAT manager, which words the refusal with the side and
//! template names.
//! **Signals & state:** none; plain data.
//! **Invariants:** a [`Error::Refused`](crate::error::Error::Refused) displays its refusal clause byte for byte (the inspectors
//! show it and the tests pin it); no caller branches on the variant except the ORBAT manager's
//! wording in [`Error::message_for_side`](crate::error::Error::message_for_side).

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why an inspector or dialog edit was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The edit's own rule or the authored block's schema refuses the result. The text is the
    /// refusal clause the inspector shows under the control.
    #[error("{0}")]
    Refused(String),
    /// A faction template cannot be updated from a side that holds no roles and no vehicles.
    #[error(
        "the side has no roles and no vehicles to update the faction template from; the template \
         keeps {stored_roles} role(s) and {stored_vehicles} vehicle(s)"
    )]
    SideHasNoContent {
        /// The roles the stored faction keeps.
        stored_roles: usize,
        /// The vehicles the stored faction keeps.
        stored_vehicles: usize,
    },
}

impl From<String> for Error {
    fn from(clause: String) -> Self {
        Self::Refused(clause)
    }
}

impl From<mission_model::Error> for Error {
    fn from(refusal: mission_model::Error) -> Self {
        Self::Refused(refusal.to_string())
    }
}

impl From<&str> for Error {
    fn from(clause: &str) -> Self {
        Self::Refused(clause.to_owned())
    }
}

impl Error {
    /// The refusal as the ORBAT manager words it for the side `side` and the template `name`: a
    /// [`Error::SideHasNoContent`] names what saving would delete and how to proceed; a
    /// [`Error::Refused`] is its clause unchanged.
    #[must_use]
    pub fn message_for_side(&self, side: &str, name: &str) -> String {
        match self {
            Self::Refused(clause) => clause.clone(),
            Self::SideHasNoContent {
                stored_roles,
                stored_vehicles,
            } => {
                let holds = if *stored_roles == 0 && *stored_vehicles == 0 {
                    "and it is empty too".to_string()
                } else {
                    format!(
                        "and it still holds {stored_roles} role(s) and {stored_vehicles} \
                         vehicle(s) that saving would delete"
                    )
                };
                format!(
                    "{side} has no roles and no vehicles, so there is nothing to update \
                     \"{name}\" from — {holds}. Place slots under {side} first, or edit the \
                     template directly in the Faction Manager."
                )
            }
        }
    }
}
