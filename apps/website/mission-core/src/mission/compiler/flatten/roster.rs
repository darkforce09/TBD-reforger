//! Role: roster.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    DiagnosticAcc, FactionIn, HashMap, HashSet, ModSettings, ModVehicle, ModVehicleSeat,
    SettingsIn, SquadIn, VEHICLE_SEAT_ROLES, VehicleIn, normalize_heading, parse_seat_id,
    render_authored, render_authored_str, roster_uid, vehicle_faction_key, vehicle_inventory,
};

/// `emitted_slot_uids` is the set of `slots[].uid` that ACTUALLY REACHED the document, not the authored slot map: `$defs/vehicle.seats[].slotId` references the wire's slots, so a crew entry naming an authored-but-unplaced seat would resolve in the payload and dangle on the wire.
pub(super) fn derive_vehicle_roster(
    vehicles: &[VehicleIn],
    squads: &[SquadIn],
    factions: &[FactionIn],
    emitted_slot_uids: &HashSet<&str>,
    aliases: &crate::mission::kit::KitAliases,
    diagnostics: &mut DiagnosticAcc,
) -> Vec<ModVehicle> {
    let squads_by_id: HashMap<&str, &SquadIn> = squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let factions_by_id: HashMap<&str, &FactionIn> =
        factions.iter().map(|f| (f.id.as_str(), f)).collect();

    let mut out = Vec::new();
    for (i, v) in vehicles.iter().enumerate() {
        match project_roster_vehicle(
            v,
            &squads_by_id,
            &factions_by_id,
            emitted_slot_uids,
            aliases,
        ) {
            Ok(row) => out.push(row),
            Err(reason) => diagnostics.vehicle_roster_dropped(i, v, &reason),
        }
    }
    out
}

/// One authored roster row projected onto `$defs/vehicle`, or the clause saying why the wire cannot carry it. See [`derive_vehicle_roster`] for the drop-whole rule.
pub(super) fn project_roster_vehicle(
    v: &VehicleIn,
    squads_by_id: &HashMap<&str, &SquadIn>,
    factions_by_id: &HashMap<&str, &FactionIn>,
    emitted_slot_uids: &HashSet<&str>,
    aliases: &crate::mission::kit::KitAliases,
) -> Result<ModVehicle, String> {
    let Some(alias) = aliases.vehicle_for_resource(v.resource_name.trim()) else {
        return Err(format!(
            "its resourceName {} has no `veh:` alias in kit-aliases.json and `$defs/vehicle` \
             requires one — substituting a vehicle that does have an alias is the T-200 silent \
             swap with ten tonnes in place of a rifleman",
            render_authored_str(&v.resource_name)
        ));
    };

    let Some(pos) = v.position.as_ref() else {
        return Err(
            "it has no map position and `$defs/vehicle` requires `x` and `z` — a place the author \
             never picked cannot be invented for it"
                .to_string(),
        );
    };

    let uid = roster_uid(v)?;

    Ok(ModVehicle {
        alias: alias.to_string(),
        uid,
        x: pos.x,
        z: pos.y,
        heading_deg: normalize_heading(pos.rotation),
        faction: vehicle_faction_key(v, squads_by_id, factions_by_id),
        seats: project_crew(v, emitted_slot_uids)?,
        inventory: vehicle_inventory(v),
    })
}

/// `slotId` is emitted verbatim and is checked for membership in `emitted_slot_uids` rather than re-gated for wire safety: it must EQUAL a `slots[].uid` already in the document, so whatever the slot emit carried, this carries, and the two cannot drift apart.
pub(super) fn project_crew(
    v: &VehicleIn,
    emitted_slot_uids: &HashSet<&str>,
) -> Result<Vec<ModVehicleSeat>, String> {
    let crew = match &v.crew {
        serde_json::Value::Null => return Ok(Vec::new()),
        serde_json::Value::Object(m) => m,
        other => {
            return Err(format!(
                "its `crew` is {}, not the seat-to-slot map the crew panel authors, so the plan \
                 cannot be read — reporting it is the alternative to shipping the vehicle as \
                 uncrewed and telling nobody",
                render_authored(other)
            ));
        }
    };

    let mut seats: Vec<ModVehicleSeat> = Vec::with_capacity(crew.len());
    for (seat_id, occupant) in crew {
        let Some(slot_id) = occupant.as_str().filter(|s| !s.trim().is_empty()) else {
            return Err(format!(
                "its crew seat {} holds {}, which does not name a slot",
                render_authored_str(seat_id),
                render_authored(occupant)
            ));
        };
        if !emitted_slot_uids.contains(slot_id) {
            return Err(format!(
                "its crew seat {} names slot {}, which is not on the compiled roster — \
                 `$defs/vehicle.seats[].slotId` references `slots[].uid`, so the reference would \
                 dangle in front of the game server",
                render_authored_str(seat_id),
                render_authored_str(slot_id)
            ));
        }
        let Some((role, index)) = parse_seat_id(seat_id) else {
            return Err(format!(
                "its crew seat {} is not a station `$defs/vehicle.seats[].role` can name \
                 (driver, commander, gunner, cargo, pilot, copilot or turret, each optionally \
                 numbered from 1)",
                render_authored_str(seat_id)
            ));
        };
        seats.push(ModVehicleSeat {
            slot_id: slot_id.to_string(),
            role: role.to_string(),
            index,
        });
    }

    let seat_key = |s: &ModVehicleSeat| {
        (
            VEHICLE_SEAT_ROLES
                .iter()
                .position(|r| *r == s.role)
                .unwrap_or(usize::MAX),
            s.index.unwrap_or(0),
        )
    };
    seats.sort_by_key(seat_key);
    if let Some(pair) = seats
        .windows(2)
        .find(|w| seat_key(&w[0]) == seat_key(&w[1]))
    {
        return Err(format!(
            "two of its crew seats name the same station ({} {}), so one soldier's seat would \
             overwrite the other's",
            pair[0].role,
            pair[0].index.unwrap_or(0)
        ));
    }
    Ok(seats)
}

/// Derive settings using the supplied domain data.
pub(super) fn derive_settings(authored: &Option<SettingsIn>) -> Option<ModSettings> {
    let Some(s) = authored else {
        return None;
    };
    let trim = |v: &Option<String>| {
        v.as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    };
    Some(ModSettings {
        respawn: trim(&s.respawn),
        spectator_policy: trim(&s.spectator_policy),
        night_vision: s.night_vision,
    })
}
