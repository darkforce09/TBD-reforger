//! The prefab rows of a terrain's catalogue and their archive form.
//!
//! **Role:** narrows the catalogue JSON into [`PrefabRow`]s, keys them into the prefab map the
//! chunk decoders join against ([`build_prefab_maps`]), and converts rows and the type inventory to
//! and from the rkyv archive (`objects/prefabs.rkyv`, `objects/type-inventory.rkyv`), reading a
//! whole archive into a [`PrefabCatalog`] ([`catalog_from_bytes`]).
//! **Position:** under `prefab_catalog`, over `world_file_formats`' prefab archive; read by
//! [`crate::prefab_tables`], `world_chunks`, the map engine's world store and scheduler, and the
//! developer tools' catalogue and binary export.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** a row's prefab id is a [`PrefabId`] (u32): the JSON narrowing refuses a numeric
//! `prefabId` that is not a whole number in `0..=u32::MAX` with [`InvalidPrefabId`], never a cast;
//! the prefab map is keyed by [`prefab_map_key`] (the id's `f64` bit pattern, the key the chunk
//! decoders derive from a chunk's numeric `pid`), so the JSON lane and the archive lane resolve
//! the same integer to the same entry; an archive is validated (alignment, schema version,
//! terrain, census) before any row is read.

use std::collections::HashMap;

use rkyv::Archived;
use serde_json::Value;

use crate::error::InvalidPrefabId;
use crate::render_classes::NO_CLASS;
use crate::render_classes::OVERSIZED_HALF_EXTENT_M;
use crate::render_classes::class_code;
use crate::render_classes::render_class_for_prefab;
use world_file_formats::archives::codec::BinaryError;
use world_file_formats::archives::codec::access_checked;
use world_file_formats::archives::prefabs::KindCensus;
use world_file_formats::archives::prefabs::PrefabCatalogArchive;
use world_file_formats::archives::prefabs::PrefabEntry as PrefabEntryArchive;
use world_file_formats::archives::prefabs::TypeInventory;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;
use world_file_formats::ids::{PrefabId, TerrainId};

/// Clone-safe prefab row subset (mirror of `WorldPrefabRow`). `prefab_id` is the integral catalogue id the chunk instances join on. Spatial half-extents / render glyph fields are carried for building rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabRow {
    /// The prefab's id in the terrain catalogue (JSON `prefabId`), the join key of every chunk
    /// instance's `pid`.
    pub prefab_id: PrefabId,

    /// Object kind (JSON `kind`, required): `building`, `water`, `tree`, `rock`, `prop` and so on.
    pub kind: String,

    /// Sub-class within the kind (JSON `class`, e.g. `pier`, `fence`); `unknown` when absent.
    pub class: String,

    /// Human-readable prefab name (JSON `label`); `None` when absent.
    pub label: Option<String>,

    /// Enfusion prefab resource path, GUID included (JSON `resourceName`); `None` when absent.
    pub resource_name: Option<String>,

    /// Footprint half-extent along world X (east), metres (JSON `spatial.halfExtentsM.x`).
    pub half_x: Option<f64>,

    /// Footprint half-extent along world Z (north), metres (JSON `spatial.halfExtentsM.y`).
    pub half_y: Option<f64>,

    /// Vertical half-extent, metres (JSON `spatial.halfExtentsM.z`).
    pub half_z: Option<f64>,

    /// Object height, metres (JSON `spatial.heightM`); scales tree and prop glyphs.
    pub height_m: Option<f64>,

    /// Glyph atlas key the object draws with (JSON `render.iconKey`); `None` draws no glyph.
    pub icon_key: Option<String>,

    /// Glyph base size, screen pixels (JSON `render.baseSizePx`); `None` takes the map default.
    pub base_size_px: Option<f64>,

    /// Glyph tint as lowercase `#rrggbb` (JSON `render.defaultColor`).
    pub default_color: Option<String>,

    /// Map zoom from which a building draws its landmark glyph early (`render.importanceZoom`).
    pub importance_zoom: Option<f64>,
}

impl Default for PrefabRow {
    /// Prefab id 0, empty `kind` and `class`, every optional field absent.
    fn default() -> Self {
        Self {
            prefab_id: PrefabId::new(0),
            kind: String::new(),
            class: String::new(),
            label: None,
            resource_name: None,
            half_x: None,
            half_y: None,
            half_z: None,
            height_m: None,
            icon_key: None,
            base_size_px: None,
            default_color: None,
            importance_zoom: None,
        }
    }
}

/// `buildPrefabMaps` byId value: prefabId → render-class code + the narrowed row.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabEntry {
    /// Render-class wire code: the index in `RENDER_CLASS_CODES`, or [`NO_CLASS`] (never drawn).
    pub code: u8,

    /// The narrowed catalogue row the code was derived from.
    pub row: PrefabRow,
}

/// The JSON `prefabId` of row `row` → its [`PrefabId`], or [`InvalidPrefabId`] when the number is
/// fractional, negative, non-finite or above `u32::MAX`.
fn prefab_id_from_json(row: usize, value: f64) -> Result<PrefabId, InvalidPrefabId> {
    if value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(PrefabId::new(value as u32))
    } else {
        Err(InvalidPrefabId { row, value })
    }
}

/// The [`build_prefab_maps`] key of a prefab: the `f64` bit pattern of its id, the key the chunk
/// decoders derive from an instance's numeric `pid`.
#[must_use]
pub fn prefab_map_key(prefab_id: PrefabId) -> u64 {
    f64::from(prefab_id.get()).to_bits()
}

#[must_use]
fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

#[must_use]
fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

/// `narrowPrefabRows(raw)` (`:291`) — keep rows with a numeric `prefabId` and string `kind`; `class` falls back to `"unknown"`. Spatial + render blocks narrowed field-by-field. A numeric `prefabId` that is not a whole number in `0..=u32::MAX` refuses the whole catalogue with [`InvalidPrefabId`].
pub fn narrow_prefab_rows(raw: &Value) -> Result<Vec<PrefabRow>, InvalidPrefabId> {
    let Some(rows) = raw.get("prefabs").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let Some(prefab_id) = r.get("prefabId").and_then(Value::as_f64) else {
            continue;
        };
        let prefab_id = prefab_id_from_json(i, prefab_id)?;
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
    Ok(out)
}

/// `buildPrefabMaps(prefabRows)` (`:381`) → (prefabId→{code,row}, has_oversized). The map is keyed by [`prefab_map_key`] so the chunk lookup matches JS `Map<number,…>` bit-for-bit (both sides turn the same integer into the same f64). `has_oversized` mirrors `cls && Math.max(hx, hy) >= 64` with `hx/hy` defaulting to 0.
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
        by_id.insert(prefab_map_key(row.prefab_id), PrefabEntry { code, row });
    }
    (by_id, has_oversized)
}

/// Canonical prefab catalog align value.
pub const PREFAB_CATALOG_ALIGN: usize = 16;

const _: () = assert!(
    PREFAB_CATALOG_ALIGN.is_multiple_of(align_of::<Archived<PrefabCatalogArchive>>()),
    "PREFAB_CATALOG_ALIGN must be a multiple of the archived type's own alignment"
);

#[must_use]
fn num_to_wire(v: Option<f64>) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    v.map_or(f32::NAN, |n| n as f32)
}

#[must_use]
fn wire_to_num(v: f32) -> Option<f64> {
    (!v.is_nan()).then(|| f64::from(v))
}

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

#[must_use]
fn wire_to_color(c: [u8; 4]) -> Option<String> {
    (c[3] != 0).then(|| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
}

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
pub fn row_to_archive(i: usize, row: &PrefabRow) -> Result<PrefabEntryArchive, BinaryError> {
    let bad = |cause: String| BinaryError::Archive {
        what: "PrefabCatalogArchive",
        cause,
    };
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
    Ok(PrefabEntryArchive {
        prefab_id: row.prefab_id,
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

/// `Archived<PrefabCatalogArchive>` → the very rows [`narrow_prefab_rows`] yields from `prefabs.json.gz`, in file order.
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
                    p.prefab_id.get(),
                    p.class_code
                ),
            });
        }
        let text = |s: &str| (!s.is_empty()).then(|| s.to_string());
        out.push(PrefabRow {
            prefab_id: p.prefab_id.to_native(),
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

/// What one `objects/prefabs.rkyv` decodes to: the same pair [`build_prefab_maps`] returns, plus the census the catalogue carries about itself.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabCatalog {
    /// [`prefab_map_key`] → render-class code + narrowed row.
    pub by_id: HashMap<u64, PrefabEntry>,

    /// Whether a drawn prefab's larger footprint half-extent reaches [`OVERSIZED_HALF_EXTENT_M`].
    pub has_oversized: bool,

    /// The terrain this catalogue was built for — checked against the caller's, see [`catalog_from_bytes`].
    pub terrain_id: TerrainId,

    /// Distinct prefabs the census declares (`levels.uniquePrefabs`); equals the row count.
    pub unique_prefabs: u32,

    /// Placed instances of every prefab on the terrain, from the census (`levels.totalInstances`).
    pub total_instances: u64,
}

/// `Archived<PrefabCatalogArchive>` → `(map, has_oversized)`, exactly as `build_prefab_maps(narrow_prefab_rows(json))` would.
pub fn from_archive(
    archive: &Archived<PrefabCatalogArchive>,
) -> Result<(HashMap<u64, PrefabEntry>, bool), BinaryError> {
    Ok(build_prefab_maps(rows_from_archive(archive)?))
}

/// One `objects/prefabs.rkyv` file → the prefab lookup, JSON-free.
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
        terrain_id: TerrainId::new(terrain_id),
        unique_prefabs,
        total_instances: archive.type_inventory.total_instances.to_native(),
    })
}

/// `type-inventory.json` → the wire census that rides inside [`PrefabCatalogArchive`] and also ships standalone as `objects/type-inventory.rkyv`.
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
        terrain_id: TerrainId::new(text("terrainId")?),
        census_status: text("censusStatus")?,
        unique_prefabs,
        total_instances: count(levels, "totalInstances")?,
        by_kind,
    })
}

/// One `objects/type-inventory.rkyv` file → the owned census.
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
#[path = "tests/prefab_rows_tests.rs"]
mod tests;
