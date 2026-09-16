//! Role: flatten.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    BTreeMap, CALLSIGN_FALLBACK, COMPILE_DATE_ANCHOR, CompileError, DiagnosticAcc, EditorPayload,
    EnvironmentAxes, HashMap, HashSet, META_NAME_MAX_CHARS, MissionMeta, ModEnvironment,
    ModFaction, ModMeta, ModMissionDocument, ModOrbatFaction, ModOrbatGroup, ModOrbatRole, ModSlot,
    ROLE_FALLBACK, RadioNetSource, SLOT_IDENTITY_DROPS, SLOT_RANKS, SLOT_STANCES, SlotIn, SquadIn,
    SubstitutionAcc, apply_timeout_to_flow, derive_briefings, derive_entities, derive_flow,
    derive_settings, derive_vehicle_roster, derive_vehicles_as_entities, derive_zones,
    emit_enum_identity, emit_wire_safe_identity, flow_seconds_authored, identity_drop_reason,
    is_authored, load_kit_aliases, mission_doc_id, mission_terrain_key, mod_slot_loadout,
    normalize_heading, or_fallback, render_authored, resolve_radio_plan, resolve_win_conditions,
    slug_key,
};

/// Compile the authored graph into a game document, preserving order and reporting refused values.
pub fn flatten_to_mod_document(
    mission: &MissionMeta,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    let aliases = load_kit_aliases();
    let parsed: EditorPayload =
        serde_json::from_slice(payload).map_err(|e| CompileError::Parse(e.to_string()))?;

    let authored_root = parsed.authored_blocks_root();
    let ed = parsed.editor;

    let squads_by_id: HashMap<&str, &SquadIn> =
        ed.squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let slots_by_id: HashMap<&str, &SlotIn> = ed.slots.iter().map(|s| (s.id.as_str(), s)).collect();

    let squad_pos: HashMap<&str, usize> = ed
        .squads
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let slot_pos: HashMap<&str, usize> = ed
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let mut diagnostics = DiagnosticAcc::default();

    let mut factions: Vec<ModFaction> = Vec::new();
    let mut orbat: BTreeMap<String, ModOrbatFaction> = BTreeMap::new();
    let mut doc_slots: Vec<ModSlot> = Vec::new();
    let mut centroids: HashMap<String, (f64, f64, i64)> = HashMap::new();
    let mut centroid_order: Vec<String> = Vec::new();
    let mut radio_sources: Vec<RadioNetSource> = Vec::new();
    let mut substitutions = SubstitutionAcc::default();
    let mut any_y = false;

    let mut any_1_3_key = false;

    for f in &ed.factions {
        let faction_key = slug_key(&f.key, "faction");
        let (default_kit, preset) = aliases.faction_default(&faction_key);
        let mut groups: Vec<ModOrbatGroup> = Vec::new();

        for squad_id in &f.squad_ids {
            let Some(sq) = squads_by_id.get(squad_id.as_str()) else {
                continue;
            };
            let mut rows: Vec<&SlotIn> = sq
                .slot_ids
                .iter()
                .filter_map(|id| slots_by_id.get(id.as_str()).copied())
                .collect();
            if rows.is_empty() {
                continue;
            }
            rows.sort_by_key(|s| s.index);

            let mut leader_slot_id: Option<String> = None;
            if is_authored(&sq.leader_slot_id) {
                let reason = match emit_wire_safe_identity(&sq.leader_slot_id) {
                    None if !sq.leader_slot_id.is_string() => Some(
                        "the authored value is not a string, and `leaderSlotId` carries a slot id",
                    ),
                    None => Some(
                        "it carries a control character, which `$defs/group.leaderSlotId` \
                         (`wireSafeString`) forbids — a tab or newline here shifts every column of \
                         the mod's tab-separated roster wire",
                    ),
                    Some(id) if !rows.iter().any(|sl| sl.id == id) => Some(
                        "no seat in this squad has that id, so the reference would dangle on the \
                         wire and the mod would fall back to picking a leader anyway",
                    ),
                    Some(id) => {
                        any_1_3_key = true;
                        leader_slot_id = Some(id);
                        None
                    }
                };
                if let Some(reason) = reason {
                    diagnostics.squad_leader_dropped(
                        squad_pos.get(sq.id.as_str()).copied().unwrap_or(0),
                        sq,
                        &render_authored(&sq.leader_slot_id),
                        reason,
                    );
                }
            }

            let callsign = if sq.callsign.is_empty() {
                or_fallback(or_fallback(&sq.name, &sq.id), CALLSIGN_FALLBACK).to_string()
            } else {
                sq.callsign.clone()
            };

            let mut role_counters: HashMap<&str, i64> = HashMap::new();
            let mut role_index: HashMap<&str, usize> = HashMap::new();
            let mut roles: Vec<ModOrbatRole> = Vec::new();

            for sl in &rows {
                let role = or_fallback(&sl.role, ROLE_FALLBACK);
                let occurrence = *role_counters.get(role).unwrap_or(&0);
                role_counters.insert(role, occurrence + 1);

                let kit = match aliases.kit_for_resource(&sl.asset_id) {
                    Some(alias) => alias.to_string(),
                    None => {
                        if !sl.asset_id.is_empty() {
                            substitutions.record(
                                &sl.asset_id,
                                &faction_key,
                                default_kit,
                                &sl.id,
                                || format!("{faction_key}:{callsign}:{role}:{occurrence}"),
                            );
                        }
                        default_kit.to_string()
                    }
                };

                if let Some(&idx) = role_index.get(role) {
                    roles[idx].count += 1;
                } else {
                    role_index.insert(role, roles.len());

                    roles.push(ModOrbatRole {
                        slot: role.to_string(),
                        kit: kit.clone(),
                        count: 1,
                    });
                }

                let x = sl.position.x;
                let z = sl.position.y;
                let elev = sl.position.z;
                let y = if elev != 0.0 && !elev.is_nan() && !elev.is_infinite() {
                    any_y = true;
                    Some(elev)
                } else {
                    None
                };

                let authored_identity: [&serde_json::Value; 5] =
                    [&sl.tag, &sl.callsign, &sl.rank, &sl.stance, &sl.unit_name];
                let mut emitted: [Option<String>; 5] = [None, None, None, None, None];
                for (i, (key, rule_id)) in SLOT_IDENTITY_DROPS.into_iter().enumerate() {
                    let value = authored_identity[i];

                    if !is_authored(value) {
                        continue;
                    }
                    let resolved = match key {
                        "rank" => emit_enum_identity(value, &SLOT_RANKS),
                        "stance" => emit_enum_identity(value, &SLOT_STANCES),
                        _ => emit_wire_safe_identity(value),
                    };
                    match resolved {
                        Some(v) => {
                            any_1_3_key = true;
                            emitted[i] = Some(v);
                        }
                        None => diagnostics.slot_identity_dropped(
                            rule_id,
                            key,
                            slot_pos.get(sl.id.as_str()).copied().unwrap_or(0),
                            sl,
                            value,
                            identity_drop_reason(key, value),
                        ),
                    }
                }
                let [tag, slot_callsign, rank, stance, unit_name] = emitted;

                doc_slots.push(ModSlot {
                    id: format!("{faction_key}:{callsign}:{role}:{occurrence}"),
                    uid: sl.id.clone(),
                    faction: faction_key.clone(),
                    group_callsign: callsign.clone(),
                    role: role.to_string(),
                    kit,
                    x,
                    z,
                    y,
                    heading_deg: normalize_heading(sl.position.rotation),
                    loadout: sl.loadout.as_ref().and_then(mod_slot_loadout),

                    callsign: slot_callsign,
                    rank,
                    stance,
                    unit_name,
                    tag,
                });

                if !centroids.contains_key(&faction_key) {
                    centroids.insert(faction_key.clone(), (0.0, 0.0, 0));
                    centroid_order.push(faction_key.clone());
                }
                let c = centroids.get_mut(&faction_key).expect("inserted");
                c.0 += x;
                c.1 += z;
                c.2 += 1;
            }

            groups.push(ModOrbatGroup {
                callsign,
                kind: "rifle_squad".to_string(),
                roles,
                leader_slot_id,
            });
        }

        let display_name = if f.name.is_empty() {
            faction_key.clone()
        } else {
            f.name.clone()
        };

        if !groups.is_empty() {
            radio_sources.push(RadioNetSource {
                faction_key: faction_key.clone(),
                display_name: display_name.clone(),
                callsigns: groups.iter().map(|g| g.callsign.clone()).collect(),
            });
            orbat.insert(faction_key.clone(), ModOrbatFaction { groups });
        }
        factions.push(ModFaction {
            key: faction_key,
            display_name,
            preset_id: preset.to_string(),
            tickets: 0,
        });
    }

    if doc_slots.is_empty() {
        return Err(CompileError::NoSlots);
    }

    let mut entities = derive_entities(&parsed.entities);
    entities.extend(derive_vehicles_as_entities(
        &parsed.vehicles,
        &ed.squads,
        &ed.factions,
        aliases,
    )?);

    let emitted_slot_uids: HashSet<&str> = doc_slots.iter().map(|s| s.uid.as_str()).collect();
    let doc_vehicles = derive_vehicle_roster(
        &parsed.vehicles,
        &ed.squads,
        &ed.factions,
        &emitted_slot_uids,
        aliases,
        &mut diagnostics,
    );

    any_1_3_key |= !doc_vehicles.is_empty();

    let env_axes = EnvironmentAxes::from_payload_bag(&parsed.environment);
    any_1_3_key |= env_axes.any_on_wire();

    let schema_version = if any_1_3_key {
        "1.3"
    } else if any_y {
        "1.2"
    } else {
        "1.1"
    }
    .to_string();

    let terrain = mission_terrain_key(&mission.terrain, &mission.custom_terrain_name);

    let zones = derive_zones(&parsed.zones, &centroid_order, &centroids, &terrain);

    let sides_holding_slots = {
        let mut sides: Vec<&str> = doc_slots.iter().map(|s| s.faction.as_str()).collect();
        sides.sort_unstable();
        sides.dedup();
        sides.len()
    };
    let derived_end_on = {
        let mut triggers = vec!["time_limit".to_string()];
        if sides_holding_slots >= 2 {
            triggers.push("faction_eliminated".to_string());
        }
        triggers
    };

    let (authored_blocks, mut block_refusals) =
        crate::data::scenario::extensions::AuthoredBlocks::parse(&authored_root);
    let (extensions, carrier_refusals) =
        crate::data::scenario::extensions::ExtensionBlocks::from_payload(&authored_root);
    block_refusals.extend(carrier_refusals);
    for (key, clause) in &block_refusals {
        diagnostics.win_conditions(
            format!(
                "The authored `{key}` block is not one this compile can carry: {clause}. The \
                 compile falls back to the value it derives, so the mission still loads — with a \
                 rule you did not choose."
            ),
            key,
        );
    }

    let zone_ids: HashSet<&str> = zones.iter().map(|z| z.id.as_str()).collect();
    let win_conditions = resolve_win_conditions(
        authored_blocks.win_conditions.as_ref(),
        derived_end_on,
        sides_holding_slots,
        &zone_ids,
        &emitted_slot_uids,
        &mut diagnostics,
    );
    let mut flow = derive_flow(&parsed.environment);
    apply_timeout_to_flow(
        &mut flow,
        &win_conditions,
        flow_seconds_authored(&parsed.environment, "timeLimitSeconds"),
        &mut diagnostics,
    );

    let max_players = if mission.max_players < 1 {
        (doc_slots.len() as i64).max(1)
    } else {
        mission.max_players
    };

    let meta = ModMeta {
        id: mission_doc_id(&mission.id),

        name: if mission.title.is_empty() {
            "Untitled Mission".to_string()
        } else {
            mission.title.chars().take(META_NAME_MAX_CHARS).collect()
        },
        author: mission.author.clone(),
        terrain,
        template_id: "editor_v1".to_string(),
        player_range: [1, max_players],
    };

    let mut environment = ModEnvironment {
        date_time: String::new(),
        weather_preset: mission.weather_preset.clone(),
        wind_dir_deg: env_axes.wind_dir_deg,
        fog: env_axes.fog,
        wind: env_axes.wind,
        view_distance: env_axes.view_distance,
    };
    if !mission.time_of_day.is_empty() {
        let t = if mission.time_of_day.len() > 5 {
            &mission.time_of_day[..5]
        } else {
            &mission.time_of_day
        };
        environment.date_time = format!("{COMPILE_DATE_ANCHOR}T{t}:00Z");
    }

    Ok(ModMissionDocument {
        schema_version,
        meta,
        environment: Some(environment),
        factions,
        orbat,
        slots: doc_slots,
        entities,
        radio_plan: resolve_radio_plan(authored_blocks.radio_plan.as_ref(), &radio_sources),
        zones,
        flow,

        win_conditions,
        extensions,
        briefings: derive_briefings(&ed.factions),
        settings: derive_settings(&parsed.settings),
        vehicles: doc_vehicles,
        kit_substitutions: substitutions.finish(),
        diagnostics: diagnostics.findings,
    })
}
