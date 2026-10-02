//! Role: zones.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    HashMap, ModCircle, ModZone, ModZoneShape, SPAWN_ZONE_RADIUS_M, ShapeIn, ZoneIn, slug_key,
    terrain_bounds,
};

/// One-decimal metre rounding — matches spawn-zone synthesis and the historical TS flatten.
pub(super) fn round_coord(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Shape from input using the supplied domain data.
pub(super) fn shape_from_input(shape: &ShapeIn) -> Option<ModZoneShape> {
    let has_polygon = shape.polygon.as_ref().is_some_and(|p| p.len() >= 3);
    let has_circle = shape.circle.as_ref().is_some_and(|c| c.r > 0.0);

    if has_polygon {
        let mut ring = Vec::new();
        for pair in shape.polygon.as_ref().expect("has_polygon") {
            if pair.len() == 2 {
                ring.push([round_coord(pair[0]), round_coord(pair[1])]);
            }
        }
        if ring.len() >= 3 {
            return Some(ModZoneShape::Polygon { polygon: ring });
        }
    }

    if has_circle {
        let c = shape.circle.as_ref().expect("has_circle");
        return Some(ModZoneShape::Circle {
            circle: ModCircle {
                x: round_coord(c.x),
                z: round_coord(c.z),
                r: round_coord(c.r),
            },
        });
    }

    None
}

/// Flatten authored zone using the supplied domain data.
pub(super) fn flatten_authored_zone(raw: &ZoneIn) -> Option<ModZone> {
    let shape = raw.shape.as_ref().and_then(shape_from_input)?;
    let id = if raw.id.is_empty() {
        return None;
    } else {
        raw.id.clone()
    };
    let kind = if raw.kind.is_empty() {
        return None;
    } else {
        raw.kind.clone()
    };
    let faction = if raw.faction.is_empty() {
        String::new()
    } else {
        slug_key(&raw.faction, "faction")
    };
    Some(ModZone {
        id,
        kind,
        label: raw.label.clone(),
        faction,
        shape,
        rules: raw.rules.clone(),
    })
}

/// Synthesize spawn zones using the supplied domain data.
pub(super) fn synthesize_spawn_zones(
    centroid_order: &[String],
    centroids: &HashMap<String, (f64, f64, i64)>,
) -> Vec<ModZone> {
    let mut zones = Vec::new();
    for faction_key in centroid_order {
        let (sx, sz, n) = centroids[faction_key];
        let nf = n as f64;
        zones.push(ModZone {
            id: format!("z_spawn_{faction_key}"),
            kind: "spawn".to_string(),
            faction: faction_key.clone(),
            label: String::new(),
            shape: ModZoneShape::Circle {
                circle: ModCircle {
                    x: round_coord(sx / nf),
                    z: round_coord(sz / nf),
                    r: SPAWN_ZONE_RADIUS_M,
                },
            },
            rules: None,
        });
    }
    zones
}

/// Synthesize terrain boundary using the supplied domain data.
pub(super) fn synthesize_terrain_boundary(terrain_key: &str) -> ModZone {
    let [min_x, min_z, max_x, max_z] = terrain_bounds(terrain_key);
    ModZone {
        id: "z_bounds".to_string(),
        kind: "boundary".to_string(),
        label: String::new(),
        faction: String::new(),
        shape: ModZoneShape::Polygon {
            polygon: vec![
                [min_x, min_z],
                [max_x, min_z],
                [max_x, max_z],
                [min_x, max_z],
            ],
        },
        rules: None,
    }
}

/// Zones have boundary using the supplied domain data.
pub(super) fn zones_have_boundary(zones: &[ModZone]) -> bool {
    zones.iter().any(|z| z.kind == "boundary")
}

/// Merge authored payload zones, per-faction spawn circles, and a terrain boundary fallback.
pub(super) fn derive_zones(
    authored: &[ZoneIn],
    centroid_order: &[String],
    centroids: &HashMap<String, (f64, f64, i64)>,
    terrain_key: &str,
) -> Vec<ModZone> {
    let mut zones: Vec<ModZone> = authored.iter().filter_map(flatten_authored_zone).collect();
    zones.extend(synthesize_spawn_zones(centroid_order, centroids));
    if !zones_have_boundary(&zones) {
        zones.push(synthesize_terrain_boundary(terrain_key));
    }
    zones
}
