//! The aperture, cover and stairwell events a sight line meets on one level.
//!
//! **Role:** gathers, within a level's band of the ray, the windows and doors it passes, the
//! furniture it crosses and the stairwells it crosses or stops on
//! (`collect_level_annotations`).
//! **Position:** called by [`crate::blueprint::sight_line`] once per level the ray enters.
//! **Signals & state:** none; appends to the caller's event list.
//! **Invariants:** an aperture counts as passed only within `APERTURE_SLACK_M` of its rectangle;
//! every event's `t` lies inside the level's band.

use crate::blueprint::sight_line::APERTURE_SLACK_M;
use crate::blueprint::sight_line::LosHit;
use crate::blueprint::sight_line::LosHitKind;
use crate::blueprint::structure::BuildingLevel;
use crate::building_ids::BuildingFeatureId;
use geometry_primitives::segment_geometry::dist_2d;
use geometry_primitives::segment_geometry::point_at;
use geometry_primitives::segment_geometry::segment_aabb_entry_t_2d;
use geometry_primitives::segment_geometry::segment_intersection_t_2d;

/// Appends to `events` what the ray meets on `lvl` within its `t` range `[t0, t1]`:
/// windows passed between sill and top, `open` doors passed below their height,
/// non-`none` furniture entered below its top, open-tread stairs crossed.
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
                    id: BuildingFeatureId::new(win.id.as_str()),
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
                id: BuildingFeatureId::new(door.id.as_str()),
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
            id: BuildingFeatureId::new(furn.id.as_str()),
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
            id: BuildingFeatureId::new(stair.id.as_str()),
            concealment: stair.los_concealment,
        });
    }
}
