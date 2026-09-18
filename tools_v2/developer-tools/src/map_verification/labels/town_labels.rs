use super::*;

pub fn town_labels(root: &Path, terrain: &str, deck_zoom: f64) -> Result<u8> {
    use website_map_engine::overlay::symbology::labels::importance::LocationLabel;
    use website_map_engine::overlay::symbology::labels::importance::declutter_town_labels;
    use website_map_engine::overlay::symbology::labels::importance::town_declutter_invariant_holds;
    use website_map_engine::overlay::symbology::labels::importance::town_label_fade_alpha;
    let loc_path = root
        .join("packages/map-assets")
        .join(terrain)
        .join("locations.json");
    if !loc_path.exists() {
        eprintln!(
            "verify-town-labels: missing {} (run T-152.6)",
            loc_path.display()
        );
        return Ok(1);
    }
    let raw = fs::read_to_string(&loc_path)?;
    let all: Vec<LocationLabel> = serde_json::from_str(&raw).context("locations.json parse")?;
    let mut failures = 0usize;
    let fail = |msgs: &mut Vec<String>, m: String| msgs.push(m);
    let mut fails: Vec<String> = Vec::new();
    println!("verify-town-labels ({terrain} @ z={deck_zoom}):");

    let drawn = declutter_town_labels(&all, deck_zoom);

    // G2 — required towns ⊆ drawn (normalized-name membership).
    let drawn_names: std::collections::HashSet<String> =
        drawn.iter().map(|l| norm_name(&l.name)).collect();
    let missing: Vec<&str> = REQUIRED_EVERON_TOWNS
        .iter()
        .copied()
        .filter(|t| !drawn_names.contains(&norm_name(t)))
        .collect();
    if missing.is_empty() {
        println!(
            "  PASS  G2 REQUIRED_EVERON_TOWNS ({}) ⊆ drawn @ z={deck_zoom}",
            REQUIRED_EVERON_TOWNS.len()
        );
    } else {
        fail(
            &mut fails,
            format!(
                "G2 required towns missing from drawn: [{}]",
                missing.join(", ")
            ),
        );
    }

    // G3 — declutter invariant (the core A3 predicate).
    if town_declutter_invariant_holds(&drawn, &all, deck_zoom) {
        println!("  PASS  G3 declutter invariant (A3 predicate, core oracle)");
    } else {
        fail(&mut fails, "G3 town_declutter_invariant_holds".to_string());
    }

    // G4 — provenance: every drawn (id, name) exists in the source rows.
    let src: std::collections::HashSet<(&str, &str)> = all
        .iter()
        .map(|l| (l.id.as_str(), l.name.as_str()))
        .collect();
    if drawn
        .iter()
        .all(|l| src.contains(&(l.id.as_str(), l.name.as_str())))
    {
        println!("  PASS  G4 name provenance = locations.json[id]");
    } else {
        fail(
            &mut fails,
            "G4 drawn label not present in source locations.json".to_string(),
        );
    }

    // G5 — empty source → 0 drawn.
    if declutter_town_labels(&[], deck_zoom).is_empty() {
        println!("  PASS  G5 empty source → |drawn|=0");
    } else {
        fail(&mut fails, "G5 empty source drew labels".to_string());
    }

    // G1 — kind hygiene (settlement lane only).
    let allowed = ["town", "village", "airport", "locality"];
    let excluded = ["peak", "hill", "natural"];
    let kind_of = |l: &LocationLabel| l.kind.clone().unwrap_or_else(|| "town".into());
    let unknown = drawn
        .iter()
        .filter(|l| !allowed.contains(&kind_of(l).as_str()))
        .count();
    let excl = drawn
        .iter()
        .filter(|l| excluded.contains(&kind_of(l).as_str()))
        .count();
    if unknown == 0 && excl == 0 {
        println!(
            "  PASS  G1 kind hygiene: {} drawn ⊆ {{town,village,airport,locality}}; 0 peak/hill/natural @ z={deck_zoom}",
            drawn.len()
        );
    } else {
        fail(
            &mut fails,
            format!("G1 kind hygiene: {excl} excluded + {unknown} unknown kind drawn"),
        );
    }

    // G4 fade endpoints (α 1.0 → 0.5 → 0.0 over z ∈ [2.0, 3.0]).
    let fa = town_label_fade_alpha;
    let approx = |a: f64, b: f64| (a - b).abs() < 1e-6;
    if approx(fa(2.0), 1.0) && approx(fa(2.5), 0.5) && approx(fa(3.0), 0.0) {
        println!(
            "  PASS  G4 fade α: 2.0→{} 2.5→{} 3.0→{}",
            fa(2.0),
            fa(2.5),
            fa(3.0)
        );
    } else {
        fail(
            &mut fails,
            format!(
                "G4 fade endpoints wrong: α(2.0)={} α(2.5)={} α(3.0)={}",
                fa(2.0),
                fa(2.5),
                fa(3.0)
            ),
        );
    }

    // G4 band edges — nothing drawn above the fade ceiling / below the widened floor.
    let above = declutter_town_labels(&all, 3.1).len();
    let below = declutter_town_labels(&all, -4.6).len();
    if above == 0 && below == 0 {
        println!(
            "  PASS  G4 band edges: |drawn|=0 @ z=3.1 (above ceiling) and z=−4.6 (below floor)"
        );
    } else {
        fail(
            &mut fails,
            format!("G4 band edges: {above} drawn @ z=3.1, {below} drawn @ z=−4.6"),
        );
    }

    println!(
        "  NOTE  GPU pack checks retired with the wasm render surface (Leptos lane gated by the editor smokes)"
    );

    for m in &fails {
        failures += 1;
        println!("  FAIL  {m}");
    }
    if failures > 0 {
        eprintln!("\nverify-town-labels: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-town-labels: OK");
        Ok(0)
    }
}

pub fn road_names(root: &Path, terrain: &str, deck_zoom: f64) -> Result<u8> {
    use website_map_engine::world::environment::locations::route_geometry::perpendicular_dist_to_polyline;
    use website_map_engine::world::environment::locations::route_geometry::road_declutter_min_dist_m;
    use website_map_engine::world::environment::locations::route_labels::parse_road_names_json;
    use website_map_engine::world::environment::locations::route_placement::ROAD_NAME_MAX_ON_SCREEN;
    use website_map_engine::world::environment::locations::route_placement::ROAD_NAME_PERP_TOL_M;
    use website_map_engine::world::environment::locations::route_placement::declutter_road_labels;
    use website_map_engine::world::environment::locations::route_placement::place_road_labels;
    use website_map_engine::world::environment::locations::route_placement::road_declutter_invariant_holds;
    let base = root.join("packages/map-assets").join(terrain);
    let names_path = base.join("road-names.json");
    let roads_path = base.join("objects/roads.json.gz");
    for (p, hint) in [(&names_path, ""), (&roads_path, "")] {
        if !p.exists() {
            eprintln!("verify-road-names: missing {}{hint}", p.display());
            return Ok(1);
        }
    }
    let names_raw = fs::read_to_string(&names_path)?;
    let names =
        parse_road_names_json(&names_raw).map_err(|e| anyhow::anyhow!("road-names: {e}"))?;
    let gz = fs::read(&roads_path)?;
    let mut store = website_map_engine::streaming::loaders::store::WorldStore::new();
    let seg_count = store
        .load_roads_gz(&gz)
        .map_err(|e| anyhow::anyhow!("roads.json.gz: {e}"))?;
    let _ = seg_count;
    let mut failures = 0usize;
    let mut fails: Vec<String> = Vec::new();
    println!("verify-road-names ({terrain} @ z={deck_zoom}):");

    let drawn = declutter_road_labels(
        &place_road_labels(&names, &store.roads, deck_zoom),
        deck_zoom,
    );

    // G3 — major roads ⊆ drawn.
    let drawn_names: std::collections::HashSet<&str> =
        drawn.iter().map(|l| l.name.as_str()).collect();
    let missing: Vec<&str> = MAJOR_EVERON_ROADS
        .iter()
        .copied()
        .filter(|r| !drawn_names.contains(r))
        .collect();
    if missing.is_empty() {
        println!(
            "  PASS  G3 MAJOR_EVERON_ROADS ({}) ⊆ drawn @ z={deck_zoom}",
            MAJOR_EVERON_ROADS.len()
        );
    } else {
        fails.push(format!(
            "G3 major roads missing from drawn: [{}]",
            missing.join(", ")
        ));
    }

    // G4 — name length ≥ 2 on every drawn row.
    if drawn.iter().all(|l| l.name.trim().chars().count() >= 2) {
        println!("  PASS  G4 name.length ≥ 2");
    } else {
        fails.push("G4 drawn label with name shorter than 2".to_string());
    }

    // G5 — placement within perpendicular tolerance of its own segment.
    let by_id: std::collections::HashMap<
        &str,
        &website_map_engine::world::terrain::roads::network::RoadSegment,
    > = store.roads.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut perp_bad = 0usize;
    for l in &drawn {
        match by_id.get(l.segment_id.as_str()) {
            Some(seg) => {
                let d = perpendicular_dist_to_polyline(&seg.points, l.x, l.y);
                if d > ROAD_NAME_PERP_TOL_M {
                    perp_bad += 1;
                }
            }
            None => perp_bad += 1,
        }
    }
    if perp_bad == 0 {
        println!("  PASS  G5 placement ≤ {ROAD_NAME_PERP_TOL_M} m perpendicular");
    } else {
        fails.push(format!(
            "G5 {perp_bad} drawn labels beyond {ROAD_NAME_PERP_TOL_M} m perpendicular"
        ));
    }

    // G6 — declutter invariant (core oracle) + min-dist restated.
    if road_declutter_invariant_holds(&drawn, deck_zoom) {
        println!(
            "  PASS  G6 declutter dist ≥ {:.0} m (core oracle)",
            road_declutter_min_dist_m(deck_zoom)
        );
    } else {
        fails.push("G6 road_declutter_invariant_holds".to_string());
    }

    // G7 — cap.
    if drawn.len() <= ROAD_NAME_MAX_ON_SCREEN {
        println!(
            "  PASS  G7 |drawn|={} ≤ {ROAD_NAME_MAX_ON_SCREEN}",
            drawn.len()
        );
    } else {
        fails.push(format!(
            "G7 |drawn|={} > {ROAD_NAME_MAX_ON_SCREEN}",
            drawn.len()
        ));
    }

    // Toggle-off oracle — empty names → 0 drawn.
    let empty = parse_road_names_json(r#"{"schemaVersion":"1.0","terrainId":"everon","roads":[]}"#)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if declutter_road_labels(
        &place_road_labels(&empty, &store.roads, deck_zoom),
        deck_zoom,
    )
    .is_empty()
    {
        println!("  PASS  toggle off oracle: empty names → |drawn|=0");
    } else {
        fails.push("toggle-off: empty names drew labels".to_string());
    }

    println!(
        "  NOTE  GPU pack checks retired with the wasm render surface (Leptos lane gated by the editor smokes)"
    );

    for m in &fails {
        failures += 1;
        println!("  FAIL  {m}");
    }
    if failures > 0 {
        eprintln!("\nverify-road-names: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-road-names: OK");
        Ok(0)
    }
}

/// Decode a 16-bit grayscale PNG into a u16 raster (channel 0). Mirrors the pngjs
/// `{ skipRescale: true }` read + `rasterFromPngjs` channel extraction.
pub(super) fn decode_u16_gray_png(bytes: &[u8]) -> Result<(Vec<u16>, usize, usize)> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().context("png read_info")?;
    let info = reader.info();
    anyhow::ensure!(
        info.bit_depth == png::BitDepth::Sixteen,
        "DEM must be 16-bit PNG; got depth={:?}",
        info.bit_depth
    );
    anyhow::ensure!(
        matches!(
            info.color_type,
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
        ),
        "DEM must be grayscale; colorType={:?}",
        info.color_type
    );
    let channels = info.color_type.samples();
    let (w, h) = (info.width as usize, info.height as usize);
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).context("png frame")?;
    let data = &buf[..frame.buffer_size()];
    let mut raster = vec![0u16; w * h];
    for (i, px) in raster.iter_mut().enumerate() {
        let off = i * channels * 2;
        *px = u16::from_be_bytes([data[off], data[off + 1]]);
    }
    Ok((raster, w, h))
}

/// JS `Number.prototype.toFixed(3)` semantics: ties round away from zero on the
/// magnitude (ECMA picks the larger n for |x|), unlike Rust's `{:.3}` half-to-even.
/// Anchor files carry exact dyadic ties (0.0625, -18.3125) where the two differ.
pub(super) fn js_fixed3(x: f64) -> String {
    let n = (x.abs() * 1000.0).round() as i64;
    let sign = if x.is_sign_negative() && (n != 0 || x < 0.0) {
        "-"
    } else {
        ""
    };
    format!("{sign}{}.{:03}", n / 1000, n % 1000)
}
