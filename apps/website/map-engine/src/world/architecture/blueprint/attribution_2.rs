//! Role: attribution 2.
//! Position: `world/architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::architecture::blueprint::attribution_1::APERTURE_SLACK_M;
use crate::world::architecture::blueprint::attribution_1::LosHit;
use crate::world::architecture::blueprint::attribution_1::LosHitKind;
use crate::world::architecture::blueprint::geometry::dist_2d;
use crate::world::architecture::blueprint::geometry::point_at;
use crate::world::architecture::blueprint::geometry::segment_aabb_entry_t_2d;
use crate::world::architecture::blueprint::geometry::segment_intersection_t_2d;
use crate::world::architecture::blueprint::structure::BuildingLevel;

/// Collect level annotations.
pub(crate) fn collect_level_annotations(
    lvl: &BuildingLevel,
    obs: [f64; 3],
    tgt: [f64; 3],
    t0: f64,
    t1: f64,
    events: &mut Vec<LosHit>,
) {
    let p0 = point_at(obs, tgt, t0);
    let p1 = point_at(obs, tgt, t1);
    let sub0 = [p0[0], p0[2]];
    let sub1 = [p1[0], p1[2]];
    let sub_span = t1 - t0;
    if sub_span <= 0.0 {
        return;
    }

    for wall in &lvl.walls {
        let Some((t_sub, hit_pt)) = segment_intersection_t_2d(sub0, sub1, wall.start, wall.end)
        else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        let ray_y = obs[1] + t * (tgt[1] - obs[1]);
        let pos = point_at(obs, tgt, t);

        let hit_window = lvl.windows.iter().find(|w| {
            w.wall_id == wall.id && dist_2d(hit_pt, w.pos2_d) <= w.width_m * 0.5 + APERTURE_SLACK_M
        });
        if let Some(win) = hit_window {
            let win_bottom = lvl.elevation_range[0] + win.sill_height_m;
            let win_top = win_bottom + win.window_height_m;
            if ray_y >= win_bottom && ray_y <= win_top {
                events.push(LosHit {
                    t,
                    pos,
                    kind: LosHitKind::Window,
                    id: win.id.clone(),
                    concealment: 0.0,
                });
                continue;
            }
        }

        let hit_door = lvl.doors.iter().find(|d| {
            d.default_state == "open"
                && dist_2d(hit_pt, d.pos2_d) <= d.width_m * 0.5 + APERTURE_SLACK_M
        });
        if let Some(door) = hit_door
            && ray_y <= lvl.elevation_range[0] + door.height_m
        {
            events.push(LosHit {
                t,
                pos,
                kind: LosHitKind::DoorOpen,
                id: door.id.clone(),
                concealment: 0.0,
            });
        }
    }

    for furn in &lvl.furniture {
        if furn.los_cover == "none" {
            continue;
        }
        let half_w = furn.size2_d[0] * 0.5;
        let half_d = furn.size2_d[1] * 0.5;
        let min_2d = [furn.pos2_d[0] - half_w, furn.pos2_d[1] - half_d];
        let max_2d = [furn.pos2_d[0] + half_w, furn.pos2_d[1] + half_d];

        let Some(t_sub) = segment_aabb_entry_t_2d(sub0, sub1, min_2d, max_2d) else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        let ray_y = obs[1] + t * (tgt[1] - obs[1]);
        let furn_top = lvl.elevation_range[0] + furn.height_m;
        if ray_y > furn_top {
            continue;
        }
        let concealment = if furn.los_cover == "full_cover" {
            1.0
        } else {
            0.60
        };
        events.push(LosHit {
            t,
            pos: point_at(obs, tgt, t),
            kind: LosHitKind::Furniture,
            id: furn.id.clone(),
            concealment,
        });
    }

    for stair in &lvl.stairs {
        if !stair.transparent_steps {
            continue;
        }
        let Some(t_sub) = segment_aabb_entry_t_2d(sub0, sub1, stair.bounds[0], stair.bounds[1])
        else {
            continue;
        };
        let t = t0 + t_sub * sub_span;
        events.push(LosHit {
            t,
            pos: point_at(obs, tgt, t),
            kind: LosHitKind::Stairs,
            id: stair.id.clone(),
            concealment: stair.los_concealment,
        });
    }
}
