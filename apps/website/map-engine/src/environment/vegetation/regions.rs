//! Role: regions.
//! Position: `environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use rkyv::Archived;
use serde_json::Value;

use crate::formats::archives::codec::BinaryError;
use crate::formats::archives::codec::access_checked;
use crate::formats::archives::forest::ForestRegion;
use crate::formats::archives::forest::ForestRegionsArchive;
use crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION;

/// One narrowed land-cover region (mirror of `LandCoverRegion`). `polygon` rings: first outer, rest holes.
#[derive(Clone, Debug, PartialEq)]
pub struct LandCoverRegion {
    /// Id.
    pub id: String,

    /// Kind.
    pub kind: String,

    /// Polygon.
    pub polygon: Vec<Vec<[f64; 2]>>,

    /// Tree count.
    pub tree_count: Option<f64>,

    /// Dominant species class.
    pub dominant_species_class: Option<String>,

    /// Density per ha.
    pub density_per_ha: Option<f64>,

    /// Area ha.
    pub area_ha: Option<f64>,

    /// Cover type.
    pub cover_type: Option<String>,
}

#[must_use]
fn is_kind(v: &str) -> bool {
    v == "forest" || v == "field" || v == "waterBody"
}

#[must_use]
fn narrow_rings(polygon: &Value) -> Option<Vec<Vec<[f64; 2]>>> {
    let arr = polygon.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut rings = Vec::with_capacity(arr.len());
    for ring in arr {
        let ra = ring.as_array()?;
        if ra.len() < 3 {
            return None;
        }
        let mut pts = Vec::with_capacity(ra.len());
        for p in ra {
            let pa = p.as_array()?;
            if pa.len() < 2 {
                return None;
            }
            let x = pa[0].as_f64().filter(|n| n.is_finite())?;
            let y = pa[1].as_f64().filter(|n| n.is_finite())?;
            pts.push([x, y]);
        }
        rings.push(pts);
    }
    Some(rings)
}

/// `parseRegionsPayload(raw)` (`:66`). Keeps a row iff `id` is a string, `kind` is a valid kind, and `narrow_rings` succeeds. Accepts a bare array or `{ regions: [...] }`.
#[must_use]
pub fn parse_regions_payload(raw: &Value) -> Vec<LandCoverRegion> {
    let rows = if raw.is_array() {
        raw.as_array()
    } else {
        raw.get("regions").and_then(Value::as_array)
    };
    let Some(rows) = rows else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(kind) = row.get("kind").and_then(Value::as_str) else {
            continue;
        };
        if !is_kind(kind) {
            continue;
        }
        let Some(polygon) = row.get("polygon").and_then(narrow_rings) else {
            continue;
        };
        out.push(LandCoverRegion {
            id: id.to_string(),
            kind: kind.to_string(),
            polygon,
            tree_count: row.get("treeCount").and_then(Value::as_f64),
            dominant_species_class: row
                .get("dominantSpeciesClass")
                .and_then(Value::as_str)
                .map(str::to_string),
            density_per_ha: row.get("densityPerHa").and_then(Value::as_f64),
            area_ha: row.get("areaHa").and_then(Value::as_f64),
            cover_type: row
                .get("coverType")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    out
}

/// Canonical forest regions align value.
pub const FOREST_REGIONS_ALIGN: usize = 16;

const _: () = assert!(
    FOREST_REGIONS_ALIGN.is_multiple_of(align_of::<Archived<ForestRegionsArchive>>()),
    "FOREST_REGIONS_ALIGN must be a multiple of the archived type's own alignment"
);

const NO_TREE_COUNT: u32 = u32::MAX;

/// One narrowed [`LandCoverRegion`] → the wire region.
pub fn region_to_archive(i: usize, r: &LandCoverRegion) -> Result<ForestRegion, BinaryError> {
    let bad = |cause: String| BinaryError::Archive {
        what: "ForestRegionsArchive",
        cause,
    };
    if r.id.is_empty() {
        return Err(bad(format!("region {i} has an empty id")));
    }
    if !is_kind(&r.kind) {
        return Err(bad(format!(
            "region {i} ({}) has kind {:?}, which is outside the {{forest, field, waterBody}} \
             taxonomy the loader narrows to",
            r.id, r.kind
        )));
    }
    check_rings(i, &r.id, r.polygon.iter().map(Vec::len)).map_err(bad)?;
    let text = |name: &str, v: Option<&String>| match v.map(String::as_str) {
        Some("") => Err(bad(format!(
            "region {i} ({}) carries an empty {name}; the wire row spells \"absent\" as the empty \
             string, so writing it would decode back as None",
            r.id
        ))),
        other => Ok(other.unwrap_or_default().to_string()),
    };
    for (name, v) in [
        ("densityPerHa", r.density_per_ha),
        ("areaHa", r.area_ha),
        ("treeCount", r.tree_count),
    ] {
        if v.is_some_and(|n| !n.is_finite()) {
            return Err(bad(format!(
                "region {i} ({}) has a non-finite {name}",
                r.id
            )));
        }
    }
    let tree_count = match r.tree_count {
        None => NO_TREE_COUNT,
        Some(n) if n.fract() == 0.0 && (0.0..f64::from(NO_TREE_COUNT)).contains(&n) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                n as u32
            }
        }
        Some(n) => {
            return Err(bad(format!(
                "region {i} ({}) has treeCount {n}, which is not a u32-representable whole number \
                 below the absent sentinel",
                r.id
            )));
        }
    };
    #[allow(clippy::cast_possible_truncation)]
    Ok(ForestRegion {
        id: r.id.clone(),
        kind: r.kind.clone(),
        polygon: r
            .polygon
            .iter()
            .map(|ring| ring.iter().map(|p| [p[0] as f32, p[1] as f32]).collect())
            .collect(),
        tree_count,
        dominant_species_class: text("dominantSpeciesClass", r.dominant_species_class.as_ref())?,
        density_per_ha: r.density_per_ha.map_or(f32::NAN, |v| v as f32),
        area_ha: r.area_ha.map_or(f32::NAN, |v| v as f32),
        cover_type: text("coverType", r.cover_type.as_ref())?,
    })
}

fn check_rings<I: IntoIterator<Item = usize>>(
    i: usize,
    id: &str,
    ring_lens: I,
) -> Result<(), String> {
    let mut rings = 0usize;
    for (k, n) in ring_lens.into_iter().enumerate() {
        rings += 1;
        if n < 3 {
            return Err(format!(
                "region {i} ({id}) ring {k} has {n} vertices; narrow_rings requires at least 3"
            ));
        }
    }
    if rings == 0 {
        return Err(format!("region {i} ({id}) has no rings"));
    }
    Ok(())
}

/// `Archived<ForestRegionsArchive>` → the very regions [`parse_regions_payload`] yields from `forest-regions.json.gz`, in file order.
pub fn from_archive(
    archive: &Archived<ForestRegionsArchive>,
) -> Result<Vec<LandCoverRegion>, BinaryError> {
    let bad = |cause: String| BinaryError::Archive {
        what: "ForestRegionsArchive",
        cause,
    };
    let mut out = Vec::with_capacity(archive.regions.len());
    for (i, r) in archive.regions.iter().enumerate() {
        let (id, kind) = (r.id.as_str(), r.kind.as_str());
        if !is_kind(kind) {
            return Err(bad(format!(
                "region {i} ({id}) carries kind {kind:?}, which the loader's taxonomy does not \
                 know — the JSON path would have dropped this row"
            )));
        }
        check_rings(i, id, r.polygon.iter().map(|ring| ring.len())).map_err(bad)?;
        let text = |s: &str| (!s.is_empty()).then(|| s.to_string());
        let num = |v: f32| (!v.is_nan()).then(|| f64::from(v));
        out.push(LandCoverRegion {
            id: id.to_string(),
            kind: kind.to_string(),
            polygon: r
                .polygon
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|p| [f64::from(p[0].to_native()), f64::from(p[1].to_native())])
                        .collect()
                })
                .collect(),
            tree_count: match r.tree_count.to_native() {
                NO_TREE_COUNT => None,
                n => Some(f64::from(n)),
            },
            dominant_species_class: text(r.dominant_species_class.as_str()),
            density_per_ha: num(r.density_per_ha.to_native()),
            area_ha: num(r.area_ha.to_native()),
            cover_type: text(r.cover_type.as_str()),
        });
    }
    Ok(out)
}

/// One `objects/forest-regions.rkyv` file → the in-memory land-cover set, JSON-free.
pub fn regions_from_bytes(raw: &[u8]) -> Result<Vec<LandCoverRegion>, BinaryError> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "ForestRegionsArchive",
        align: FOREST_REGIONS_ALIGN,
    })?;
    let archive = access_checked::<ForestRegionsArchive>(&buf[pad..])?;
    let version = archive.schema_version.to_native();
    if version != ARCHIVE_SCHEMA_VERSION {
        return Err(BinaryError::UnsupportedVersion {
            what: "ForestRegionsArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual: version,
        });
    }
    from_archive(archive)
}

fn aligned_copy(raw: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buf: Vec<u8> = Vec::with_capacity(raw.len() + FOREST_REGIONS_ALIGN);
    let pad = buf.as_ptr().align_offset(FOREST_REGIONS_ALIGN);
    if pad >= FOREST_REGIONS_ALIGN {
        return None;
    }
    buf.resize(pad, 0);
    buf.extend_from_slice(raw);
    (buf[pad..].as_ptr().align_offset(FOREST_REGIONS_ALIGN) == 0).then_some((buf, pad))
}

#[cfg(test)]
#[path = "tests/regions_tests.rs"]
mod tests;
