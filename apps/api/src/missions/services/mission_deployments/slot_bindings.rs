//! Binding an event mission's ORBAT seats to the compiled slots of the artifact a deployment
//! runs. The artifact's own version payload names every compiled slot's squad and position:
//! the ORBAT derivation and the compiler walk the same factions, squads and slots in the same
//! order, so the n-th template slot is the n-th compiled slot. A seat binds to the compiled slot
//! at its faction, squad and position when both carry the same role. A deployment for an event runs only
//! when seats and compiled slots correspond one to one; anything else is refused with every
//! unpaired seat and slot named.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::operations::services::OrbatSquadTemplate;

/// The role the compiler writes for a slot authored without one.
const UNASSIGNED_ROLE: &str = "unassigned";

/// One ORBAT seat of the event mission.
#[derive(Debug, Clone)]
pub struct OrbatSeat {
    pub id: Uuid,
    pub faction: String,
    pub squad: String,
    pub slot_index: i64,
    pub role: String,
}

/// One compiled slot of the artifact's document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSlot {
    pub uid: String,
    pub role: String,
}

/// Why seats and compiled slots do not correspond.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BindingMismatch {
    /// Seats with no compiled slot of the same role at their faction, squad and position.
    pub unbound_seats: Vec<String>,
    /// Compiled slots no seat stands for.
    pub unseated_slots: Vec<String>,
    /// The template and the document disagree, so the artifact's slots have no positions.
    pub document_disagrees: Option<String>,
}

fn normalized_role(role: &str) -> &str {
    if role.is_empty() {
        UNASSIGNED_ROLE
    } else {
        role
    }
}

/// The compiled slots of a mission document, in document order.
pub fn compiled_slots(document: &Value) -> Vec<CompiledSlot> {
    document["slots"]
        .as_array()
        .map(|slots| {
            slots
                .iter()
                .map(|slot| CompiledSlot {
                    uid: slot["uid"].as_str().unwrap_or_default().to_owned(),
                    role: slot["role"].as_str().unwrap_or_default().to_owned(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Bind every seat to its compiled slot, or name everything that does not correspond.
pub fn bind_seats(
    template: &[OrbatSquadTemplate],
    compiled: &[CompiledSlot],
    seats: &[OrbatSeat],
) -> Result<Vec<(Uuid, String)>, BindingMismatch> {
    let positions: usize = template.iter().map(|squad| squad.slots.len()).sum();
    if positions != compiled.len() {
        return Err(BindingMismatch {
            document_disagrees: Some(format!(
                "the version's ORBAT has {positions} slots and its compiled document {}",
                compiled.len()
            )),
            ..BindingMismatch::default()
        });
    }
    let mut by_position: HashMap<(&str, &str, i64), &CompiledSlot> = HashMap::new();
    let mut cursor = compiled.iter();
    for squad in template {
        for (index, slot) in squad.slots.iter().enumerate() {
            let Some(compiled_slot) = cursor.next() else {
                break;
            };
            if normalized_role(&slot.role) != compiled_slot.role {
                return Err(BindingMismatch {
                    document_disagrees: Some(format!(
                        "{} position {index} is {} in the ORBAT and {} in the document",
                        squad.squad,
                        normalized_role(&slot.role),
                        compiled_slot.role
                    )),
                    ..BindingMismatch::default()
                });
            }
            by_position.insert(
                (squad.faction.as_str(), squad.squad.as_str(), index as i64),
                compiled_slot,
            );
        }
    }
    let mut mismatch = BindingMismatch::default();
    let mut bindings = Vec::with_capacity(seats.len());
    let mut bound_uids: Vec<&str> = Vec::with_capacity(seats.len());
    for seat in seats {
        match by_position.get(&(seat.faction.as_str(), seat.squad.as_str(), seat.slot_index)) {
            Some(slot)
                if slot.role == normalized_role(&seat.role)
                    && !bound_uids.contains(&slot.uid.as_str()) =>
            {
                bound_uids.push(slot.uid.as_str());
                bindings.push((seat.id, slot.uid.clone()));
            }
            _ => mismatch.unbound_seats.push(format!(
                "{} {} position {} ({})",
                seat.faction,
                seat.squad,
                seat.slot_index,
                normalized_role(&seat.role)
            )),
        }
    }
    mismatch.unseated_slots = compiled
        .iter()
        .filter(|slot| !bound_uids.contains(&slot.uid.as_str()))
        .map(|slot| format!("{} ({})", slot.uid, slot.role))
        .collect();
    if mismatch.unbound_seats.is_empty() && mismatch.unseated_slots.is_empty() {
        Ok(bindings)
    } else {
        Err(mismatch)
    }
}

#[cfg(test)]
#[path = "tests/slot_bindings.rs"]
mod tests;
