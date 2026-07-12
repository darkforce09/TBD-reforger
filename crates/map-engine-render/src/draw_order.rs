//! Draw-order contract for the render engine — **pure** (no wgpu/web types) so the ordering
//! relations are natively unit-tested (`lane_order_pins`), while the wasm32-gated `engine`
//! module consumes it (T-151.11.1; audits P-01/X-01).

/// A batch's role — governs the fixed W1 draw order (basemap → hillshade → grid) via
/// [`lane_order`] and lets the editor find/replace a lane in place on LOD / opacity change.
/// `Stress`/`Calibration` are the T-151.0 spike batches (never mixed with the editor lanes).
#[derive(Clone, Copy, PartialEq)]
pub enum LaneRole {
    Stress,
    Calibration,
    Satellite,
    /// W4 sea underlay (after basemap, before hillshade).
    Sea,
    Hillshade,
    /// W4 land-cover hulls.
    Landcover,
    Contours,
    RoadsCasing,
    Roads,
    /// W3 world-building OBB fills (`world-buildings`).
    WorldBuildings,
    /// W3 world-building outline casing (`world-buildings-outline`).
    WorldBuildingsOutline,
    ForestFill,
    ForestOutline,
    /// T-151.8 exact-count density heatmap (tree ladder over-budget rung).
    DensityHeat,
    /// W5 tree + vegetation glyphs.
    WorldTrees,
    /// W5 prop + rockLarge glyphs.
    WorldProps,
    /// W5 building badges.
    WorldBadges,
    /// T-152.1 cartographic text labels (after badges, before grid).
    WorldLabels,
    /// W6 mission slot rings.
    Slots,
    /// W6 drag-preview overlay (T-061).
    SlotDrag,
    /// W6 cluster discs (T-065).
    Clusters,
    Grid,
    /// Selection marquee fill (topmost with its outline).
    Marquee,
    /// Selection marquee 1 px border (T-151.11.1 — Deck `useSelectionLayer` LINE parity).
    MarqueeOutline,
}

/// Draw-order key (T-151.11.1 — Deck layer-array parity, `c4831451^:TacticalMap.tsx:382-395`):
/// … world glyph lanes → **grid** → slots → slot-drag → clusters → marquee (fill, then border).
/// Deck drew the grid above every world layer but **below** the mission lanes; T-151.6..T-151.10
/// had Grid above Slots/Clusters (grid lines overprinted unit markers — audit P-01).
/// Spike batches sort first, never interleaved. Relations pinned by `lane_order_pins` below.
pub fn lane_order(role: LaneRole) -> u8 {
    match role {
        LaneRole::Stress | LaneRole::Calibration => 0,
        LaneRole::Satellite => 1,
        LaneRole::Sea => 2,
        LaneRole::Hillshade => 3,
        LaneRole::Landcover => 4,
        LaneRole::Contours => 5,
        LaneRole::RoadsCasing => 6,
        LaneRole::Roads => 7,
        LaneRole::WorldBuildings => 8,
        LaneRole::WorldBuildingsOutline => 9,
        LaneRole::ForestFill => 10,
        LaneRole::ForestOutline => 11,
        LaneRole::DensityHeat => 12,
        LaneRole::WorldTrees => 13,
        LaneRole::WorldProps => 14,
        LaneRole::WorldBadges => 15,
        LaneRole::WorldLabels => 16,
        LaneRole::Grid => 17,
        LaneRole::Slots => 18,
        LaneRole::SlotDrag => 19,
        LaneRole::Clusters => 20,
        LaneRole::Marquee => 21,
        LaneRole::MarqueeOutline => 22,
    }
}

/// Map a public role u32 (upload API) → [`LaneRole`]. Returns `None` for unknown ids.
pub fn lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        0 => LaneRole::Sea,
        1 => LaneRole::Landcover,
        2 => LaneRole::Contours,
        3 => LaneRole::RoadsCasing,
        4 => LaneRole::Roads,
        5 => LaneRole::ForestFill,
        6 => LaneRole::ForestOutline,
        7 => LaneRole::Marquee,
        _ => return None,
    })
}

/// T-151.11.1 — lane-order pins (audit P-01/X-01). These relations ARE the layer contract;
/// any renumbering that breaks Deck parity fails here before it can ship.
#[cfg(test)]
mod lane_order_pins {
    use super::{LaneRole as L, lane_order};

    #[test]
    fn labels_sit_between_badges_and_grid() {
        assert!(lane_order(L::WorldLabels) > lane_order(L::WorldBadges));
        assert!(lane_order(L::WorldLabels) < lane_order(L::Grid));
    }

    #[test]
    fn grid_sits_between_world_glyphs_and_mission_lanes() {
        assert!(lane_order(L::Grid) > lane_order(L::WorldBadges));
        assert!(lane_order(L::Grid) > lane_order(L::WorldLabels));
        assert!(lane_order(L::Grid) > lane_order(L::WorldTrees));
        assert!(lane_order(L::Grid) > lane_order(L::WorldProps));
        assert!(lane_order(L::Grid) < lane_order(L::Slots));
        assert!(lane_order(L::Grid) < lane_order(L::SlotDrag));
        assert!(lane_order(L::Grid) < lane_order(L::Clusters));
    }

    #[test]
    fn marquee_lanes_are_topmost_fill_then_border() {
        let max_non_marquee = [
            L::Satellite,
            L::Sea,
            L::Hillshade,
            L::Landcover,
            L::Contours,
            L::RoadsCasing,
            L::Roads,
            L::WorldBuildings,
            L::WorldBuildingsOutline,
            L::ForestFill,
            L::ForestOutline,
            L::DensityHeat,
            L::WorldTrees,
            L::WorldProps,
            L::WorldBadges,
            L::WorldLabels,
            L::Grid,
            L::Slots,
            L::SlotDrag,
            L::Clusters,
        ]
        .into_iter()
        .map(lane_order)
        .max()
        .unwrap();
        assert!(lane_order(L::Marquee) > max_non_marquee);
        assert!(lane_order(L::MarqueeOutline) > lane_order(L::Marquee));
    }

    #[test]
    fn basemap_stack_order_is_deck_parity() {
        // satellite → sea → hillshade → landcover → contours → roads → buildings → forest.
        let chain = [
            L::Satellite,
            L::Sea,
            L::Hillshade,
            L::Landcover,
            L::Contours,
            L::RoadsCasing,
            L::Roads,
            L::WorldBuildings,
            L::WorldBuildingsOutline,
            L::ForestFill,
            L::ForestOutline,
            L::DensityHeat,
            L::WorldTrees,
            L::WorldProps,
            L::WorldBadges,
        ];
        for w in chain.windows(2) {
            assert!(
                lane_order(w[0]) < lane_order(w[1]),
                "order violated: {:?} !< {:?}",
                lane_order(w[0]),
                lane_order(w[1])
            );
        }
    }

    /// The compute-cull indirect emission keys on `> lane_order(WorldTrees)`; the first ordered
    /// role after trees must therefore be WorldProps — pin it so the emission point is stable.
    #[test]
    fn first_role_after_trees_is_props() {
        assert_eq!(lane_order(L::WorldProps), lane_order(L::WorldTrees) + 1);
    }
}
