use super::*;

/// Strip `//`/`//!` line comments, `/* … */` block comments and the CONTENTS of double-quoted
/// string literals from EnforceScript source, so an identifier count reflects code, not prose or
/// data. Deliberately simple (no escaped-quote-in-string edge lawyering) — the corpus is the mod's
/// own hand-written `.c`, and the count only has to be STABLE and identifier-scoped, not a parser.
pub(crate) fn strip_enfusion_comments_and_strings(src: &str) -> String {
    // Block comments first (can span lines).
    let no_block = regex::Regex::new(r"(?s)/\*.*?\*/")
        .map(|re| re.replace_all(src, " ").into_owned())
        .unwrap_or_else(|_| src.to_string());
    let str_re = regex::Regex::new(r#""(?:\\.|[^"\\])*""#).ok();
    let mut out = String::with_capacity(no_block.len());
    for line in no_block.lines() {
        // T-946.24 — LITERALS FIRST, THEN `//`. It used to be the other way round, so a line
        // holding a `"http://…"` was truncated at the literal's own slashes and every identifier
        // after it vanished from the count. Found by the wave-242 verifier, which used exactly
        // that shape to hide an undefined call from the mirror-lockstep check. Blanking literals
        // first removes the slashes that are DATA before looking for the ones that start a
        // comment.
        let blanked = match &str_re {
            Some(re) => re.replace_all(line, "\"\"").into_owned(),
            None => line.to_string(),
        };
        let code = match blanked.find("//") {
            Some(i) => &blanked[..i],
            None => blanked.as_str(),
        };
        out.push_str(code);
        out.push('\n');
    }
    out
}

/// Count whole-word occurrences of `name` as an identifier across every `.c` under `mod_root`,
/// after stripping comments and string literals. The reader-count of a wire key.
pub(super) fn count_mod_readers(mod_root: &Path, name: &str) -> Result<usize> {
    let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(name)))?;
    let mut total = 0usize;
    for entry in walkdir::WalkDir::new(mod_root)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && IGNORE_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "c").unwrap_or(false))
    {
        let Ok(text) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let stripped = strip_enfusion_comments_and_strings(&text);
        total += re.find_iter(&stripped).count();
    }
    Ok(total)
}

/// The `UNREAD_WIRE_FIELDS` invariant as a list of failure strings (empty = all fields still unread
/// at their baseline). Shared by the runtime gate and the unit test so neither can drift from the
/// other's idea of "unread". `mod_root` is `apps/mod/tbd-framework`.
pub(super) fn unread_wire_field_failures(mod_root: &Path) -> Result<Vec<String>> {
    // A field spelled twice would let one row mask the other; catch the authoring slip here.
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for f in UNREAD_WIRE_FIELDS {
        if !seen.insert(f.name) {
            out.push(format!("{}: duplicated row in UNREAD_WIRE_FIELDS", f.name));
            continue;
        }
        let got = count_mod_readers(mod_root, f.name)?;
        if got != f.expected {
            out.push(format!(
                "'{}' now has {got} mod identifier(s) (baseline {}) — if {} landed the reader, \
                 DROP the field's \"no reader on any shipped build\" wording in mission.schema.json \
                 and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the \
                 pre-existing '{}' identifier, re-pin the baseline here on purpose",
                f.name, f.expected, f.ticket, f.why
            ));
        }
    }
    Ok(out)
}

/// One 1.3 wire field: `(name, expected_reader_count, ticket_that_lands_the_reader, why_baseline)`.
///
/// `expected` is 0 for a field whose spelling appears nowhere in the mod, or the measured count of a
/// pre-existing UNRELATED identifier of the same spelling (with `why` naming what it actually is).
/// A real reader for the field is a +1 over `expected` and fails the `== expected` assertion.
pub(super) struct UnreadField {
    pub(super) name: &'static str,
    pub(super) expected: usize,
    pub(super) ticket: &'static str,
    pub(super) why: &'static str,
}

/// The full set of `mission.schema.json` fields T-706 added on the wire with no mod-side reader yet.
/// Order groups them by owning ticket. Every entry is a field whose schema description asserts "no
/// reader on any shipped build"; when that stops being true this table is what fails.
pub(super) const UNREAD_WIRE_FIELDS: &[UnreadField] = &[
    // T-212 / T-685 — objectives[] typed entities + capture/defend/height rules on zoneRules.
    // T-685 — zone volume / counts / owner: RETIRED 2026-09-07. `attackerCount` /
    // `defenderCount` / `advantagePercent` / `minHeight` / `maxHeight` / `startingOwner`
    // gained identifiers when TBD_ZoneVolume.c + TBD_MissionZoneRulesStruct bind landed.
    // Baselines were 0, so they retire rather than re-pin. Flatten still omits the keys
    // (T-946.36); hand-staged 1.3 JSON reaches the reader.
    // T-689 — play-area vehicle-class filter: RETIRED 2026-09-07. `vehicleClasses` gained
    // identifiers when TBD_PlayAreaVehicleAxis.c plus the TBD_MissionZoneRulesStruct bind
    // landed (ZoneRegistry apply). Baseline was 0, so it retires rather than re-pin.
    // Authored `[]` and absent both Count()==0 (T-946.42); both confine everyone.
    // `objectives` collides with TBD_ObjectivesComponent's own member (the win-condition objective
    // list it already tracks) — NOT a reader of the new top-level `objectives[]` document array.
    //
    // RE-PINNED 13 -> 16 (2026-09-08, T-212), not retired. The 13 are still that unrelated
    // component member and still worth a tripwire; T-212's `TBD_ObjectiveEntityReader` added 3
    // (`TBD_ObjectiveEntityDocStruct.objectives`, which is what JsonLoadContext binds, plus the
    // two Read() references to it). Same class as `seats` 8 -> 12 and `gadgets` 6 -> 33: retiring
    // a non-zero-baseline row silently drops the tripwire the non-zero half exists for, which is
    // exactly what the wave-242 verifier flagged.
    UnreadField {
        name: "objectives",
        expected: 16,
        ticket: "T-212",
        why: "13 are TBD_ObjectivesComponent's own objective-list field, unrelated to the mission-doc objectives[] array; 3 are T-212's TBD_ObjectiveEntityReader binding that array",
    },
    // T-212 — objective per-side framing + WOG's _Lock/_AutoLose: RETIRED 2026-09-08. `framing`
    // (0 -> 10) and `autoLose` (0 -> 2) gained readers when TBD_ObjectiveRegistry.c grew
    // `TBD_ObjectiveEntityReader` — a third typed JsonLoadContext pass over GetRawJson(), which
    // TBD_MissionDocumentStruct has no field for. Baselines were 0 — the whole assertion was "no
    // reader yet" — so they RETIRE rather than re-pin, the T-674/T-680/T-681 handling.
    //
    // Re-pinning them at 10 and 2 was considered and rejected: a non-zero baseline in this table
    // MEANS "these hits are a pre-existing UNRELATED identifier" (that is what
    // `nonzero_baselines_explain_the_pre_existing_identifier` enforces in the `why` wording), and
    // for these two it would be false — every hit is T-212's own reader. The same reasoning
    // retired `callsign` and `tag` at T-674 rather than pin a sentence that lies.
    //
    // `lock` still has no row and still needs none: it shares the word with `entity.lock` /
    // `vehicle.lock`, which T-680 already retired, so there was nothing left to pin. T-212 took
    // its count 6 -> 8 (`TBD_ObjectiveEntityStruct.lock` and the copy onto the objective).
    //
    // What replaced the tripwire: `t212_objective_spine_tests` at the bottom of this file, which
    // asserts the complement — every `$defs/objective` property must KEEP a reader in the
    // objectives lane, so deleting the reader is a red instead of a silent regression.
    // T-675 / T-076 — vehicles[] roster: RETIRED 2026-09-06. `vehicles` gained its reader when
    // T-675.2 landed `TBD_MissionVehicleStruct.c` and the `TBD_MissionDocumentStruct.vehicles`
    // binding. Its baseline was 0 — the whole assertion was "no reader yet" — so it retires rather
    // than re-pins. `JsonLoadContext` binds by member name, so the identifier IS the contract: no
    // reader for this field can exist without spelling it.
    //
    // `seats` is RE-PINNED, not retired. Its baseline of 8 was pre-existing briefing/lobby
    // seat-count UI, UNRELATED to the vehicle crew plan, and the wave-242 verifier's finding was
    // precisely that retiring a non-zero-baseline row silently drops that tripwire. T-675.2's crew
    // reader added 4, so the new floor is 12 and the unrelated 8 stay guarded.
    UnreadField {
        name: "seats",
        expected: 11,
        ticket: "T-675",
        why: "7 pre-existing seat-count identifiers in TBD_LobbyCatalog and TBD_PlayersCatalog + 4 crew-plan binding identifiers in TBD_MissionVehicleStruct",
    },
    // T-676 / T-079 — trigger activation/effects: RETIRED 2026-09-06. `editorTriggers`,
    // `activation`, `effects` and `variantId` all gained readers when TBD_TriggerRuntime.c landed,
    // so their "no reader on any shipped build" assertions are retired rather than re-pinned — a
    // non-zero baseline here means a pre-existing UNRELATED identifier, which these are not.
    // T-677 — per-squad waypoints: RETIRED 2026-09-06. `waypoints` and `vehicleUid` gained
    // readers when TBD_WaypointRuntime.c landed, so their "no reader on any shipped build"
    // assertions are retired rather than re-pinned.
    // T-678 — group AI state: RETIRED 2026-09-06. `combatMode` and `formation` gained readers
    // when TBD_GroupState.c landed (second JsonLoadContext pass over orbat groups). Baselines
    // were 0, so they retire rather than re-pin. T-677 already retired waypoint `speedMode` /
    // `behaviour` (identifier count is global); group-level defaults for those two now live
    // in the same GroupState reader.
    // T-679 — placement scatter: RETIRED 2026-09-07. `placementRadius` / `placementShape`
    // gained identifiers when TBD_PlacementScatter.c landed (second GetRawJson pass, ForSlot
    // from SpawnSlotBody). Baselines were 0, so they retire rather than re-pin. Flatten still
    // omits the keys (T-946.36); hand-staged 1.3 JSON reaches the reader.
    // T-680 — vehicle states: RETIRED 2026-09-06. `lock` / `fuel` / `ammo` gained readers when
    // TBD_VehicleState.c landed (second JsonLoadContext pass over vehicles[], applied from
    // TBD_SpawnManager after SeatAuthoredCrews). Baselines were 0, so they retire rather than
    // re-pin. Apply is on the vehicles[] roster; an entities[]-only row still has no consumer.
    // T-681 — entity states: RETIRED 2026-09-06. `allowDamage` / `showModel` / `stamina` /
    // `health` gained identifiers when TBD_EntityState.c landed (second JsonLoadContext pass
    // over entities[], applied from TBD_SpawnManager after VehicleState). Baselines were 0,
    // so they retire rather than re-pin. `stamina` is bound and skip-logged: Reforger has no
    // per-character enable toggle. Bools apply only when bound true (T-946.37 class).
    //
    // `size` is RE-PINNED, not retired. Its baseline of 3 was T-673 marker.size plus a
    // file-size comment, UNRELATED to entity OBJ-SIZE. T-681's SetScale reader added 11,
    // so the new floor is 14 and the unrelated 3 stay guarded (same class as seats 8→12).
    UnreadField {
        name: "size",
        expected: 14,
        ticket: "T-681",
        why: "T-673 marker.size reader (3 identifiers) plus the pre-existing file-size comment, a different field from entity OBJ-SIZE; re-pinned 3 -> 14 after T-681",
    },
    // T-682 — environment fog/wind/viewDistance: RETIRED 2026-09-06. TBD_EnvironmentReader.c
    // plus ModEnvironment serialisation landed those identifiers. Editor authoring stays refused
    // (author_env); that is a different gate.
    // T-684 — missionParams[]: RETIRED 2026-09-06. TBD_MissionParams.c plus the
    // TBD_MissionDocumentStruct.missionParams binding landed the identifier. Baseline was 0,
    // so it retires rather than re-pin. flatten.rs still does not emit the array (T-946.35
    // shape); hand-staged 1.3 JSON reaches the reader.
    // T-673 — marker style/area fields.
    // `shape` collides with the EXISTING zone-shape reader (`TBD_MissionShapeStruct`, the circle/
    // polygon zone geometry) PLUS T-673's marker.shape glyph selector. Re-pinned 32 -> 34
    // (2026-09-06, wave 244) rather than deleted: the 32 zone-geometry identifiers are still a
    // tripwire. T-673 added 2 marker.shape identifiers. Re-pinned 34 -> 36 (2026-09-07,
    // wave 248): T-679's `TBD_PlacementScatter.Scatter` takes a local parameter named `shape`
    // (two identifiers in tbd-framework) — unrelated to marker.shape / zone geometry.
    UnreadField {
        name: "shape",
        expected: 36,
        ticket: "T-673",
        why: "existing zone-shape reader (TBD_MissionShapeStruct circle/polygon geometry), a different field from marker.shape; T-679 Scatter local parameter added 2; re-pinned 34 -> 36 after T-679",
    },
    // `area` collides with the loadout-area (`LoadoutArea`) identifier family — NOT a marker reader.
    // Measured 13 (gate semantics, comments+strings stripped): TBD_LoadoutEquipHelper 7 +
    // TBD_RegistryScan 6, both LoadoutAreaType (worn-garment area). The play-area vocabulary
    // (TBD_PlayAreaComponent, ~10 RAW `area` hits) is adjacent to the T-689 lane but strips to 0
    // here — so it is NOT in this baseline; a legit re-pin may arrive with T-689's play-area reader.
    UnreadField {
        name: "area",
        expected: 11,
        ticket: "T-673",
        why: "11 pre-existing worn-garment identifiers in TBD_LoadoutEquipHelper (7) and TBD_LoadoutPreviewDresser (4), unrelated to marker.area geometry",
    },
    // T-705 — per-player gadget flags: RETIRED 2026-09-07. `compass` / `watch` / `gps`
    // gained identifiers when TBD_GadgetFlags.c landed (second GetRawJson pass, apply after
    // spawn). Baselines were 0, so they retire rather than re-pin. Flatten still omits
    // `slot.gadgets` (T-946.43); hand-staged 1.3 JSON reaches the reader.
    //
    // `gadgets` is RE-PINNED, not retired. Its baseline of 6 was the radio/gadget subsystem's
    // own vocabulary, UNRELATED to the slot.gadgets flag block. T-705's reader added 27, so
    // the new floor is 33 and the unrelated 6 stay guarded.
    UnreadField {
        name: "gadgets",
        expected: 33,
        ticket: "T-705",
        why: "6 radio/gadget subsystem identifiers, unrelated to slot.gadgets, plus 27 from the T-705 reader",
    },
    // T-654 — variant conditional-inclusion: RETIRED 2026-09-07. `variants` gained
    // identifiers when TBD_MissionLoader.c started filtering at ParseMissionJson
    // (profile TBD_VariantConfig.json else default:true). Baseline was 0, so it
    // retires rather than re-pin. objectives[] / editorTriggers[] still re-parse
    // GetRawJson() (T-946.48); kept vehicles may still list a variant-excluded crew slotId.
    // T-674 — objective-style slot identity: RETIRED 2026-09-06, all six rows.
    //
    // T-674.2 landed the Enfusion reader, so `rank` (17), `stance` (11), `unitName` (5) and
    // `leaderSlotId` (3) went from a clean 0 to real `JsonLoadContext`-bound members —
    // `JsonLoadContext` binds by field NAME, so those identifiers ARE the contract and no way of
    // writing the reader avoids them.
    //
    // `callsign` (16 -> 22) and `tag` (42 -> 45) go with them, and that is the part worth stating.
    // A non-zero baseline in this table means "these hits are a pre-existing UNRELATED identifier",
    // which `nonzero_baselines_explain_the_pre_existing_identifier` enforces in the `why` wording.
    // Once a field genuinely gains a reader that sentence is false, so re-pinning them at 22 and 45
    // would assert something untrue about a row whose whole purpose is the assertion. The unrelated
    // identifiers they used to pin (the existing `group.callsign` reader, the UI list-row `int
    // tag`) lose their tripwire with them; that is a real cost, and it is smaller than a table that
    // lies.
];
