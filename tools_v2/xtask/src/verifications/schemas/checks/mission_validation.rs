use super::*;

pub(super) fn validate(
    root: &Path,
    sroot: &Path,
    schema: &dyn Fn(&str) -> Result<Value>,
    sorted_json_files: &dyn Fn(&Path) -> Result<Vec<String>>,
    check: &dyn Fn(&str, &jsonschema::Validator, &Value),
    failures: &std::cell::Cell<usize>,
    v_mission: &jsonschema::Validator,
) -> Result<()> {
    println!("Golden missions:");
    let missions_dir = sroot.join("golden-missions");
    for f in sorted_json_files(&missions_dir)? {
        check(&f, v_mission, &read_json(&missions_dir.join(&f))?);
    }

    // ── T-706 — schemaVersion 1.3 wire fields must stay UNREAD until their reader lands ───────
    // The ticket's own acceptance: every field the 1.3 pass added is on the wire and read by
    // NOTHING mod-side; when a reader lands under its owning ticket, this trips and forces the
    // "no reader on any shipped build" wording to come out. See UNREAD_WIRE_FIELDS.
    println!("T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):");
    {
        let mod_root = root.join("apps/mod/tbd-framework");
        let bad = unread_wire_field_failures(&mod_root)?;
        if bad.is_empty() {
            println!(
                "  PASS  {} field(s) still unread at baseline (readers land per owning ticket)",
                UNREAD_WIRE_FIELDS.len()
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  a 1.3 wire field gained a reader — update mission.schema.json");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-450 — MISSION_FILE_MAX_BYTES pin (schema keyword ↔ mod constant ↔ goldens) ─────────
    // JSON Schema cannot express whole-document byte size. The ceiling lives on the schema as
    // `x-tbd-missionFileMaxBytes` and must stay equal to `TBD_MissionLoader.MISSION_FILE_MAX_BYTES`
    // (`8 * 1024 * 1024`). Without this gate a description-only comment would rot silently.
    println!("Mission file byte ceiling (T-450):");
    {
        let mut bad: Vec<String> = Vec::new();
        let mission_schema = schema("mission.schema.json")?;
        let pinned = match mission_schema["x-tbd-missionFileMaxBytes"].as_u64() {
            Some(n) => n as usize,
            None => {
                bad.push(
                    "mission.schema.json missing x-tbd-missionFileMaxBytes — the enforceable \
                     size pin is gone (description-only is not a pin)"
                        .to_string(),
                );
                0
            }
        };
        const EXPECTED: usize = 8 * 1024 * 1024;
        if pinned != 0 && pinned != EXPECTED {
            bad.push(format!(
                "x-tbd-missionFileMaxBytes={pinned}, expected {EXPECTED} \
                 (TBD_MissionLoader.MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024)"
            ));
        }
        let loader = root.join(
            "apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionLoader.c",
        );
        let loader_src =
            fs::read_to_string(&loader).with_context(|| format!("read {}", loader.display()))?;
        if !loader_src.contains("MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024") {
            bad.push(
                "TBD_MissionLoader.c no longer declares \
                 `MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024` — schema pin drifted from mod"
                    .to_string(),
            );
        }
        if !loader_src.contains("x-tbd-missionFileMaxBytes") {
            bad.push(
                "TBD_MissionLoader.c comment no longer cites schema \
                 `x-tbd-missionFileMaxBytes` (T-450 cross-pin)"
                    .to_string(),
            );
        }
        for f in sorted_json_files(&missions_dir)? {
            let bytes = fs::metadata(missions_dir.join(&f))
                .with_context(|| format!("stat golden {f}"))?
                .len() as usize;
            if pinned != 0 && bytes > pinned {
                bad.push(format!(
                    "golden {f} is {bytes} B > x-tbd-missionFileMaxBytes={pinned}"
                ));
            }
        }
        // Synthetic: a schema-VALID document padded past the ceiling must exceed the pin.
        // meta.author has no maxLength, so schema alone would accept it — that is exactly
        // the pre-T-450 defect this gate exists to keep closed.
        if pinned != 0 {
            let mut doc = read_json(&missions_dir.join("last-stand-at-montfort.json"))?;
            doc["meta"]["author"] = Value::String("x".repeat(pinned));
            let raw = serde_json::to_vec(&doc)?;
            if raw.len() <= pinned {
                bad.push(format!(
                    "synthetic pad failed to exceed ceiling ({} B ≤ {pinned})",
                    raw.len()
                ));
            } else {
                let schema_errs: Vec<_> = v_mission.iter_errors(&doc).collect();
                if !schema_errs.is_empty() {
                    bad.push(format!(
                        "synthetic oversized doc is schema-invalid ({}); pad a field without \
                         maxLength so this fixture isolates the byte check",
                        schema_errs.len()
                    ));
                }
            }
        }
        if bad.is_empty() {
            println!(
                "  PASS  x-tbd-missionFileMaxBytes={EXPECTED} matches TBD_MissionLoader; \
                 goldens under ceiling; oversized synthetic exceeds pin"
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  mission file byte ceiling");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.36 — kit alias ↔ spawn registry cross-reference ────────────────────────────────
    // The check mission.schema.json structurally cannot do; see the KNOWN_UNRESOLVABLE_KITS
    // header for why a closed enum would be the wrong answer.
    println!("Kit alias registry cross-reference (T-181.36):");
    let (reg_path, reg_aliases) = spawn_registry_aliases(root)?;
    println!(
        "  note  {} alias(es) from {}",
        reg_aliases.len(),
        reg_path
            .strip_prefix(root)
            .unwrap_or(&reg_path)
            .to_string_lossy()
    );
    let allow_kits: HashSet<(&str, &str)> = KNOWN_UNRESOLVABLE_KITS.iter().copied().collect();
    for f in sorted_json_files(&missions_dir)? {
        let doc = read_json(&missions_dir.join(&f))?;
        let bad: Vec<(String, String)> = dangling_kits(&doc, &reg_aliases)
            .into_iter()
            .filter(|(_, k)| !allow_kits.contains(&(f.as_str(), k.as_str())))
            .collect();
        let waived = dangling_kits(&doc, &reg_aliases).len() - bad.len();
        if bad.is_empty() {
            let note = if waived > 0 {
                format!(" ({waived} waived — see KNOWN_UNRESOLVABLE_KITS)")
            } else {
                String::new()
            };
            println!("  PASS  {f}{note}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {f}");
            for (ptr, alias) in &bad {
                println!(
                    "        {ptr} -> '{alias}' is not defined in the spawn registry — \
                     TBD_SpawnManager would fail this slot permanently and the mission is rejected"
                );
            }
        }

        // `preset:` is reported, not failed. Nothing in the mod resolves a preset alias today
        // (TBD_MissionValidator only checks presetId for emptiness, and no spawn path reads it),
        // so failing on it would be enforcing a rule the runtime does not have. It is printed so
        // the debt is visible the moment presets DO become load-bearing.
        let bad_presets: Vec<String> = mission_preset_refs(&doc)
            .into_iter()
            .filter(|(_, p)| !reg_aliases.contains(p))
            .map(|(ptr, p)| format!("{ptr} -> '{p}'"))
            .collect();
        if !bad_presets.is_empty() {
            println!(
                "  note  {f}: {} preset alias(es) not in the registry (not fatal — no mod code \
                 resolves preset: yet): {}",
                bad_presets.len(),
                bad_presets.join(", ")
            );
        }
    }

    // ── T-249 — slot-y golden pins schema 1.2 optional y + Y_ABSENT / HasJsonY path ───────────
    // No other committed golden authors slots[].y, so deleting this file would leave the entire
    // spawn-height branch (TBD_MissionSlotStruct.Y_ABSENT, HasJsonY(), TBD_SpawnManager spawn Y
    // policy) unexercised in CI despite T-092.1 shipping it.
    println!("slot-y golden (T-249):");
    const SLOT_Y_GOLDEN: &str = "slot-y-absent-and-present.json";
    {
        let path = missions_dir.join(SLOT_Y_GOLDEN);
        let mut bad: Vec<String> = Vec::new();
        if !path.is_file() {
            bad.push(format!(
                "{SLOT_Y_GOLDEN} is missing — the only committed golden that exercises \
                 slots[].y present vs absent (TBD_MissionSlotStruct.Y_ABSENT / HasJsonY)"
            ));
        } else {
            let doc = read_json(&path)?;
            if doc["schemaVersion"].as_str() != Some("1.2") {
                bad.push("schemaVersion must be \"1.2\"".to_string());
            }
            let slots = doc["slots"].as_array().cloned().unwrap_or_default();
            let mut with_y = 0usize;
            let mut without_y = 0usize;
            for (i, s) in slots.iter().enumerate() {
                match s.get("y") {
                    None => without_y += 1,
                    Some(v) if v.is_number() => with_y += 1,
                    Some(_) => bad.push(format!("/slots/{i}/y must be a number when present")),
                }
            }
            if with_y == 0 {
                bad.push(
                    "need >=1 slot WITH explicit y (HasJsonY true / jsonY spawn path)".to_string(),
                );
            }
            if without_y == 0 {
                bad.push(
                    "need >=1 slot WITHOUT y (Y_ABSENT sentinel / terrain-surface spawn path)"
                        .to_string(),
                );
            }
        }
        if bad.is_empty() {
            println!("  PASS  {SLOT_Y_GOLDEN}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {SLOT_Y_GOLDEN}");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.36 — kit-aliases.json must mirror the registry it claims to be generated from ──
    // `packages/tbd-schema/registry/kit-aliases.json` is the INVERSE table (ResourceName -> alias)
    // that the mission-compile flatten uses, and its own header says it is generated from the mod
    // registry. Nothing enforced that. A kit added to one and not the other does not error: the
    // flatten silently falls back to the faction default kit, so an authored medic compiles into a
    // rifleman. Two definitions and no enforcement is exactly how they drift.
    println!("kit-aliases.json <-> spawn registry mirror (T-181.36):");
    {
        let ka_path = sroot.join("registry/kit-aliases.json");
        let ka = read_json(&ka_path)?;
        let reg_doc = read_json(&reg_path)?;
        let reg_kits: BTreeMap<String, String> = reg_doc["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let a = e.get("alias").and_then(Value::as_str)?;
                a.starts_with("kit:")
                    .then(|| (a.to_string(), e["guid"].as_str().unwrap_or("").to_string()))
            })
            .collect();
        let ka_kits: BTreeMap<String, String> = ka["kits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|k| {
                Some((
                    k.get("alias").and_then(Value::as_str)?.to_string(),
                    k["resourceName"].as_str().unwrap_or("").to_string(),
                ))
            })
            .collect();
        let mut bad = Vec::new();
        for (alias, guid) in &reg_kits {
            match ka_kits.get(alias) {
                None => bad.push(format!(
                    "{alias} is in the registry but missing from kit-aliases.json — the flatten \
                     would compile it to the faction default kit instead"
                )),
                Some(rn) if rn != guid => bad.push(format!(
                    "{alias} resolves to a different prefab in each file:\n          registry      {guid}\n          kit-aliases   {rn}"
                )),
                Some(_) => {}
            }
        }
        for alias in ka_kits.keys() {
            if !reg_kits.contains_key(alias) {
                bad.push(format!(
                    "{alias} is in kit-aliases.json but not in the registry — a mission compiled \
                     with it would be rejected at boot"
                ));
            }
        }
        // The per-faction fallbacks are what a slot degrades TO, so a dangling one is worse than
        // a dangling kit: it fails silently for every unmapped slot at once.
        for (fk, fv) in ka["factionDefaults"].as_object().into_iter().flatten() {
            for key in ["kit", "preset"] {
                let Some(alias) = fv.get(key).and_then(Value::as_str) else {
                    continue;
                };
                if !reg_aliases.contains(alias) {
                    bad.push(format!(
                        "factionDefaults.{fk}.{key} = '{alias}' does not resolve in the registry — \
                         every slot that falls back to it would fail"
                    ));
                }
            }
        }
        if bad.is_empty() {
            println!("  PASS  kit-aliases.json ({} kits, in sync)", ka_kits.len());
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  kit-aliases.json");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.34 — negative goldens: fixtures the gate is REQUIRED to reject ─────────────────
    // A vocabulary nobody tests is not enforced. Delete the `container` enum from
    // mission.schema.json and every positive golden still passes; these are what notice.
    // Each fixture is a wrapper, not a mission — see golden-missions-invalid/README.md.
    println!("Negative goldens (must FAIL — T-181.34):");
    let neg_dir = sroot.join("golden-missions-invalid");
    for f in sorted_json_files(&neg_dir)? {
        let w = read_json(&neg_dir.join(&f))?;
        let (Some(gate), Some(at), Some(doc)) = (
            w["mustFail"]["gate"].as_str(),
            w["mustFail"]["at"].as_str(),
            w.get("document"),
        ) else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {f} — malformed fixture (need mustFail.gate, mustFail.at, document)");
            continue;
        };

        // A finding "at or below" the declared pointer. Requiring ALL findings to match is what
        // pins the fixture to its reason — a fixture that failed on an unrelated typo elsewhere
        // would otherwise be a false green that outlives the check it was written for.
        let at_or_below = |p: &str| p == at || p.starts_with(&format!("{at}/"));
        let schema_errs: Vec<String> = v_mission
            .iter_errors(doc)
            .map(|e| e.instance_path().to_string())
            .collect();

        let verdict: Result<String, Vec<String>> = match gate {
            "schema" => {
                if schema_errs.is_empty() {
                    Err(vec![
                        "mission.schema.json ACCEPTED it — the check this fixture pins is gone"
                            .to_string(),
                    ])
                } else if let Some(off) = schema_errs
                    .iter()
                    .find(|p| !at_or_below(p))
                    .map(String::as_str)
                {
                    Err(vec![format!(
                        "rejected, but for the wrong reason: error at '{off}', expected '{at}'"
                    )])
                } else {
                    Ok(format!("mission.schema.json rejects {at}"))
                }
            }
            "registry" => {
                let dangling = dangling_kits(doc, &reg_aliases);
                if !schema_errs.is_empty() {
                    // The fixture must isolate the registry check. If the schema also rejects it,
                    // a green here would not prove the cross-reference works.
                    Err(vec![format!(
                        "expected a registry-only failure but mission.schema.json also rejects it at {}",
                        schema_errs.join(", ")
                    )])
                } else if dangling.is_empty() {
                    Err(vec![
                        "the registry cross-reference ACCEPTED it — the check is gone, or the \
                         alias was added to the registry"
                            .to_string(),
                    ])
                } else if let Some((off, _)) =
                    dangling.iter().find(|(p, _)| !at_or_below(p)).cloned()
                {
                    Err(vec![format!(
                        "rejected, but for the wrong reason: dangling alias at '{off}', expected '{at}'"
                    )])
                } else {
                    Ok(format!("registry cross-reference rejects {at}"))
                }
            }
            other => Err(vec![format!(
                "unknown mustFail.gate '{other}' (expected 'schema' or 'registry')"
            )]),
        };

        match verdict {
            Ok(how) => println!("  PASS  {f} — correctly rejected ({how})"),
            Err(why) => {
                failures.set(failures.get() + 1);
                println!("  FAIL  {f} — must fail but did not, or failed wrongly");
                for w in why {
                    println!("        {w}");
                }
            }
        }
    }
    Ok(())
}
