use super::*;

pub(super) fn validate(
    reg_file: &dyn Fn(&str) -> PathBuf,
    catalog_file: &dyn Fn(&str) -> PathBuf,
    check: &dyn Fn(&str, &jsonschema::Validator, &Value),
    failures: &std::cell::Cell<usize>,
    v_registry: &jsonschema::Validator,
    v_items: &jsonschema::Validator,
    v_compat: &jsonschema::Validator,
) -> Result<()> {
    println!("Registry:");
    check(
        "registry.example.json",
        v_registry,
        &read_json(&reg_file("registry.example.json"))?,
    );
    check(
        "registry.vanilla-poc.json",
        v_registry,
        &read_json(&reg_file("registry.vanilla-poc.json"))?,
    );

    println!("Registry items:");
    let items_sample = read_json(&reg_file("registry-items.sample.json"))?;
    let items_wb = read_json(&catalog_file("registry-items.workbench.json"))?;
    check("registry-items.sample.json", v_items, &items_sample);
    check("registry-items.workbench.json", v_items, &items_wb);

    // Addon provenance + variant_of integrity (FK walkers).
    let fk = |label: String, ok: bool, pass_note: String, bad: Vec<String>| {
        if ok {
            println!("  PASS  {label} ({pass_note})");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for b in bad.iter().take(10) {
                println!("        {b}");
            }
            if bad.len() > 10 {
                println!("        ... {} more", bad.len() - 10);
            }
        }
    };
    let addon_refs = |items: &Value| -> (usize, usize, Vec<String>) {
        let known: HashSet<&str> = items["addons"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
            .unwrap_or_default();
        let mut with_addon = 0;
        let mut bad = Vec::new();
        let total = items["items"].as_array().map(Vec::len).unwrap_or(0);
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(addon) = it.get("addon").and_then(Value::as_str) else {
                continue;
            };
            with_addon += 1;
            if !known.contains(addon) {
                bad.push(format!(
                    "dangling {} addon {addon}",
                    it["resource_name"].as_str().unwrap_or("?")
                ));
            }
        }
        (with_addon, total, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (with_addon, total, bad) = addon_refs(items);
        fk(
            format!("{label} (addon provenance"),
            bad.is_empty(),
            format!("addon provenance, {with_addon}/{total} items carry addon"),
            bad,
        );
    }
    let variant_refs = |items: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut variants = 0;
        let mut bad = Vec::new();
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(vof) = it.get("variant_of").and_then(Value::as_str) else {
                continue;
            };
            variants += 1;
            let rn = it["resource_name"].as_str().unwrap_or("?");
            if !known.contains(vof) {
                bad.push(format!("{rn} variant_of {vof}"));
            }
            if vof == rn {
                bad.push(format!("{rn} is its own variant"));
            }
        }
        (variants, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (variants, bad) = variant_refs(items);
        fk(
            format!("{label} (variant_of integrity"),
            bad.is_empty(),
            format!("variant_of integrity, {variants} variants"),
            bad,
        );
    }

    println!("Registry compat:");
    let edge_refs = |items: &Value, compat: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut bad = Vec::new();
        let edges = compat["edges"].as_array().map(Vec::len).unwrap_or(0);
        for e in compat["edges"].as_array().into_iter().flatten() {
            let et = e["edge_type"].as_str().unwrap_or("?");
            for endpoint in ["from_node", "to_node"] {
                if let Some(n) = e[endpoint].as_str()
                    && !known.contains(n)
                {
                    bad.push(format!("dangling {et} {endpoint} {n}"));
                }
            }
        }
        (edges, bad)
    };
    let compat_sample = read_json(&reg_file("registry-compat.sample.json"))?;
    check("registry-compat.sample.json", v_compat, &compat_sample);
    let (edges, bad) = edge_refs(&items_sample, &compat_sample);
    fk(
        "registry-compat.sample.json vs registry-items.sample.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );
    let compat_wb = read_json(&catalog_file("registry-compat.workbench.json"))?;
    check("registry-compat.workbench.json", v_compat, &compat_wb);
    let (edges, bad) = edge_refs(&items_wb, &compat_wb);
    fk(
        "registry-compat.workbench.json vs registry-items.workbench.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );
    Ok(())
}
