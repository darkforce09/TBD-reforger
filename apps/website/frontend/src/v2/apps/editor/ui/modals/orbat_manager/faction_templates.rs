//! Faction templates for the ORBAT manager.

use super::*;

#[must_use]
/// Checks whether the operator confirmed applying a faction template.
pub fn apply_confirm_allows(confirmed: bool) -> bool {
    confirmed
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Explains why a faction cannot be updated from an empty side.
pub enum SaveFromSideRefusal {
    NoContent {
        stored_roles: usize,
        stored_vehicles: usize,
    },
}

impl SaveFromSideRefusal {
    #[must_use]
    /// Provides message for the ORBAT dialog.
    pub fn message(&self, side: &str, name: &str) -> String {
        match *self {
            Self::NoContent {
                stored_roles,
                stored_vehicles,
            } => {
                let holds = if stored_roles == 0 && stored_vehicles == 0 {
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

/// Combines derived ORBAT rows with stored fields the mission cannot express.
pub fn merge_faction_doc_from_side(
    stored: &FactionDoc,
    derived: FactionDoc,
) -> Result<FactionDoc, SaveFromSideRefusal> {
    if derived.roles.is_empty() && derived.vehicles.is_empty() {
        return Err(SaveFromSideRefusal::NoContent {
            stored_roles: stored.roles.len(),
            stored_vehicles: stored.vehicles.len(),
        });
    }

    let mut labels: HashMap<&str, Vec<Option<String>>> = HashMap::new();
    for v in &stored.vehicles {
        labels
            .entry(v.vehicle.as_str())
            .or_default()
            .push(v.label.clone());
    }
    for queue in labels.values_mut() {
        queue.reverse();
    }

    let vehicles = derived
        .vehicles
        .into_iter()
        .map(|mut v| {
            if v.label.is_none() {
                v.label = labels
                    .get_mut(v.vehicle.as_str())
                    .and_then(Vec::pop)
                    .flatten();
            }
            v
        })
        .collect();

    Ok(FactionDoc {
        side: derived.side,
        name: stored.name.clone(),
        emblem: stored.emblem.clone(),
        roles: derived.roles,
        vehicles,
    })
}

#[must_use]
/// Describes rows a Save operation would remove from the library faction.
pub fn save_from_side_shrink_warning(
    stored: &FactionDoc,
    next: &FactionDoc,
    side: &str,
) -> Option<String> {
    let lost_roles = stored.roles.len().saturating_sub(next.roles.len());
    let lost_vehicles = stored.vehicles.len().saturating_sub(next.vehicles.len());
    if lost_roles == 0 && lost_vehicles == 0 {
        return None;
    }
    Some(format!(
        "Update \"{}\" from {side}?\n\nThis drops {lost_roles} role(s) and {lost_vehicles} \
         vehicle(s) the template holds but {side} does not.",
        next.name
    ))
}

#[must_use]
/// Lists templates matching the active non-civilian side.
pub fn template_options_for_side<'a>(
    library: &'a [UserFaction],
    side: &str,
) -> Vec<&'a UserFaction> {
    library
        .iter()
        .filter(|f| f.side == side && f.side != "CIV" && side != "CIV")
        .collect()
}

#[must_use]
/// Lists placeable vehicles from the active registry.
pub fn registry_vehicle_options(items: &[RegistryItem]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = items
        .iter()
        .filter(|it| it.kind == "vehicle" && it.r#abstract != Some(true) && it.variant_of.is_none())
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

#[must_use]
/// Counts roles in the projected faction document.
pub fn faction_doc_role_count(doc: &FactionDoc) -> usize {
    doc.roles.len()
}
