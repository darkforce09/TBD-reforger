//! Blueprint assembly: normalized interpretation products → local-frame schema-1.0.0
//! `BuildingBlueprint`, jsonschema-validated against the tbd-schema contract before writing.
//! The live extractor wrote `category: "scanned"` and furniture `category: "scan_mass"`, both
//! outside the schema enums — they only survived because ingest validates via serde (plain
//! `String`). This emitter validates for real, so it emits the legal `"generic"` / `"prop"`.

use std::path::Path;

use anyhow::{Context, Result, bail};
use map_engine_core::building_blueprint::{
    BBox2D, BuildingBlueprint, BuildingFurniture, BuildingLevel, BuildingWall, FloorPolygon,
    OverallFootprint, PlateGrid, VerticalProfile,
};

use super::march::r2;
use super::params::Params;
use super::types::{VerticalScan, VoxelDump};
use super::walls::BandWalls;

pub struct BandProducts {
    pub band_lo: f64,
    pub band_hi: f64,
    pub walls: BandWalls,
    /// Largest traced outer ring, normalized frame (empty for slab-less bands).
    pub footprint: Vec<[f64; 2]>,
    /// All traced plate pieces (outer + holes), normalized frame.
    pub floor_polygons: Vec<FloorPolygon>,
    /// Per-cell topmost in-window y_down entry, normalized frame; empty = no plate scanned.
    pub plate_heights: Vec<Option<f64>>,
    pub plate_cells: usize,
    /// Synthesized eave→ridge band: named "Attic", carries no plate by design.
    pub is_attic: bool,
}

pub fn assemble(
    dump: &VoxelDump,
    vert: &VerticalScan,
    bands: Vec<BandProducts>,
    p: &Params,
) -> BuildingBlueprint {
    let m = dump.meta();
    let (ox, oy, oz) = (m.origin[0], m.origin[1], m.origin[2]);
    let local_pt = |pt: [f64; 2]| [pt[0] + ox, pt[1] + oz];
    let (dims_nx, dims_nz) = (vert.nx, vert.nz);

    let mut levels = Vec::new();
    let mut wall_seq = 0usize;
    for (li, band) in bands.iter().enumerate() {
        let mut walls = Vec::new();
        for (w, is_ext) in band.walls.walls.iter().zip(&band.walls.exterior) {
            walls.push(BuildingWall {
                id: format!("w_scan_{li}_{wall_seq}"),
                start: local_pt(w.start),
                end: local_pt(w.end),
                thickness: w.thickness,
                is_exterior: *is_ext,
                material: "scanned".to_string(),
            });
            wall_seq += 1;
        }
        let mut furniture = Vec::new();
        for mass in &band.walls.masses {
            let r = mass.rect;
            let (cx, cz) = ((r[0] + r[2]) * 0.5 + ox, (r[1] + r[3]) * 0.5 + oz);
            furniture.push(BuildingFurniture {
                id: format!("mass_{li}_{wall_seq}"),
                name: "scanned mass".to_string(),
                category: "prop".to_string(),
                prefab_resource: String::new(),
                pos2_d: [cx, cz],
                rotation_deg: 0.0,
                size2_d: [r[2] - r[0], r[3] - r[1]],
                height_m: band.band_hi - band.band_lo,
                blocks_movement: true,
                los_cover: "full_cover".to_string(),
            });
            wall_seq += 1;
        }
        let plate = (!band.plate_heights.is_empty()).then(|| PlateGrid {
            origin: [ox, oz],
            cell_size_m: m.cell,
            nx: dims_nx,
            nz: dims_nz,
            heights_m: band
                .plate_heights
                .iter()
                .map(|h| h.map(|y| r2(y + oy)))
                .collect(),
        });
        levels.push(BuildingLevel {
            level_index: li,
            name: if band.is_attic {
                "Attic".to_string()
            } else {
                ordinal_floor_name(li + 1)
            },
            elevation_range: [band.band_lo + oy, band.band_hi + oy],
            slice_height_m: band.band_lo + oy + p.slice_height_above_floor_m,
            footprint_polygon: band.footprint.iter().map(|pt| local_pt(*pt)).collect(),
            plate,
            floor_polygons: band
                .floor_polygons
                .iter()
                .map(|fp| FloorPolygon {
                    outer: fp.outer.iter().map(|pt| local_pt(*pt)).collect(),
                    holes: fp
                        .holes
                        .iter()
                        .map(|h| h.iter().map(|pt| local_pt(*pt)).collect())
                        .collect(),
                })
                .collect(),
            walls,
            doors: Vec::new(),   // apertures muted: walls-first iteration
            windows: Vec::new(), // (dump carries the data; no re-dump needed to enable)
            stairs: Vec::new(),
            furniture,
        });
    }

    place_furniture(dump, &mut levels, p);

    let ground = levels.first();
    let polygon: Vec<[f64; 2]> = ground
        .map(|l| l.footprint_polygon.clone())
        .unwrap_or_default();
    let (mut mn, mut mx) = ([f64::MAX; 2], [f64::MIN; 2]);
    for pt in &polygon {
        for a in 0..2 {
            mn[a] = mn[a].min(pt[a]);
            mx[a] = mx[a].max(pt[a]);
        }
    }
    if polygon.is_empty() {
        mn = [m.bbox_min[0], m.bbox_min[2]];
        mx = [m.bbox_max[0], m.bbox_max[2]];
    }
    let plate_cells = bands.first().map(|b| b.plate_cells).unwrap_or(0);

    BuildingBlueprint {
        schema_version: "1.0.0".to_string(),
        prefab_id: m.slug.clone(),
        resource_name: m.resource.clone(),
        model_mesh: None,
        label: Some(m.slug.clone()),
        kind: "building".to_string(),
        category: "generic".to_string(),
        destructible: true,
        vertical_profile: VerticalProfile {
            pivot_elevation_offset_m: 0.0,
            foundation_skirt_depth_m: (-m.bbox_min[1]).max(0.0),
            total_height_m: m.bbox_max[1],
            eave_height_m: vert.eave + oy,
            ridge_height_m: vert.ridge + oy,
            chimney_height_m: vert.chimney.map(|c| c + oy),
            roof_type: if vert.chimney.is_some() {
                "with_chimney"
            } else {
                "scanned"
            }
            .to_string(),
        },
        overall_footprint: OverallFootprint {
            polygon2_d: polygon,
            bounding_box2_d: BBox2D {
                min: mn,
                max: mx,
                width_m: mx[0] - mn[0],
                depth_m: mx[1] - mn[1],
            },
            footprint_sq_m: (plate_cells as f64 * m.cell * m.cell * 100.0).round() / 100.0,
        },
        roof: super::roof::build(vert, m, p),
        levels,
    }
}

/// Furniture records from the dump's excluded-prop lines: local-frame positions pass through;
/// yaw becomes ROOT-relative (the dumper records world yaw; the live extractor wrote world yaw
/// into the local-frame record — wrong for rotated instances).
fn place_furniture(dump: &VoxelDump, levels: &mut [BuildingLevel], p: &Params) {
    let root_yaw = dump.meta().root_yaw_deg;
    for (seq, f) in dump.furniture.iter().enumerate() {
        let (fw, fh, fd) = (f.size[0], f.size[1], f.size[2]);
        if fw.max(fd) > p.furn_max_plan_m || fh > p.furn_max_height_m {
            continue;
        }
        let cover = if fh >= p.furn_full_cover_m {
            "full_cover"
        } else if fh < p.furn_none_below_m {
            "none"
        } else {
            "low_cover"
        };
        let name = if f.name.is_empty() {
            "prop".to_string()
        } else {
            f.name.clone()
        };
        let rec = BuildingFurniture {
            id: format!("furn_scan_{seq}"),
            name,
            category: "prop".to_string(),
            prefab_resource: f.res.clone(),
            pos2_d: [f.pos[0], f.pos[2]],
            rotation_deg: f.world_yaw_deg - root_yaw,
            size2_d: [fw.max(0.2), fd.max(0.2)],
            height_m: fh.max(0.2),
            blocks_movement: true,
            los_cover: cover.to_string(),
        };
        for lvl in levels.iter_mut() {
            if f.pos[1] >= lvl.elevation_range[0] - p.furn_level_slack_m
                && f.pos[1] < lvl.elevation_range[1]
            {
                lvl.furniture.push(rec);
                break;
            }
        }
    }
}

fn ordinal_floor_name(n: usize) -> String {
    match n {
        1 => "1st Floor".to_string(),
        2 => "2nd Floor".to_string(),
        3 => "3rd Floor".to_string(),
        _ => format!("{n}th Floor"),
    }
}

/// Validate against the tbd-schema contract, then write pretty JSON.
pub fn validate_and_write(
    bp: &BuildingBlueprint,
    schema_path: &Path,
    out_path: &Path,
) -> Result<()> {
    let mut value = serde_json::to_value(bp)?;
    // The contract types `modelMesh` / `chimneyHeightM` as non-null and optional BY OMISSION;
    // the Rust Options serialize to null, so absent means the key must go, not carry null.
    if value
        .get("modelMesh")
        .is_some_and(serde_json::Value::is_null)
    {
        value
            .as_object_mut()
            .expect("blueprint object")
            .remove("modelMesh");
    }
    if let Some(vp) = value
        .get_mut("verticalProfile")
        .and_then(|v| v.as_object_mut())
        && vp
            .get("chimneyHeightM")
            .is_some_and(serde_json::Value::is_null)
    {
        vp.remove("chimneyHeightM");
    }
    let schema_text = std::fs::read_to_string(schema_path)
        .with_context(|| format!("read schema {}", schema_path.display()))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let validator = jsonschema::validator_for(&schema)
        .with_context(|| format!("compile schema {}", schema_path.display()))?;
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| format!("  {} @ {}", e, e.instance_path()))
        .collect();
    if !errors.is_empty() {
        bail!(
            "blueprint fails the schema contract:\n{}",
            errors.join("\n")
        );
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_blueprint::{plate, slabs, synth, walls};
    use crate::root::find_repo_root;

    #[test]
    fn box_room_blueprint_passes_the_schema_contract() {
        let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
        let m = d.meta().clone();
        let p = Params {
            min_floor_y: -0.5 - m.origin[1],
            ..Default::default()
        };
        let v = slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
        let lo = v.floors[0];
        let hi = v.eave.max(lo + p.top_band_min_m);
        let bw = walls::extract_band(&d, &v, lo, hi, walls::Algo::Segments, &p, None);
        let (pg, plate_heights) = plate::floor_plate(&d.y_down, v.nx, v.nz, lo, &p);
        let plate_cells = pg.count();
        let traced = crate::map_blueprint::rings::trace(&pg, m.cell, p.plate_min_ring_area_m2);
        let (footprint, floor_polygons) = traced.contract();
        let bp = assemble(
            &d,
            &v,
            vec![BandProducts {
                band_lo: lo,
                band_hi: hi,
                walls: bw,
                footprint,
                floor_polygons,
                plate_heights,
                plate_cells,
                is_attic: false,
            }],
            &p,
        );

        assert_eq!(bp.levels.len(), 1);
        assert_eq!(bp.levels[0].walls.len(), 4);
        // Local frame: west wall (z-running, constant x) centerline near x = 0.075.
        let west_x = bp.levels[0]
            .walls
            .iter()
            .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
            .map(|w| w.start[0])
            .fold(f64::MAX, f64::min);
        assert!(
            (west_x - 0.075).abs() < 0.06,
            "local-frame west centerline, got {west_x}"
        );

        let root = find_repo_root().expect("repo root");
        let schema = root.join("packages/tbd-schema/schema/building-blueprint.schema.json");
        let tmp = std::env::temp_dir().join("tbd_bp_synth_schema_test.json");
        validate_and_write(&bp, &schema, &tmp).expect("schema-valid blueprint");
        let _ = std::fs::remove_file(tmp);
    }
}
