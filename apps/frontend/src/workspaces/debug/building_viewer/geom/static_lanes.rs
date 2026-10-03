//! Static building geometry mapped into render lane payloads.

use super::*;

/// A mesh heightfield onto the floor lane: one quad per cell with a surface, tinted through
/// [`ramp`]; cells above `accent_above` take the chimney accent. Relief reads at a glance —
/// the same stepped gradient the RoofGrid cells had, now from the mesh on every view.
fn paint_heightfield(
    out: &mut StaticLanes,
    hf: &HeightField,
    stops: &[(f64, [f32; 4])],
    accent_above: Option<f64>,
) {
    for row in 0..hf.rows {
        for col in 0..hf.cols {
            let Some(y) = hf.at(col, row) else {
                continue;
            };
            let color = if accent_above.is_some_and(|a| y > a) {
                COL_ROOF_CHIMNEY
            } else {
                ramp(stops, y)
            };
            append_polygon(
                &mut out.floor_pos,
                &mut out.floor_col,
                &mut out.floor_idx,
                &rect_corners(hf.cell_center(col, row), [hf.cell_m, hf.cell_m], 0.0),
                color,
            );
            out.mesh_cell_count += 1;
        }
    }
}

/// Door swing arc as a hairline polyline: quarter circle of radius `width` around the hinge,
/// starting on the wall line and sweeping toward the side whose arc midpoint lies inside the
/// floor footprint for `inward` doors (outside for `outward`).
#[allow(clippy::too_many_arguments)] // one call site; a params struct would be pure ceremony
fn swing_arc(
    out: &mut Vec<f32>,
    count: &mut u32,
    lvl: &BuildingLevel,
    pos: [f64; 2],
    width: f64,
    hinge_side: &str,
    swing: &str,
    wall_dir: [f64; 2],
) {
    let u = wall_dir;
    let hinge = if hinge_side == "right" {
        [pos[0] + u[0] * width * 0.5, pos[1] + u[1] * width * 0.5]
    } else {
        [pos[0] - u[0] * width * 0.5, pos[1] - u[1] * width * 0.5]
    };
    // Leaf direction when closed = along the wall toward the door center.
    let leaf0 = [pos[0] - hinge[0], pos[1] - hinge[1]];
    let leaf_len = (leaf0[0] * leaf0[0] + leaf0[1] * leaf0[1]).sqrt().max(1e-9);
    let l0 = [leaf0[0] / leaf_len, leaf0[1] / leaf_len];
    let a0 = l0[1].atan2(l0[0]);
    for sign in [1.0f64, -1.0] {
        // Candidate quarter sweep; keep the one matching the swing side.
        let mid = a0 + sign * std::f64::consts::FRAC_PI_4;
        let mid_pt = [hinge[0] + width * mid.cos(), hinge[1] + width * mid.sin()];
        let inside = point_in_polygon(mid_pt, &lvl.footprint_polygon);
        let want_inside = swing != "outward";
        if inside != want_inside {
            continue;
        }
        let mut prev = [hinge[0] + width * a0.cos(), hinge[1] + width * a0.sin()];
        for i in 1..=16 {
            let a = a0 + sign * std::f64::consts::FRAC_PI_2 * (f64::from(i) / 16.0);
            let p = [hinge[0] + width * a.cos(), hinge[1] + width * a.sin()];
            seg(out, to_world(prev), to_world(p), COL_ARC);
            *count += 1;
            prev = p;
        }
        // Leaf line at full-open.
        seg(out, to_world(hinge), to_world(prev), COL_ARC);
        *count += 1;
        return;
    }
}

/// Tessellate one (blueprint, drawing, view-floor) state into lane payloads. With a mesh
/// `drawing` the structure is the mesh's: eye-height section cuts are the walls, slab faces
/// the floor, roof faces the roof, and lower floors show through this floor's voids; the
/// blueprint contributes plates, apertures, furniture, stairs and rings as annotations.
/// Without one (no sidecar) the blueprint supplies the structure. The Roof view draws
/// the overall footprint plate plus every floor's section (or centerlines) as ghosts — the
/// plan of what you stand ON, not a floor with its own walls.
#[must_use]
pub fn build_static_lanes(
    bp: &BuildingBlueprint,
    drawing: Option<&BuildingDrawing>,
    view: ViewFloor,
) -> StaticLanes {
    let mut out = StaticLanes::default();
    let ViewFloor::Level(active) = view else {
        // Roof.
        append_polygon(
            &mut out.floor_pos,
            &mut out.floor_col,
            &mut out.floor_idx,
            &bp.overall_footprint.polygon2_d,
            COL_FLOOR,
        );
        // Heightfield cells over the plate: dark at the eave, light at the ridge, chimney
        // spikes (above ridge + 0.3) in the accent color. This is the emitted RoofGrid drawn
        // verbatim — ridge lines, hips, valleys and dormer pits are eyeballable directly.
        if let Some(roof) = bp
            .roof
            .as_ref()
            .filter(|r| r.is_valid() && drawing.is_none())
        {
            let vp = &bp.vertical_profile;
            let span = (vp.ridge_height_m - vp.eave_height_m).max(0.1);
            let cell = roof.cell_size_m;
            for cx in 0..roof.nx {
                for cz in 0..roof.nz {
                    let Some(h) = roof.heights_m[cx * roof.nz + cz] else {
                        continue;
                    };
                    let col = if h > vp.ridge_height_m + 0.3 {
                        COL_ROOF_CHIMNEY
                    } else {
                        let t = (((h - vp.eave_height_m) / span).clamp(0.0, 1.0)) as f32;
                        [
                            COL_ROOF_LO[0] + (COL_ROOF_HI[0] - COL_ROOF_LO[0]) * t,
                            COL_ROOF_LO[1] + (COL_ROOF_HI[1] - COL_ROOF_LO[1]) * t,
                            COL_ROOF_LO[2] + (COL_ROOF_HI[2] - COL_ROOF_LO[2]) * t,
                            COL_ROOF_LO[3] + (COL_ROOF_HI[3] - COL_ROOF_LO[3]) * t,
                        ]
                    };
                    let center = [
                        roof.origin[0] + (cx as f64 + 0.5) * cell,
                        roof.origin[1] + (cz as f64 + 0.5) * cell,
                    ];
                    append_polygon(
                        &mut out.floor_pos,
                        &mut out.floor_col,
                        &mut out.floor_idx,
                        &rect_corners(center, [cell, cell], 0.0),
                        col,
                    );
                    out.roof_cell_count += 1;
                }
            }
        }
        // The mesh's full top surface as a stepped heightfield — the roof plan. Eave→ridge
        // ramp from the vertical profile (the field's own range when the profile is flat),
        // chimney accent above the ridge; replaces the RoofGrid cells when present.
        if let Some(d) = drawing.filter(|d| d.roof.range().is_some()) {
            let vp = &bp.vertical_profile;
            let profiled = vp.eave_height_m < vp.ridge_height_m;
            let (eave, ridge) = if profiled {
                (vp.eave_height_m, vp.ridge_height_m)
            } else {
                (d.roof_y[0], d.roof_y[1])
            };
            paint_heightfield(
                &mut out,
                &d.roof,
                &[(eave, COL_ROOF_LO), (ridge, COL_ROOF_HI)],
                profiled.then_some(vp.ridge_height_m + 0.3),
            );
        }
        // Every floor's wall centerlines as ghosts — few and clean, the plan through the
        // roof (the mesh cuts would be hundreds of segments of noise here).
        for ghost in &bp.levels {
            for wall in &ghost.walls {
                seg(
                    &mut out.hairlines,
                    to_world(wall.start),
                    to_world(wall.end),
                    COL_GHOST,
                );
                out.hairline_count += 1;
            }
        }
        return out;
    };
    let Some(lvl) = bp.levels.get(active) else {
        return out;
    };
    let mesh_level = drawing.and_then(|d| d.levels.get(active));

    if let Some(l) = mesh_level {
        // The mesh's top surface below this floor's cut plane as a stepped heightfield:
        // floor mid-tone, treads / sills / lower roofs brighter with height, stairwell
        // pits and the floors below darker with depth. Replaces the blueprint plate, its
        // traced rings and its stairs plate — the mesh now shows all of that for real.
        paint_heightfield(
            &mut out,
            &l.surface,
            &[
                (l.lo - PIT_DEPTH_M, COL_PIT),
                (l.floor_min_y(), COL_PLATE_LO),
                (l.lo + FLOOR_WINDOW_M[1], COL_PLATE_HI),
                (l.cut_main_y, COL_RAISED),
            ],
            None,
        );
    } else {
        // Active floor plate: the verbatim PlateGrid when present (one tinted quad per covered
        // cell — partial mezzanines and double-height voids render exactly as measured, and
        // landings read via the height ramp), else the traced-polygon fill for pre-plate assets.
        match lvl.plate.as_ref().filter(|g| g.is_valid()) {
            Some(plate) => {
                let base = lvl.elevation_range[0];
                let cell = plate.cell_size_m;
                for cx in 0..plate.nx {
                    for cz in 0..plate.nz {
                        let Some(h) = plate.heights_m[cx * plate.nz + cz] else {
                            continue;
                        };
                        // ±0.4 m ramp around the level base between the two floor shades.
                        let t = (((h - base) / 0.8 + 0.5).clamp(0.0, 1.0)) as f32;
                        let col = [
                            COL_PLATE_LO[0] + (COL_PLATE_HI[0] - COL_PLATE_LO[0]) * t,
                            COL_PLATE_LO[1] + (COL_PLATE_HI[1] - COL_PLATE_LO[1]) * t,
                            COL_PLATE_LO[2] + (COL_PLATE_HI[2] - COL_PLATE_LO[2]) * t,
                            COL_PLATE_LO[3] + (COL_PLATE_HI[3] - COL_PLATE_LO[3]) * t,
                        ];
                        let center = [
                            plate.origin[0] + (cx as f64 + 0.5) * cell,
                            plate.origin[1] + (cz as f64 + 0.5) * cell,
                        ];
                        append_polygon(
                            &mut out.floor_pos,
                            &mut out.floor_col,
                            &mut out.floor_idx,
                            &rect_corners(center, [cell, cell], 0.0),
                            col,
                        );
                        out.plate_cell_count += 1;
                    }
                }
            }
            None => {
                append_polygon(
                    &mut out.floor_pos,
                    &mut out.floor_col,
                    &mut out.floor_idx,
                    &lvl.footprint_polygon,
                    COL_FLOOR,
                );
            }
        }
        // Traced plate boundary rings (outer + holes) as hairline loops over the grid — the
        // derived polygon contract drawn against the verbatim cells, so ring-vs-grid
        // coincidence is directly eyeballable.
        for piece in &lvl.floor_polygons {
            for ring in std::iter::once(&piece.outer).chain(piece.holes.iter()) {
                let n = ring.len();
                for i in 0..n {
                    seg(
                        &mut out.hairlines,
                        to_world(ring[i]),
                        to_world(ring[(i + 1) % n]),
                        COL_PLATE_EDGE,
                    );
                    out.hairline_count += 1;
                }
            }
        }
        for st in &lvl.stairs {
            let ring = [
                [st.bounds[0][0], st.bounds[0][1]],
                [st.bounds[1][0], st.bounds[0][1]],
                [st.bounds[1][0], st.bounds[1][1]],
                [st.bounds[0][0], st.bounds[1][1]],
            ];
            append_polygon(
                &mut out.floor_pos,
                &mut out.floor_col,
                &mut out.floor_idx,
                &ring,
                COL_STAIRS,
            );
            // Tread hatch: lines across the short axis.
            let (w, d) = (
                st.bounds[1][0] - st.bounds[0][0],
                st.bounds[1][1] - st.bounds[0][1],
            );
            let n = st.step_count.clamp(4, 24);
            for i in 1..n {
                let f = f64::from(i) / f64::from(n);
                let (a, b) = if w >= d {
                    let x = st.bounds[0][0] + w * f;
                    ([x, st.bounds[0][1]], [x, st.bounds[1][1]])
                } else {
                    let z = st.bounds[0][1] + d * f;
                    ([st.bounds[0][0], z], [st.bounds[1][0], z])
                };
                seg(&mut out.stairs, to_world(a), to_world(b), COL_HATCH);
                out.stairs_count += 1;
            }
        }
    }

    match mesh_level {
        // The mesh's section at eye height IS the wall drawing: true double-line outlines,
        // window gaps, mullions, columns, collision furniture — one strip per segment for
        // weight when zoomed in (ROADS_CASING) plus a constant 1 px hairline
        // (FOREST_OUTLINE) so the outline never thins out at low zoom. The low cut draws
        // the wall continuous under the window gaps (sills), dim.
        Some(l) => {
            for s in &l.cut_main {
                let pts = [to_world(s[0]), to_world(s[1])];
                let verts = expand_polyline_strip(&pts, CUT_STRIP_M, COL_WALL_EXT);
                push_strip(&mut out.walls, &verts);
                out.wall_count += 1;
                seg(&mut out.cuts, pts[0], pts[1], COL_CUT);
                out.cut_count += 1;
            }
            for s in &l.cut_low {
                seg(
                    &mut out.hairlines,
                    to_world(s[0]),
                    to_world(s[1]),
                    COL_CUT_LOW,
                );
                out.hairline_count += 1;
            }
        }
        // Fallback: the blueprint's walls at nominal thickness.
        None => {
            for wall in &lvl.walls {
                let col = if wall.is_exterior {
                    COL_WALL_EXT
                } else {
                    COL_WALL_INT
                };
                let pts = [to_world(wall.start), to_world(wall.end)];
                let verts = expand_polyline_strip(&pts, wall.thickness.max(0.06), col);
                push_strip(&mut out.walls, &verts);
                out.wall_count += 1;
            }
        }
    }

    // Aperture overlays along the wall direction, slightly wider than the wall.
    let wall_of = |id: &str| lvl.walls.iter().find(|w| w.id == id);
    for win in &lvl.windows {
        let dir = wall_of(&win.wall_id)
            .map(|w| {
                let d = [w.end[0] - w.start[0], w.end[1] - w.start[1]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
                [d[0] / l, d[1] / l]
            })
            .unwrap_or([1.0, 0.0]);
        let th = wall_of(&win.wall_id).map_or(0.3, |w| w.thickness) + 0.10;
        let a = [
            win.pos2_d[0] - dir[0] * win.width_m * 0.5,
            win.pos2_d[1] - dir[1] * win.width_m * 0.5,
        ];
        let b = [
            win.pos2_d[0] + dir[0] * win.width_m * 0.5,
            win.pos2_d[1] + dir[1] * win.width_m * 0.5,
        ];
        let verts = expand_polyline_strip(&[to_world(a), to_world(b)], th, COL_WINDOW);
        push_strip(&mut out.apertures, &verts);
        out.aperture_count += 1;
        // Facing normal tick.
        let n = [win.normal[0], win.normal[1]];
        let tip = [win.pos2_d[0] + n[0] * 0.9, win.pos2_d[1] + n[1] * 0.9];
        seg(
            &mut out.hairlines,
            to_world(win.pos2_d),
            to_world(tip),
            COL_NORMAL,
        );
        out.hairline_count += 1;
    }
    for door in &lvl.doors {
        let wall = wall_of(&door.wall_id);
        let dir = wall
            .map(|w| {
                let d = [w.end[0] - w.start[0], w.end[1] - w.start[1]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
                [d[0] / l, d[1] / l]
            })
            .unwrap_or([1.0, 0.0]);
        let th = wall.map_or(0.3, |w| w.thickness) + 0.10;
        let col = if door.default_state == "open" {
            COL_DOOR_OPEN
        } else {
            COL_DOOR_CLOSED
        };
        let a = [
            door.pos2_d[0] - dir[0] * door.width_m * 0.5,
            door.pos2_d[1] - dir[1] * door.width_m * 0.5,
        ];
        let b = [
            door.pos2_d[0] + dir[0] * door.width_m * 0.5,
            door.pos2_d[1] + dir[1] * door.width_m * 0.5,
        ];
        let verts = expand_polyline_strip(&[to_world(a), to_world(b)], th, col);
        push_strip(&mut out.apertures, &verts);
        out.aperture_count += 1;
        swing_arc(
            &mut out.arcs,
            &mut out.arc_count,
            lvl,
            door.pos2_d,
            door.width_m,
            &door.hinge_side,
            &door.swing_direction,
            dir,
        );
    }

    // Furniture plates.
    for f in &lvl.furniture {
        let col = match f.los_cover.as_str() {
            "full_cover" => COL_FURN_FULL,
            "low_cover" => COL_FURN_LOW,
            _ => COL_FURN_NONE,
        };
        let ring: Vec<[f64; 2]> = rect_corners(f.pos2_d, f.size2_d, f.rotation_deg).to_vec();
        append_polygon(
            &mut out.furn_pos,
            &mut out.furn_col,
            &mut out.furn_idx,
            &ring,
            col,
        );
    }

    match (drawing, mesh_level) {
        // Lower floors show ONLY through this floor's voids (stairwells, double-height
        // spaces): their eye-height cuts, clipped by this level's floor-coverage raster,
        // dimmer the deeper they are.
        (Some(d), Some(l)) => {
            for (j, below) in d.levels.iter().enumerate().take(active) {
                let col = if j + 1 == active {
                    COL_GHOST
                } else {
                    COL_GHOST_DEEP
                };
                for s in through_voids(&below.cut_main, &l.surface, l.floor_min_y(), PLAN_CELL_M) {
                    seg(&mut out.hairlines, to_world(s[0]), to_world(s[1]), col);
                    out.hairline_count += 1;
                }
            }
        }
        // Fallback: every other floor's wall centerlines as ghosts.
        _ => {
            for (i, ghost) in bp.levels.iter().enumerate() {
                if i == active {
                    continue;
                }
                for wall in &ghost.walls {
                    seg(
                        &mut out.hairlines,
                        to_world(wall.start),
                        to_world(wall.end),
                        COL_GHOST,
                    );
                    out.hairline_count += 1;
                }
            }
        }
    }

    out
}
