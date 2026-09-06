//! Land-cover region narrowing — port of `parseRegionsPayload`/`narrowRings`
//! (`landCoverRegions.ts:66`/`:51`). Accepts both the shipped `{ regions: [...] }` wrapper and
//! the golden bare-array shape. Structural (**Class S**) vs the TS loader.

use rkyv::Archived;
use serde_json::Value;

use super::binary::archives::{ARCHIVE_SCHEMA_VERSION, ForestRegion, ForestRegionsArchive};
use super::binary::{BinaryError, access_checked};

/// One narrowed land-cover region (mirror of `LandCoverRegion`). `polygon` rings: first outer,
/// rest holes.
#[derive(Clone, Debug, PartialEq)]
pub struct LandCoverRegion {
    pub id: String,
    pub kind: String,
    pub polygon: Vec<Vec<[f64; 2]>>,
    pub tree_count: Option<f64>,
    pub dominant_species_class: Option<String>,
    pub density_per_ha: Option<f64>,
    pub area_ha: Option<f64>,
    pub cover_type: Option<String>,
}

/// `kind ∈ {forest, field, waterBody}` (the N5 taxonomy).
#[must_use]
fn is_kind(v: &str) -> bool {
    v == "forest" || v == "field" || v == "waterBody"
}

/// `narrowRings(polygon)` (`:51`) — a non-empty array of rings; each ring `len ≥ 3` of finite
/// `[x, y]` points. Returns `None` (drops the region) on any violation.
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

/// `parseRegionsPayload(raw)` (`:66`). Keeps a row iff `id` is a string, `kind` is a valid kind,
/// and `narrow_rings` succeeds. Accepts a bare array or `{ regions: [...] }`.
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

/* ─────────────────── T-935.11 — the rkyv forest regions ─────────────────── */

/// The alignment an `objects/forest-regions.rkyv` buffer must sit on before [`access_checked`]
/// will look at it — 16, what `to_bytes`' `AlignedVec` writes with, proved at compile time below
/// to cover the archived type's own alignment.
pub const FOREST_REGIONS_ALIGN: usize = 16;

const _: () = assert!(
    FOREST_REGIONS_ALIGN.is_multiple_of(align_of::<Archived<ForestRegionsArchive>>()),
    "FOREST_REGIONS_ALIGN must be a multiple of the archived type's own alignment"
);

/// A region with no `treeCount` on the wire. `u32::MAX` rather than `0`, because `0` is a legal
/// (if odd) census and a field whose absence and whose zero are the same byte cannot be narrowed
/// back to the `Option` the parser produces.
const NO_TREE_COUNT: u32 = u32::MAX;

/// One narrowed [`LandCoverRegion`] → the wire region.
///
/// The absent-optional encoding is the same idea as the prefab catalogue's
/// ([`super::prefab::row_to_archive`]): NaN for a missing number, `""` for a missing string,
/// [`NO_TREE_COUNT`] for a missing census. Both directions live in this file so they cannot drift
/// apart across the crate boundary — `tbd-tools`' emitter calls this rather than rolling its own.
///
/// # Errors
/// [`BinaryError::Archive`] when the region cannot be encoded without changing its meaning: an
/// empty `id` or a present-but-empty optional string (both would decode as absent), a
/// `treeCount` that is not a `u32`-representable whole number, a non-finite number, a `kind`
/// outside the N5 taxonomy, or a polygon that violates `narrow_rings` (no rings, or a ring under
/// three vertices — a degenerate ring reaches the landcover mesh compose, not an error message).
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

/// `narrow_rings`' structural invariant, stated once for both directions. Takes ring *lengths*
/// rather than the rings, because the two callers hold different types for the same shape — a
/// `Vec<Vec<[f64; 2]>>` on the parser side and an `ArchivedVec<ArchivedVec<[f32_le; 2]>>` on the
/// wire side — and the invariant is about neither of them.
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

/// `Archived<ForestRegionsArchive>` → the very regions [`parse_regions_payload`] yields from
/// `forest-regions.json.gz`, in file order.
///
/// # Errors
/// [`BinaryError::Archive`] when a region carries a `kind` outside the taxonomy or a polygon that
/// `narrow_rings` would have dropped. Both are refusals rather than skips: the JSON parser drops
/// such a row *before* anything sees it, so a binary reader that passed one through would put a
/// shape on the map that the JSON path provably never draws.
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
///
/// The buffer is copied once into an aligned one because `fs::read`/`fetch` hand back a `Vec<u8>`
/// that is only 1-aligned by contract; after that nothing is deserialised — [`from_archive`] reads
/// the validated archive in place.
///
/// # Does this archive need the world it was built for? No, and here is the argument
///
/// The wave-240 rule (an answer about the world must carry the world it was built for) exists
/// because the water *raster* answers "is this tile water?" by indexing pixels through an extent
/// that arrives in a separate manifest field: a stale `worldBounds` there silently relocates every
/// answer. A region ring is the opposite shape — its vertices are absolute world metres, so
/// nothing outside the file participates in placing them, and there is no second field whose
/// staleness could move them. There is therefore no extent to carry.
///
/// What the file does depend on is which terrain it was exported from, and
/// [`ForestRegionsArchive`] has no field for that — unlike
/// [`PrefabCatalogArchive`](super::binary::archives::PrefabCatalogArchive), which carries
/// `type_inventory.terrain_id` and where [`catalog_from_bytes`](super::prefab::catalog_from_bytes)
/// refuses a caller that disagrees. So this file's terrain identity rests entirely on the URL it
/// was fetched from (`/map-assets/{terrain}/objects/forest-regions.rkyv`). Closing that gap means
/// adding a `terrain_id` to `ForestRegionsArchive` in `binary/archives.rs`, which T-935.11 does
/// not own; it is reported rather than widened silently.
///
/// # Errors
/// * [`BinaryError::Misaligned`] — the aligned copy could not be placed.
/// * [`BinaryError::Archive`] — rkyv validation rejected the buffer, or a region is one the JSON
///   parser would have dropped (see [`from_archive`]).
/// * [`BinaryError::UnsupportedVersion`] — a well-formed archive written by a different schema.
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

/// `raw` copied into a buffer whose byte at `pad` sits on [`FOREST_REGIONS_ALIGN`]. Capacity is
/// reserved up front so the `extend_from_slice` cannot reallocate and move the buffer out from
/// under the offset that was just measured.
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
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;

    fn golden(name: &str) -> Value {
        let path = format!(
            "{}/../../packages/tbd-schema/golden/map-objects/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_slice(&fs::read(&path).expect("read golden")).expect("parse golden")
    }

    #[test]
    fn parses_bare_array_golden() {
        let regions = parse_regions_payload(&golden("map-object-regions-everon-sample.json"));
        assert_eq!(regions.len(), 4);
        assert_eq!(regions[0].id, "forest-everon-001");
        assert_eq!(regions[0].kind, "forest");
        assert_eq!(regions[0].tree_count, Some(12400.0));
        assert_eq!(regions[0].polygon.len(), 1); // one outer ring
        assert_eq!(regions[0].polygon[0].len(), 5); // closed square (5 verts)
        assert_eq!(regions[3].kind, "waterBody");
    }

    #[test]
    fn accepts_wrapped_and_drops_malformed() {
        let raw = json!({ "regions": [
            { "id": "f1", "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
            { "id": "bad-kind", "kind": "swamp", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
            { "id": "short-ring", "kind": "field", "polygon": [[[0, 0], [1, 0]]] },
            { "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] }
        ]});
        let regions = parse_regions_payload(&raw);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].id, "f1");
    }

    #[test]
    fn non_payload_is_empty() {
        assert_eq!(parse_regions_payload(&Value::Null).len(), 0);
        assert_eq!(parse_regions_payload(&json!("<html>")).len(), 0);
    }

    /* ───────────────── T-935.11 — the rkyv forest regions ───────────────── */

    use super::super::binary::to_bytes;
    use crate::world::bytes_to_json;
    use std::path::PathBuf;

    /// The committed everon export ships 36 land-cover regions (`store.rs`'s census pin says so
    /// too). Re-pin deliberately if the export changes.
    const EVERON_REGIONS: usize = 36;
    /// Polygon vertices summed over the island; a floor, not an equality, so an empty or
    /// LFS-pointered tree cannot pass while a legitimate re-export can.
    const EVERON_VERTEX_FLOOR: usize = 2_000;

    fn everon_json_regions() -> Vec<LandCoverRegion> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon/objects/forest-regions.json.gz");
        let raw = fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        parse_regions_payload(&bytes_to_json(&raw).expect("regions decode"))
    }

    fn to_archive(regions: &[LandCoverRegion]) -> ForestRegionsArchive {
        ForestRegionsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            regions: regions
                .iter()
                .enumerate()
                .map(|(i, r)| region_to_archive(i, r).expect("region → archive"))
                .collect(),
        }
    }

    /// THE SLICE'S PIN. Emit the everon regions from the committed JSON, read them back through
    /// the shipped entry point, and prove they are the very regions `parse_regions_payload`
    /// yields — vertex by vertex, in order.
    ///
    /// The oracle is the JSON set narrowed to f32 (`f64::from(v as f32)`), because that is the
    /// archive's actual contract; `everon_rings_stay_within_f32_of_the_json_metres` below is what
    /// keeps that projection from blessing an emitter that moved every coordinate.
    #[test]
    fn everon_regions_archive_equals_the_json_regions() {
        let json = everon_json_regions();
        assert_eq!(
            json.len(),
            EVERON_REGIONS,
            "everon region corpus changed; re-pin EVERON_REGIONS deliberately"
        );

        let bytes = to_bytes(&to_archive(&json)).expect("serialise");
        assert!(
            bytes_to_json(&bytes).is_err(),
            "the archive must not be parseable as JSON, or this test has not proven the rkyv \
             route ran"
        );
        assert_ne!(&bytes[..2], &[0x1f, 0x8b], "must not carry gzip magic");

        let back = regions_from_bytes(&bytes).expect("read back");
        assert_eq!(back.len(), json.len());
        #[allow(clippy::cast_possible_truncation)]
        let f32_narrow = |v: Option<f64>| v.map(|n| f64::from(n as f32));
        let mut vertices = 0usize;
        let mut kinds = std::collections::BTreeSet::new();
        for (i, (a, j)) in back.iter().zip(json.iter()).enumerate() {
            assert_eq!(a.id, j.id, "region {i}: id");
            assert_eq!(a.kind, j.kind, "region {i}: kind");
            assert_eq!(
                a.dominant_species_class, j.dominant_species_class,
                "region {i}: dominantSpeciesClass"
            );
            assert_eq!(a.cover_type, j.cover_type, "region {i}: coverType");
            assert_eq!(a.tree_count, j.tree_count, "region {i}: treeCount");
            for (name, got, want) in [
                (
                    "densityPerHa",
                    a.density_per_ha,
                    f32_narrow(j.density_per_ha),
                ),
                ("areaHa", a.area_ha, f32_narrow(j.area_ha)),
            ] {
                assert_eq!(
                    got.map(f64::to_bits),
                    want.map(f64::to_bits),
                    "region {i} ({}): {name}",
                    j.id
                );
            }
            assert_eq!(a.polygon.len(), j.polygon.len(), "region {i}: ring count");
            for (k, (ra, rj)) in a.polygon.iter().zip(j.polygon.iter()).enumerate() {
                assert_eq!(ra.len(), rj.len(), "region {i} ring {k}: vertex count");
                for (v, (pa, pj)) in ra.iter().zip(rj.iter()).enumerate() {
                    for axis in 0..2 {
                        #[allow(clippy::cast_possible_truncation)]
                        let want = f64::from(pj[axis] as f32);
                        assert_eq!(
                            pa[axis].to_bits(),
                            want.to_bits(),
                            "region {i} ({}) ring {k} vertex {v} axis {axis}",
                            j.id
                        );
                    }
                }
                vertices += ra.len();
            }
            kinds.insert(a.kind.clone());
        }
        assert!(
            vertices >= EVERON_VERTEX_FLOOR,
            "only {vertices} ring vertices compared — the corpus is empty or LFS-pointered, so \
             this test proved nothing"
        );
        assert!(!kinds.is_empty(), "no kinds seen: {kinds:?}");
    }

    /// The f32 projection has to be a *narrowing*, not a change of answer. Everon is 12.8 km wide
    /// and f32 carries ~7 significant digits, so no region vertex may move by as much as a
    /// millimetre — without this, the parity test above would bless an emitter that halved every
    /// coordinate, because both sides would agree on the wrong number.
    #[test]
    fn everon_rings_stay_within_f32_of_the_json_metres() {
        let json = everon_json_regions();
        let archive = to_archive(&json);
        let mut worst = 0.0_f64;
        for (a, j) in archive.regions.iter().zip(json.iter()) {
            for (ra, rj) in a.polygon.iter().zip(j.polygon.iter()) {
                for (pa, pj) in ra.iter().zip(rj.iter()) {
                    worst = worst
                        .max((f64::from(pa[0]) - pj[0]).abs())
                        .max((f64::from(pa[1]) - pj[1]).abs());
                }
            }
        }
        assert!(worst < 1e-3, "worst vertex drift {worst} m (>= 1 mm)");
    }

    /// Rule 16 — a well-formed archive written by a different schema is refused; `access_checked`
    /// cannot catch this because the layout is legal and only the meaning moved.
    #[test]
    fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
        let mut archive = to_archive(&everon_json_regions());
        archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
        let bytes = to_bytes(&archive).expect("serialise");
        assert!(access_checked::<ForestRegionsArchive>(&bytes).is_ok());
        assert!(matches!(
            regions_from_bytes(&bytes),
            Err(BinaryError::UnsupportedVersion { .. })
        ));
    }

    /// A row the JSON parser would have dropped must not survive the binary route either: an
    /// unknown kind and a two-vertex ring are both refusals, not shapes on the map.
    #[test]
    fn rows_the_json_parser_would_drop_are_refused_on_the_binary_route() {
        let ok = ForestRegion {
            id: "f1".into(),
            kind: "forest".into(),
            polygon: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
            tree_count: 4,
            dominant_species_class: String::new(),
            density_per_ha: f32::NAN,
            area_ha: f32::NAN,
            cover_type: String::new(),
        };
        let one = |r: ForestRegion| ForestRegionsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            regions: vec![r],
        };
        assert_eq!(
            regions_from_bytes(&to_bytes(&one(ok.clone())).expect("serialise"))
                .expect("control")
                .len(),
            1
        );
        for (needle, r) in [
            (
                "taxonomy does not know",
                ForestRegion {
                    kind: "swamp".into(),
                    ..ok.clone()
                },
            ),
            (
                "at least 3",
                ForestRegion {
                    polygon: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                    ..ok.clone()
                },
            ),
            (
                "no rings",
                ForestRegion {
                    polygon: Vec::new(),
                    ..ok.clone()
                },
            ),
        ] {
            let bytes = to_bytes(&one(r)).expect("serialise");
            assert!(
                access_checked::<ForestRegionsArchive>(&bytes).is_ok(),
                "the bytes must validate, or this proves nothing about the meaning check"
            );
            let msg = regions_from_bytes(&bytes)
                .expect_err("must refuse")
                .to_string();
            assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
        }
    }

    /// Absent and present survive as themselves, including a `treeCount` of 0 (which is a census,
    /// not an absence) — and the writer refuses the values it could not encode honestly.
    #[test]
    fn absent_and_present_optionals_round_trip() {
        let regions = vec![
            LandCoverRegion {
                id: "full".into(),
                kind: "field".into(),
                polygon: vec![vec![[0.0, 0.0], [8.0, 0.0], [8.0, 8.0]]],
                tree_count: Some(0.0),
                dominant_species_class: Some("conifer".into()),
                density_per_ha: Some(12.5),
                area_ha: Some(3.25),
                cover_type: Some("grass".into()),
            },
            LandCoverRegion {
                id: "bare".into(),
                kind: "waterBody".into(),
                polygon: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
                tree_count: None,
                dominant_species_class: None,
                density_per_ha: None,
                area_ha: None,
                cover_type: None,
            },
        ];
        let bytes = to_bytes(&to_archive(&regions)).expect("serialise");
        let back = regions_from_bytes(&bytes).expect("read back");
        assert_eq!(back, regions, "every optional must survive as itself");
        assert_eq!(back[0].tree_count, Some(0.0), "a real 0 is not \"absent\"");
        assert!(back[1].tree_count.is_none() && back[1].cover_type.is_none());

        for (needle, r) in [
            (
                "empty id",
                LandCoverRegion {
                    id: String::new(),
                    ..regions[0].clone()
                },
            ),
            (
                "empty coverType",
                LandCoverRegion {
                    cover_type: Some(String::new()),
                    ..regions[0].clone()
                },
            ),
            (
                "not a u32-representable whole number",
                LandCoverRegion {
                    tree_count: Some(1.5),
                    ..regions[0].clone()
                },
            ),
            (
                "outside the {forest, field, waterBody}",
                LandCoverRegion {
                    kind: "swamp".into(),
                    ..regions[0].clone()
                },
            ),
        ] {
            let msg = region_to_archive(0, &r)
                .expect_err("must refuse")
                .to_string();
            assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
        }
    }

    /// Nothing malformed yields a partial region set.
    #[test]
    fn malformed_buffers_error_rather_than_yielding_regions() {
        let bytes = to_bytes(&to_archive(&everon_json_regions())).expect("serialise");
        for (what, buf) in [
            ("empty", &[][..]),
            ("truncated tail", &bytes[..bytes.len() - 1]),
            ("shifted head", &bytes[1..]),
            ("json", br#"{"regions":[]}"#),
        ] {
            assert!(regions_from_bytes(buf).is_err(), "{what} was accepted");
        }
    }
}
