use super::fixtures::{edge, item, picks};
use super::*;

#[test]
fn rows_cover_all_14_keys_incl_edge_rows() {
    assert_eq!(LOADOUT_ROWS.len(), 14);
    // optic + magazine are edge rows immediately after primary.
    assert_eq!(LOADOUT_ROWS[0].key, "primary");
    assert!(matches!(
        LOADOUT_ROWS[1].source,
        RowSource::Edge {
            edge: "optic_on_weapon",
            depends_on: "primary"
        }
    ));
    assert!(matches!(
        LOADOUT_ROWS[2].source,
        RowSource::Edge {
            edge: "mag_in_weapon",
            depends_on: "primary"
        }
    ));
    // rail order ≠ loadout order (vest pulled up before pants).
    let rail: Vec<&str> = RAIL_REGIONS.iter().map(|r| r.key).collect();
    let vest_i = rail.iter().position(|k| *k == "vest").unwrap();
    let pants_i = rail.iter().position(|k| *k == "pants").unwrap();
    assert!(vest_i < pants_i);
    // doll excludes optic/magazine.
    assert_eq!(DOLL_REGIONS.len(), 12);
    assert!(!DOLL_REGIONS
        .iter()
        .any(|r| r.key == "optic" || r.key == "magazine"));
}

#[test]
fn items_for_returns_counterpart_both_directions() {
    let g = CompatGraph::from_edges(&[
        edge("weap_m4", "optic_acog", "optic_on_weapon"),
        edge("mag_stanag", "weap_m4", "mag_in_weapon"),
    ]);
    assert_eq!(
        g.items_for("weap_m4", "optic_on_weapon"),
        vec!["optic_acog"]
    );
    assert_eq!(g.items_for("weap_m4", "mag_in_weapon"), vec!["mag_stanag"]);
    assert!(g.accepts("weap_m4", "optic_acog", "optic_on_weapon"));
    assert!(!g.accepts("weap_m4", "optic_eotech", "optic_on_weapon"));
    assert!(g.items_for("weap_ak", "optic_on_weapon").is_empty());
}

#[test]
fn optic_row_options_filtered_by_edges_and_current_preserved() {
    let items = vec![
        item("optic_acog", "ACOG", "gear_optic"),
        item("optic_eotech", "EOTech", "gear_optic"),
    ];
    let idx = index_by_name(&items);
    let g = CompatGraph::from_edges(&[edge("weap_m4", "optic_acog", "optic_on_weapon")]);
    let optic_row = row("optic").unwrap();

    // primary picked → only the compatible ACOG offered.
    let p = picks(&[("primary", "weap_m4")]);
    let opts = row_options(optic_row, "", &p, &items, &idx, Some(&g));
    assert_eq!(
        opts.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
        vec!["optic_acog"]
    );

    // an incompatible live pick stays visible, flagged.
    let opts = row_options(optic_row, "optic_eotech", &p, &items, &idx, Some(&g));
    assert!(opts
        .iter()
        .any(|o| o.value == "optic_eotech" && o.incompatible));

    // no primary → no options.
    assert!(row_options(optic_row, "", &HashMap::new(), &items, &idx, Some(&g)).is_empty());
}

#[test]
fn kind_row_excludes_abstract_and_variants() {
    let mut base = item("rifle_base", "Rifle (base)", "gear_primary");
    base.r#abstract = Some(true);
    let mut variant = item("rifle_camo", "Rifle (camo)", "gear_primary");
    variant.variant_of = Some("rifle_m16".into());
    let items = vec![item("rifle_m16", "M16", "gear_primary"), base, variant];
    let idx = index_by_name(&items);
    let opts = row_options(
        row("primary").unwrap(),
        "",
        &HashMap::new(),
        &items,
        &idx,
        None,
    );
    assert_eq!(
        opts.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
        vec!["rifle_m16"]
    );
}

#[test]
fn validation_flags_stranded_and_orphan_edges() {
    let g = CompatGraph::from_edges(&[edge("weap_m4", "optic_acog", "optic_on_weapon")]);
    // valid: compatible optic on its weapon.
    let ok = picks(&[("primary", "weap_m4"), ("optic", "optic_acog")]);
    assert!(validate_loadout(&ok, Some(&g), CompatStatus::Ready).is_empty());
    // optic with no primary → "Requires a Primary pick".
    let orphan = picks(&[("optic", "optic_acog")]);
    let e = validate_loadout(&orphan, Some(&g), CompatStatus::Ready);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].key, "optic");
    // incompatible optic → rejected.
    let bad = picks(&[("primary", "weap_m4"), ("optic", "optic_eotech")]);
    assert_eq!(
        validate_loadout(&bad, Some(&g), CompatStatus::Ready).len(),
        1
    );
    // unavailable feed → no edge validation.
    assert!(validate_loadout(&bad, Some(&g), CompatStatus::Unavailable).is_empty());
}

#[test]
fn weight_is_honest_about_unknowns() {
    let mut m4 = item("weap_m4", "M4", "gear_primary");
    m4.weight_kg = Some(3.4);
    let helmet = item("helm", "Helmet", "gear_helmet"); // no weight
    let items = vec![m4, helmet];
    let idx = index_by_name(&items);
    let p = picks(&[("primary", "weap_m4"), ("headCover", "helm")]);
    let w = loadout_weight(&p, &idx);
    assert_eq!(w.item_count, 2);
    assert_eq!(w.unknown_count, 1);
    assert!((w.known_kg - 3.4).abs() < 1e-9);
    assert_eq!(
        format_loadout_weight(&w),
        "≥ 3.4 kg · 1 item without weight data"
    );

    // all known → plain readout.
    let p2 = picks(&[("primary", "weap_m4")]);
    assert_eq!(
        format_loadout_weight(&loadout_weight(&p2, &idx)),
        "3.4 kg · 1 item"
    );
}
