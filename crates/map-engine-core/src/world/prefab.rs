//! Prefab-row narrowing + prefab map build — ports of `narrowPrefabRows`/`narrowSpatial`/
//! `narrowRender` (`worldObjectsCore.ts:291`) and `buildPrefabMaps` (`:381`). The map value
//! (`code` + row) is what a chunk's per-instance class lookup reads; `has_oversized` is the
//! §6 oversized-ring flag.

use std::collections::HashMap;

use rkyv::Archived;
use serde_json::Value;

use super::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, KindCensus, PrefabCatalogArchive, PrefabEntry as PrefabEntryArchive,
    TypeInventory,
};
use super::binary::{BinaryError, access_checked};
use super::classify::{NO_CLASS, OVERSIZED_HALF_EXTENT_M, class_code, render_class_for_prefab};

/// Clone-safe prefab row subset (mirror of `WorldPrefabRow`). `prefab_id` keeps full f64
/// precision (the join key). Spatial half-extents / render glyph fields are carried for W3.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefabRow {
    pub prefab_id: f64,
    pub kind: String,
    pub class: String,
    pub label: Option<String>,
    pub resource_name: Option<String>,
    pub half_x: Option<f64>,
    pub half_y: Option<f64>,
    pub half_z: Option<f64>,
    pub height_m: Option<f64>,
    pub icon_key: Option<String>,
    pub base_size_px: Option<f64>,
    pub default_color: Option<String>,
    pub importance_zoom: Option<f64>,
}

/// `buildPrefabMaps` byId value: prefabId → render-class code + the narrowed row.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabEntry {
    pub code: u8,
    pub row: PrefabRow,
}

/// A JSON value that is a number → `Some(f64)`, mirroring `typeof v === 'number'`.
#[must_use]
fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// A JSON value that is a string → owned `String`, mirroring `typeof v === 'string'`.
#[must_use]
fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

/// `narrowPrefabRows(raw)` (`:291`) — keep rows with a numeric `prefabId` and string `kind`;
/// `class` falls back to `"unknown"`. Spatial + render blocks narrowed field-by-field.
#[must_use]
pub fn narrow_prefab_rows(raw: &Value) -> Vec<PrefabRow> {
    let Some(rows) = raw.get("prefabs").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(prefab_id) = r.get("prefabId").and_then(Value::as_f64) else {
            continue;
        };
        let Some(kind) = r.get("kind").and_then(Value::as_str) else {
            continue;
        };
        let class = r
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let spatial = r.get("spatial");
        let he = spatial.and_then(|s| s.get("halfExtentsM"));
        let render = r.get("render");

        out.push(PrefabRow {
            prefab_id,
            kind: kind.to_string(),
            class,
            label: text(r.get("label")),
            resource_name: text(r.get("resourceName")),
            half_x: num(he.and_then(|h| h.get("x"))),
            half_y: num(he.and_then(|h| h.get("y"))),
            half_z: num(he.and_then(|h| h.get("z"))),
            height_m: num(spatial.and_then(|s| s.get("heightM"))),
            icon_key: text(render.and_then(|r| r.get("iconKey"))),
            base_size_px: num(render.and_then(|r| r.get("baseSizePx"))),
            default_color: text(render.and_then(|r| r.get("defaultColor"))),
            importance_zoom: num(render.and_then(|r| r.get("importanceZoom"))),
        });
    }
    out
}

/// `buildPrefabMaps(prefabRows)` (`:381`) → (prefabId→{code,row}, has_oversized). The map is
/// keyed by `prefab_id.to_bits()` so the chunk lookup matches JS `Map<number,…>` bit-for-bit
/// (both sides parse the same integer to the same f64). `has_oversized` mirrors
/// `cls && Math.max(hx, hy) >= 64` with `hx/hy` defaulting to 0.
#[must_use]
pub fn build_prefab_maps(rows: Vec<PrefabRow>) -> (HashMap<u64, PrefabEntry>, bool) {
    let mut by_id = HashMap::with_capacity(rows.len());
    let mut has_oversized = false;
    for row in rows {
        let cls = render_class_for_prefab(&row.kind, &row.class);
        let code = cls.map_or(NO_CLASS, class_code);
        let hx = row.half_x.unwrap_or(0.0);
        let hy = row.half_y.unwrap_or(0.0);
        if cls.is_some() && hx.max(hy) >= OVERSIZED_HALF_EXTENT_M {
            has_oversized = true;
        }
        by_id.insert(row.prefab_id.to_bits(), PrefabEntry { code, row });
    }
    (by_id, has_oversized)
}

/* ─────────────────── T-935.11 — the rkyv prefab catalogue ─────────────────── */

/// The alignment an `objects/prefabs.rkyv` buffer must sit on before [`access_checked`] will look
/// at it. Same number and same reasoning as [`ROAD_NETWORK_ALIGN`](super::ROAD_NETWORK_ALIGN):
/// 16 is what [`to_bytes`](super::binary::to_bytes)' `AlignedVec` writes with, and the `const`
/// below *proves* it covers the archived type's own alignment at compile time rather than
/// trusting the number.
pub const PREFAB_CATALOG_ALIGN: usize = 16;

const _: () = assert!(
    PREFAB_CATALOG_ALIGN.is_multiple_of(align_of::<Archived<PrefabCatalogArchive>>()),
    "PREFAB_CATALOG_ALIGN must be a multiple of the archived type's own alignment"
);

/// # The absent-optional encoding, which is the whole reason this file owns both directions
///
/// [`PrefabRow`] is a narrowing of JSON, so eight of its fields are `Option`: a prefab with no
/// `render` block has **no** `iconKey`, which is not the same fact as "its `iconKey` is the empty
/// string". The wire row ([`PrefabEntryArchive`]) has no `Option`s — it is a flat f32/String
/// record — so the two directions have to agree on a sentinel, and if they are written in
/// different crates they will eventually disagree. So they are here, next to each other, and
/// `tbd-tools`' emitter calls [`row_to_archive`] rather than rolling its own:
///
/// | field kind | absent on the wire | why that sentinel is safe |
/// |---|---|---|
/// | numeric (`half_*`, `height_m`, `base_size_px`, `importance_zoom`) | `NaN` | JSON cannot express NaN, so `serde_json` never parses one — a NaN in the file can only mean "absent" |
/// | text (`label`, `resource_name`, `icon_key`) | `""` | [`row_to_archive`] **refuses** a present-but-empty string, so `""` is unambiguous |
/// | `default_color` | alpha `0` | a parsed `#rrggbb` is always written with alpha `255` |
///
/// This is deliberately NOT the "zero when the export has no spatial block" wording in
/// `archives.rs`'s doc comment for `half_extents`: zero is a *legal half-extent*, so it cannot
/// also mean absent without making `from_archive` disagree with the JSON parse on a real row.
/// Everon has no zero half-extents today, which is exactly why the ambiguity would have gone
/// unnoticed.
///
/// `Some(v)` → `v as f32`; the round trip is `f64::from(v as f32)`, not the identity — the wire
/// row is the compact f32 twin of the parser struct (`archives.rs` module docs), and the everon
/// parity pin states the projection that way rather than comparing `==` and hoping.
#[must_use]
fn num_to_wire(v: Option<f64>) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    v.map_or(f32::NAN, |n| n as f32)
}

/// The inverse of [`num_to_wire`]: NaN is absent, everything else is a value.
#[must_use]
fn wire_to_num(v: f32) -> Option<f64> {
    (!v.is_nan()).then(|| f64::from(v))
}

/// `#rrggbb` → `[r, g, b, 255]`. `None` for anything else — including uppercase hex and an
/// `#rrggbbaa`, because [`wire_to_color`] can only re-emit the lowercase 6-digit form and a
/// writer that silently mangled the string would break the round trip in a way no test on
/// everon's 84 coloured rows would catch.
#[must_use]
fn color_to_wire(s: &str) -> Option<[u8; 4]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    #[allow(clippy::cast_possible_truncation)]
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 0xff])
}

/// The inverse of [`color_to_wire`]: alpha `0` is absent.
#[must_use]
fn wire_to_color(c: [u8; 4]) -> Option<String> {
    (c[3] != 0).then(|| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
}

/// A present string that would encode as the absent sentinel, or a non-finite number: refuse.
fn wire_text(field: &'static str, i: usize, v: Option<&String>) -> Result<String, BinaryError> {
    match v.map(String::as_str) {
        Some("") => Err(BinaryError::Archive {
            what: "PrefabCatalogArchive",
            cause: format!(
                "prefab row {i} carries an empty {field}; the wire row spells \"absent\" as the \
                 empty string, so writing it would decode back as None"
            ),
        }),
        other => Ok(other.unwrap_or_default().to_string()),
    }
}

/// One narrowed [`PrefabRow`] → the wire row, class code included.
///
/// The code is computed *here*, from [`render_class_for_prefab`] + [`class_code`], so the byte in
/// the file is always this build's answer — and [`rows_from_archive`] re-derives it and refuses a
/// file that disagrees. That is what makes the byte load-bearing rather than decoration: a writer
/// built against a renumbered `RENDER_CLASS_CODES` produces a file this build **rejects** instead
/// of one it draws with every class shifted by one.
///
/// # Errors
/// [`BinaryError::Archive`] when the row cannot be encoded without changing its meaning: a
/// `prefab_id` that is not a `u32`-representable whole number (it is the join key and the wire
/// field is `u32`), a present-but-empty string, a non-finite number, or a `default_color` that is
/// not `#rrggbb`. Every one of these is fatal at write time rather than a silent narrowing,
/// because the emitter is the last place that can see the JSON the row came from.
pub fn row_to_archive(i: usize, row: &PrefabRow) -> Result<PrefabEntryArchive, BinaryError> {
    let bad = |cause: String| BinaryError::Archive {
        what: "PrefabCatalogArchive",
        cause,
    };
    if !row.prefab_id.is_finite()
        || row.prefab_id.fract() != 0.0
        || row.prefab_id < 0.0
        || row.prefab_id > f64::from(u32::MAX)
    {
        return Err(bad(format!(
            "prefab row {i} has prefabId {}, which is not a u32-representable whole number — the \
             wire catalogue is addressed by a u32 join key",
            row.prefab_id
        )));
    }
    for (name, v) in [
        ("halfExtentsM.x", row.half_x),
        ("halfExtentsM.y", row.half_y),
        ("halfExtentsM.z", row.half_z),
        ("heightM", row.height_m),
        ("baseSizePx", row.base_size_px),
        ("importanceZoom", row.importance_zoom),
    ] {
        if v.is_some_and(|n| !n.is_finite()) {
            return Err(bad(format!(
                "prefab row {i} has a non-finite {name}; NaN is the wire's \"absent\" sentinel, so \
                 writing it would decode back as None"
            )));
        }
    }
    let default_color = match row.default_color.as_deref() {
        None => [0, 0, 0, 0],
        Some(s) => color_to_wire(s).ok_or_else(|| {
            bad(format!(
                "prefab row {i} has defaultColor {s:?}, which is not a lowercase #rrggbb — the \
                 wire row stores four bytes and could not re-emit this string"
            ))
        })?,
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(PrefabEntryArchive {
        prefab_id: row.prefab_id as u32,
        kind: row.kind.clone(),
        class: row.class.clone(),
        class_code: render_class_for_prefab(&row.kind, &row.class).map_or(NO_CLASS, class_code),
        label: wire_text("label", i, row.label.as_ref())?,
        resource_name: wire_text("resourceName", i, row.resource_name.as_ref())?,
        half_extents: [
            num_to_wire(row.half_x),
            num_to_wire(row.half_y),
            num_to_wire(row.half_z),
        ],
        height_m: num_to_wire(row.height_m),
        icon_key: wire_text("iconKey", i, row.icon_key.as_ref())?,
        base_size_px: num_to_wire(row.base_size_px),
        default_color,
        importance_zoom: num_to_wire(row.importance_zoom),
    })
}

/// `Archived<PrefabCatalogArchive>` → the very rows [`narrow_prefab_rows`] yields from
/// `prefabs.json.gz`, in file order.
///
/// # Errors
/// [`BinaryError::Archive`] when a row's stored `class_code` is not what this build computes from
/// its `kind`/`class`. See [`row_to_archive`] for why that is fatal.
pub fn rows_from_archive(
    archive: &Archived<PrefabCatalogArchive>,
) -> Result<Vec<PrefabRow>, BinaryError> {
    let mut out = Vec::with_capacity(archive.prefabs.len());
    for (i, p) in archive.prefabs.iter().enumerate() {
        let (kind, class) = (p.kind.as_str(), p.class.as_str());
        let want = render_class_for_prefab(kind, class).map_or(NO_CLASS, class_code);
        if p.class_code != want {
            return Err(BinaryError::Archive {
                what: "PrefabCatalogArchive",
                cause: format!(
                    "prefab row {i} (prefabId {}, {kind}/{class}) carries class code {}, but this \
                     build's RENDER_CLASS_CODES table says {want} — the archive was written \
                     against a different table and every instance of this prefab would draw as \
                     the wrong class",
                    p.prefab_id.to_native(),
                    p.class_code
                ),
            });
        }
        let text = |s: &str| (!s.is_empty()).then(|| s.to_string());
        out.push(PrefabRow {
            prefab_id: f64::from(p.prefab_id.to_native()),
            kind: kind.to_string(),
            class: class.to_string(),
            label: text(p.label.as_str()),
            resource_name: text(p.resource_name.as_str()),
            half_x: wire_to_num(p.half_extents[0].to_native()),
            half_y: wire_to_num(p.half_extents[1].to_native()),
            half_z: wire_to_num(p.half_extents[2].to_native()),
            height_m: wire_to_num(p.height_m.to_native()),
            icon_key: text(p.icon_key.as_str()),
            base_size_px: wire_to_num(p.base_size_px.to_native()),
            default_color: wire_to_color(p.default_color),
            importance_zoom: wire_to_num(p.importance_zoom.to_native()),
        });
    }
    Ok(out)
}

/// What one `objects/prefabs.rkyv` decodes to: the same pair [`build_prefab_maps`] returns, plus
/// the census the catalogue carries about itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefabCatalog {
    /// `prefabId.to_bits()` → render-class code + narrowed row.
    pub by_id: HashMap<u64, PrefabEntry>,
    pub has_oversized: bool,
    /// The terrain this catalogue was built for — checked against the caller's, see
    /// [`catalog_from_bytes`].
    pub terrain_id: String,
    pub unique_prefabs: u32,
    pub total_instances: u64,
}

/// `Archived<PrefabCatalogArchive>` → `(map, has_oversized)`, exactly as
/// `build_prefab_maps(narrow_prefab_rows(json))` would.
///
/// # Errors
/// From [`rows_from_archive`].
pub fn from_archive(
    archive: &Archived<PrefabCatalogArchive>,
) -> Result<(HashMap<u64, PrefabEntry>, bool), BinaryError> {
    Ok(build_prefab_maps(rows_from_archive(archive)?))
}

/// One `objects/prefabs.rkyv` file → the prefab lookup, JSON-free.
///
/// # `terrain` is not decoration — it is rule 18
///
/// A catalogue is a *catalogue*, so it does not answer spatial questions: `half_extents` are a
/// prefab's own dimensions, not a place, and the only coordinate anywhere near this file is the
/// join key. So it needs no world extent. It does, however, describe **one export of one
/// terrain**, and it already carries which one (`type_inventory.terrain_id`, written by
/// `build-world-objects` from the same `terrainId` the JSON has). Loading everon's catalogue for
/// arland would key every chunk's class lookup off the wrong table — silently, because every id
/// would still be *found*. So the caller must state the terrain it thinks it is loading and a
/// disagreement is refused, not logged.
///
/// The second refusal is the same idea one level down: `type_inventory.unique_prefabs` is the
/// count the census was computed over, so a catalogue whose row vector is a different length is a
/// half-written or stale file whose instance totals describe rows it does not contain.
///
/// # Errors
/// * [`BinaryError::Misaligned`] — the aligned copy could not be placed.
/// * [`BinaryError::Archive`] — rkyv validation rejected the buffer, the terrain or the census
///   disagrees, or a row's class code does not match this build's table.
/// * [`BinaryError::UnsupportedVersion`] — a well-formed archive written by a different schema.
///   `access_checked` cannot catch this: the layout is legal, the *meaning* is not.
pub fn catalog_from_bytes(raw: &[u8], terrain: &str) -> Result<PrefabCatalog, BinaryError> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "PrefabCatalogArchive",
        align: PREFAB_CATALOG_ALIGN,
    })?;
    let archive = access_checked::<PrefabCatalogArchive>(&buf[pad..])?;
    let version = archive.schema_version.to_native();
    if version != ARCHIVE_SCHEMA_VERSION {
        return Err(BinaryError::UnsupportedVersion {
            what: "PrefabCatalogArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual: version,
        });
    }
    let terrain_id = archive.type_inventory.terrain_id.as_str();
    if terrain_id != terrain {
        return Err(BinaryError::Archive {
            what: "PrefabCatalogArchive",
            cause: format!(
                "catalogue was built for terrain {terrain_id:?} but the caller is loading \
                 {terrain:?} — every prefab id would resolve against the wrong table"
            ),
        });
    }
    let unique_prefabs = archive.type_inventory.unique_prefabs.to_native();
    if unique_prefabs as usize != archive.prefabs.len() {
        return Err(BinaryError::Archive {
            what: "PrefabCatalogArchive",
            cause: format!(
                "census declares {unique_prefabs} unique prefabs but the catalogue carries {} \
                 rows — the file is stale or half-written",
                archive.prefabs.len()
            ),
        });
    }
    let (by_id, has_oversized) = from_archive(archive)?;
    Ok(PrefabCatalog {
        by_id,
        has_oversized,
        terrain_id: terrain_id.to_string(),
        unique_prefabs,
        total_instances: archive.type_inventory.total_instances.to_native(),
    })
}

/// `type-inventory.json` → the wire census that rides inside [`PrefabCatalogArchive`] and also
/// ships standalone as `objects/type-inventory.rkyv`.
///
/// `by_kind` keeps the emitted key order (`serde_json` is built with `preserve_order` in every
/// crate that touches this file), which is `INSTANCE_KINDS` order with `road` last — the order
/// every committed inventory has.
///
/// # Errors
/// [`BinaryError::Archive`] when a required field is missing or is not the type the census
/// contract says. Every one of them is fatal rather than defaulted: a census silently defaulting
/// `totalInstances` to 0 is exactly the "reports success over an input it never examined" shape
/// the binary lane exists to remove.
pub fn inventory_to_archive(doc: &Value) -> Result<TypeInventory, BinaryError> {
    let bad = |cause: String| BinaryError::Archive {
        what: "TypeInventory",
        cause,
    };
    let text = |k: &str| {
        doc.get(k)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| bad(format!("type-inventory.json has no string {k}")))
    };
    let levels = doc
        .get("levels")
        .ok_or_else(|| bad("type-inventory.json has no levels block".to_string()))?;
    let count = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_u64)
            .ok_or_else(|| bad(format!("type-inventory.json levels.{k} is not a count")))
    };
    let unique = count(levels, "uniquePrefabs")?;
    let unique_prefabs = u32::try_from(unique)
        .map_err(|_| bad(format!("levels.uniquePrefabs {unique} does not fit a u32")))?;
    let by_kind_raw = doc
        .get("byKind")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("type-inventory.json has no byKind object".to_string()))?;
    let mut by_kind = Vec::with_capacity(by_kind_raw.len());
    for (kind, row) in by_kind_raw {
        let types = count(row, "prefabTypes")?;
        by_kind.push(KindCensus {
            kind: kind.clone(),
            prefab_types: u32::try_from(types).map_err(|_| {
                bad(format!(
                    "byKind.{kind}.prefabTypes {types} does not fit a u32"
                ))
            })?,
            instances: count(row, "instances")?,
        });
    }
    Ok(TypeInventory {
        terrain_id: text("terrainId")?,
        census_status: text("censusStatus")?,
        unique_prefabs,
        total_instances: count(levels, "totalInstances")?,
        by_kind,
    })
}

/// One `objects/type-inventory.rkyv` file → the owned census.
///
/// # This one has no version stamp, and that is a gap, not an oversight
///
/// Every other Tier-2 archive carries `schema_version` as its first field; [`TypeInventory`] does
/// not have the field at all (`archives.rs`), so the standalone census file cannot say which
/// contract wrote it. The copy inside [`PrefabCatalogArchive`] *is* covered, by that archive's own
/// `schema_version`, which is why [`catalog_from_bytes`] is the checked route and this is the
/// convenience one. Adding the field is an `archives.rs` change (T-935.11 does not own it).
///
/// # Errors
/// [`BinaryError::Misaligned`] / [`BinaryError::Archive`], as [`catalog_from_bytes`].
pub fn inventory_from_bytes(raw: &[u8]) -> Result<TypeInventory, BinaryError> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "TypeInventory",
        align: PREFAB_CATALOG_ALIGN,
    })?;
    let archive = access_checked::<TypeInventory>(&buf[pad..])?;
    rkyv::deserialize::<TypeInventory, rkyv::rancor::Error>(archive).map_err(|cause| {
        BinaryError::Archive {
            what: "TypeInventory",
            cause: cause.to_string(),
        }
    })
}

/// `raw` copied into a buffer whose byte at `pad` sits on [`PREFAB_CATALOG_ALIGN`].
///
/// Capacity is reserved up front so the `extend_from_slice` cannot reallocate and move the
/// buffer out from under the offset that was just measured. (Same helper, same reasoning, as
/// `roads.rs`'s; they are separate because either format's alignment may move independently.)
fn aligned_copy(raw: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buf: Vec<u8> = Vec::with_capacity(raw.len() + PREFAB_CATALOG_ALIGN);
    let pad = buf.as_ptr().align_offset(PREFAB_CATALOG_ALIGN);
    if pad >= PREFAB_CATALOG_ALIGN {
        return None;
    }
    buf.resize(pad, 0);
    buf.extend_from_slice(raw);
    (buf[pad..].as_ptr().align_offset(PREFAB_CATALOG_ALIGN) == 0).then_some((buf, pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn narrow_keeps_wellformed_rows() {
        let raw = json!({
            "prefabs": [
                { "prefabId": 0, "kind": "tree", "class": "conifer",
                  "spatial": { "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 }, "heightM": 12 },
                  "render": { "iconKey": "tree-conifer", "baseSizePx": 18, "defaultColor": "#2d5a27" } },
                { "prefabId": "bad", "kind": "tree" },          // non-numeric id → dropped
                { "prefabId": 5 },                                // missing kind → dropped
                { "prefabId": 9, "kind": "building" }             // class defaults to "unknown"
            ]
        });
        let rows = narrow_prefab_rows(&raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].prefab_id, 0.0);
        assert_eq!(rows[0].half_x, Some(1.2));
        assert_eq!(rows[0].height_m, Some(12.0));
        assert_eq!(rows[0].icon_key.as_deref(), Some("tree-conifer"));
        assert_eq!(rows[1].class, "unknown");
    }

    #[test]
    fn build_maps_codes_and_oversized() {
        let rows = vec![
            PrefabRow {
                prefab_id: 0.0,
                kind: "tree".into(),
                class: "conifer".into(),
                ..Default::default()
            },
            PrefabRow {
                prefab_id: 9.0,
                kind: "building".into(),
                class: "residential".into(),
                half_x: Some(70.0),
                half_y: Some(3.0),
                ..Default::default()
            }, // oversized
            PrefabRow {
                prefab_id: 3.0,
                kind: "misc".into(),
                class: "x".into(),
                ..Default::default()
            }, // NO_CLASS
        ];
        let (by_id, oversized) = build_prefab_maps(rows);
        assert!(oversized);
        assert_eq!(by_id.get(&0.0_f64.to_bits()).unwrap().code, 1); // tree
        assert_eq!(by_id.get(&9.0_f64.to_bits()).unwrap().code, 0); // building
        assert_eq!(by_id.get(&3.0_f64.to_bits()).unwrap().code, NO_CLASS);
    }

    #[test]
    fn oversized_only_when_classified() {
        // A 70 m half-extent on an UNclassified prefab must NOT set oversized (JS: `cls && …`).
        let rows = vec![PrefabRow {
            prefab_id: 1.0,
            kind: "misc".into(),
            class: "x".into(),
            half_x: Some(70.0),
            ..Default::default()
        }];
        let (_by_id, oversized) = build_prefab_maps(rows);
        assert!(!oversized);
    }

    /* ───────────────── T-935.11 — the rkyv prefab catalogue ───────────────── */

    use super::super::binary::to_bytes;
    use crate::world::bytes_to_json;
    use std::path::PathBuf;

    /// The committed everon catalogue: 1623 prefabs, 1,216,066 instances (`store.rs`'s census pin
    /// says the same). Re-pin deliberately if the export changes — a silently shrinking corpus is
    /// how a parity test stops proving anything.
    const EVERON_PREFABS: usize = 1623;
    const EVERON_INSTANCES: u64 = 1_216_066;

    fn everon() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/map-assets/everon")
    }

    fn everon_json_rows() -> Vec<PrefabRow> {
        let raw = std::fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs.json.gz");
        narrow_prefab_rows(&bytes_to_json(&raw).expect("prefabs decode"))
    }

    fn everon_inventory() -> TypeInventory {
        let raw = std::fs::read_to_string(everon().join("objects/type-inventory.json"))
            .expect("type-inventory.json");
        inventory_to_archive(&serde_json::from_str(&raw).expect("inventory parse"))
            .expect("inventory → archive")
    }

    /// The everon catalogue, as the emitter writes it.
    fn everon_archive() -> PrefabCatalogArchive {
        let rows = everon_json_rows();
        PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: rows
                .iter()
                .enumerate()
                .map(|(i, r)| row_to_archive(i, r).expect("row → archive"))
                .collect(),
            type_inventory: everon_inventory(),
        }
    }

    /// The archive's actual contract for a JSON number: it is the f32 twin of the parser struct,
    /// so the round trip is `f64::from(v as f32)`, not the identity. Written out longhand here
    /// rather than calling `num_to_wire`/`wire_to_num` — an oracle that reuses the production
    /// helpers cannot catch a bug in them.
    #[allow(clippy::cast_possible_truncation)]
    fn f32_narrow(v: Option<f64>) -> Option<f64> {
        v.map(|n| f64::from(n as f32))
    }

    /// THE SLICE'S PIN. Emit the everon catalogue from the committed JSON, read it back through
    /// the shipped public path (`catalog_from_bytes`, validation and terrain check and all), and
    /// prove the rows are the very rows `narrow_prefab_rows` yields — field by field, in order.
    ///
    /// Numbers compare by `to_bits()`, not `==`: `==` calls `-0.0` equal to `+0.0` and `NaN`
    /// unequal to itself, and the point of a binary twin is that the *values* agree exactly.
    #[test]
    fn everon_catalogue_archive_equals_the_json_rows() {
        let json = everon_json_rows();
        assert_eq!(
            json.len(),
            EVERON_PREFABS,
            "everon prefab corpus changed; re-pin EVERON_PREFABS deliberately"
        );

        let bytes = to_bytes(&everon_archive()).expect("serialise");
        assert!(
            bytes_to_json(&bytes).is_err(),
            "the archive must not be parseable as JSON, or this test has not proven the rkyv \
             route ran"
        );
        assert_ne!(&bytes[..2], &[0x1f, 0x8b], "must not carry gzip magic");

        let archive = access_checked::<PrefabCatalogArchive>(&bytes).expect("validated access");
        let back = rows_from_archive(archive).expect("rows");
        assert_eq!(back.len(), json.len(), "row count");

        // Coverage counters: without these the loop could compare 1623 rows that all happen to
        // take the same branch, and the absent-optional encoding would be untested.
        let (mut absent_icon, mut present_icon, mut coloured, mut importance) = (0, 0, 0, 0);
        for (i, (a, j)) in back.iter().zip(json.iter()).enumerate() {
            assert_eq!(a.prefab_id.to_bits(), j.prefab_id.to_bits(), "row {i}: id");
            assert_eq!(a.kind, j.kind, "row {i}: kind");
            assert_eq!(a.class, j.class, "row {i}: class");
            assert_eq!(a.label, j.label, "row {i}: label");
            assert_eq!(a.resource_name, j.resource_name, "row {i}: resourceName");
            assert_eq!(a.icon_key, j.icon_key, "row {i}: iconKey");
            assert_eq!(a.default_color, j.default_color, "row {i}: defaultColor");
            for (name, got, want) in [
                ("halfX", a.half_x, f32_narrow(j.half_x)),
                ("halfY", a.half_y, f32_narrow(j.half_y)),
                ("halfZ", a.half_z, f32_narrow(j.half_z)),
                ("heightM", a.height_m, f32_narrow(j.height_m)),
                ("baseSizePx", a.base_size_px, f32_narrow(j.base_size_px)),
                (
                    "importanceZoom",
                    a.importance_zoom,
                    f32_narrow(j.importance_zoom),
                ),
            ] {
                assert_eq!(
                    got.map(f64::to_bits),
                    want.map(f64::to_bits),
                    "row {i} ({}): {name}",
                    j.kind
                );
            }
            if a.icon_key.is_some() {
                present_icon += 1;
            } else {
                absent_icon += 1;
            }
            coloured += usize::from(a.default_color.is_some());
            importance += usize::from(a.importance_zoom.is_some());
        }
        assert!(
            absent_icon > 0 && present_icon > 0,
            "the corpus exercised only one side of the absent-iconKey encoding \
             ({absent_icon} absent / {present_icon} present)"
        );
        assert!(
            coloured > 0 && coloured < back.len(),
            "defaultColor: {coloured} of {} rows — one side of the encoding untested",
            back.len()
        );
        assert!(
            importance > 0 && importance < back.len(),
            "importanceZoom: {importance} of {} rows — one side of the encoding untested",
            back.len()
        );

        // …and the shipped entry point yields the same lookup the JSON path builds.
        let cat = catalog_from_bytes(&bytes, "everon").expect("catalogue");
        let (want_map, want_oversized) = build_prefab_maps(
            json.iter()
                .map(|r| PrefabRow {
                    half_x: f32_narrow(r.half_x),
                    half_y: f32_narrow(r.half_y),
                    half_z: f32_narrow(r.half_z),
                    height_m: f32_narrow(r.height_m),
                    base_size_px: f32_narrow(r.base_size_px),
                    importance_zoom: f32_narrow(r.importance_zoom),
                    ..r.clone()
                })
                .collect(),
        );
        assert_eq!(cat.by_id, want_map, "prefab lookup");
        assert_eq!(cat.has_oversized, want_oversized, "has_oversized");
        assert_eq!(cat.terrain_id, "everon");
        assert_eq!(cat.unique_prefabs as usize, EVERON_PREFABS);
        assert_eq!(cat.total_instances, EVERON_INSTANCES);
    }

    /// Rule 18 — the catalogue says which world it was built for, and a caller that disagrees is
    /// refused rather than served a table in which every id still resolves.
    #[test]
    fn a_catalogue_for_another_terrain_is_refused() {
        let bytes = to_bytes(&everon_archive()).expect("serialise");
        assert!(catalog_from_bytes(&bytes, "everon").is_ok(), "control");
        let err = catalog_from_bytes(&bytes, "arland").expect_err("must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("\"everon\"") && msg.contains("\"arland\""),
            "{msg}"
        );
    }

    /// The census has to describe the rows it ships with. `access_checked` cannot see this — both
    /// numbers are legal on their own.
    #[test]
    fn a_census_that_does_not_match_the_row_count_is_refused() {
        let mut archive = everon_archive();
        archive.type_inventory.unique_prefabs -= 1;
        let bytes = to_bytes(&archive).expect("serialise");
        assert!(
            access_checked::<PrefabCatalogArchive>(&bytes).is_ok(),
            "the bytes must still validate, or this proves nothing about the meaning check"
        );
        let msg = catalog_from_bytes(&bytes, "everon")
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains("stale or half-written"), "{msg}");
    }

    /// Rule 16 — a well-formed archive written by a different schema is refused. `access_checked`
    /// cannot catch this: the layout is legal, the *meaning* is what moved.
    #[test]
    fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
        let mut archive = everon_archive();
        archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
        let bytes = to_bytes(&archive).expect("serialise");
        assert!(access_checked::<PrefabCatalogArchive>(&bytes).is_ok());
        assert!(matches!(
            catalog_from_bytes(&bytes, "everon"),
            Err(BinaryError::UnsupportedVersion { .. })
        ));
    }

    /// A class byte written against a different `RENDER_CLASS_CODES` is refused. Without this the
    /// file would draw every instance of that prefab as the wrong class, permanently and with no
    /// error anywhere.
    #[test]
    fn a_drifted_class_code_is_refused() {
        let mut archive = everon_archive();
        let row = archive
            .prefabs
            .iter_mut()
            .find(|p| p.kind == "tree")
            .expect("a tree prefab");
        row.class_code = row.class_code.wrapping_add(1);
        let bytes = to_bytes(&archive).expect("serialise");
        assert!(access_checked::<PrefabCatalogArchive>(&bytes).is_ok());
        let msg = catalog_from_bytes(&bytes, "everon")
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains("RENDER_CLASS_CODES"), "{msg}");
    }

    /// Absent and present survive the round trip as themselves, on a row built to take every
    /// branch — the everon pin covers the corpus, this covers the encoding.
    #[test]
    fn absent_and_present_optionals_round_trip() {
        let rows = vec![
            PrefabRow {
                prefab_id: 7.0,
                kind: "building".into(),
                class: "civic".into(),
                label: Some("Hut".into()),
                resource_name: Some("{ABC}Hut.et".into()),
                half_x: Some(1.5),
                half_y: Some(2.25),
                half_z: Some(0.0), // a REAL zero half-extent: must not decode as absent
                height_m: Some(4.0),
                icon_key: Some("hut".into()),
                base_size_px: Some(18.0),
                default_color: Some("#0a1bff".into()),
                importance_zoom: Some(-4.0),
            },
            PrefabRow {
                prefab_id: 8.0,
                kind: "rock".into(),
                class: "boulder".into(),
                ..Default::default()
            },
        ];
        let archive = PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: rows
                .iter()
                .enumerate()
                .map(|(i, r)| row_to_archive(i, r).expect("encode"))
                .collect(),
            type_inventory: TypeInventory {
                terrain_id: "probe".into(),
                census_status: "partial".into(),
                unique_prefabs: 2,
                total_instances: 0,
                by_kind: Vec::new(),
            },
        };
        let bytes = to_bytes(&archive).expect("serialise");
        let back =
            rows_from_archive(access_checked::<PrefabCatalogArchive>(&bytes).expect("access"))
                .expect("rows");
        assert_eq!(back, rows, "every optional must survive as itself");
        assert_eq!(back[0].half_z, Some(0.0), "a real 0.0 is not \"absent\"");
        assert!(back[1].label.is_none() && back[1].height_m.is_none());
    }

    /// The three encodings a writer could silently corrupt are hard errors instead.
    #[test]
    fn rows_that_cannot_be_encoded_without_lying_are_refused() {
        let base = PrefabRow {
            prefab_id: 1.0,
            kind: "prop".into(),
            class: "fence".into(),
            ..Default::default()
        };
        let cases: [(&str, PrefabRow); 5] = [
            (
                "not a u32-representable whole number",
                PrefabRow {
                    prefab_id: 1.5,
                    ..base.clone()
                },
            ),
            (
                "not a u32-representable whole number",
                PrefabRow {
                    prefab_id: -1.0,
                    ..base.clone()
                },
            ),
            (
                "empty label",
                PrefabRow {
                    label: Some(String::new()),
                    ..base.clone()
                },
            ),
            (
                "not a lowercase #rrggbb",
                PrefabRow {
                    default_color: Some("#2D5A27".into()),
                    ..base.clone()
                },
            ),
            (
                "non-finite heightM",
                PrefabRow {
                    height_m: Some(f64::NAN),
                    ..base.clone()
                },
            ),
        ];
        for (needle, row) in cases {
            let msg = row_to_archive(0, &row)
                .expect_err("must refuse")
                .to_string();
            assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
        }
        row_to_archive(0, &base).expect("the control row must encode");
    }

    /// Nothing malformed reaches the row loop: an empty buffer, a truncated one, a gzip payload
    /// and a corrupt tail are all errors, never a partial catalogue.
    #[test]
    fn malformed_buffers_error_rather_than_yielding_rows() {
        let bytes = to_bytes(&everon_archive()).expect("serialise");
        for (what, buf) in [
            ("empty", &[][..]),
            ("truncated tail", &bytes[..bytes.len() - 1]),
            ("shifted head", &bytes[1..]),
            ("json", br#"{"prefabs":[]}"#),
        ] {
            assert!(
                catalog_from_bytes(buf, "everon").is_err(),
                "{what} was accepted"
            );
        }
    }

    /// The standalone census file decodes to the very inventory the catalogue carries — the
    /// emitter writes one value into two files and this is what says so.
    #[test]
    fn the_standalone_census_equals_the_one_inside_the_catalogue() {
        let inventory = everon_inventory();
        let bytes = to_bytes(&inventory).expect("serialise");
        assert_eq!(inventory_from_bytes(&bytes).expect("read back"), inventory);
        assert_eq!(inventory.terrain_id, "everon");
        assert_eq!(inventory.unique_prefabs as usize, EVERON_PREFABS);
        assert_eq!(inventory.total_instances, EVERON_INSTANCES);
        // The nine census kinds, `road` last (`INSTANCE_KINDS` order, preserved by serde_json).
        assert_eq!(inventory.by_kind.len(), 9, "{:?}", inventory.by_kind);
        assert_eq!(inventory.by_kind[0].kind, "building");
        assert_eq!(inventory.by_kind[8].kind, "road");
        assert_eq!(
            inventory.by_kind.iter().map(|k| k.instances).sum::<u64>(),
            EVERON_INSTANCES,
            "the per-kind census must add up to the declared total"
        );
        assert!(inventory_from_bytes(&[]).is_err(), "empty buffer");
    }

    /// A census missing a required field is a hard error, not a zero. (The "reports success over
    /// an input it never examined" shape: a defaulted `totalInstances` of 0 reads as a real
    /// answer.)
    #[test]
    fn an_incomplete_census_is_refused() {
        for doc in [
            json!({ "censusStatus": "partial", "levels": { "uniquePrefabs": 1, "totalInstances": 2 }, "byKind": {} }),
            json!({ "terrainId": "everon", "censusStatus": "partial", "byKind": {} }),
            json!({ "terrainId": "everon", "censusStatus": "partial", "levels": { "uniquePrefabs": 1 }, "byKind": {} }),
            json!({ "terrainId": "everon", "censusStatus": "partial", "levels": { "uniquePrefabs": 1, "totalInstances": 2 } }),
        ] {
            assert!(inventory_to_archive(&doc).is_err(), "accepted: {doc}");
        }
        assert!(
            inventory_to_archive(&json!({
                "terrainId": "everon", "censusStatus": "partial",
                "levels": { "uniquePrefabs": 1, "totalInstances": 2 },
                "byKind": { "tree": { "prefabTypes": 1, "instances": 2 } }
            }))
            .is_ok(),
            "the control document must be accepted"
        );
    }
}
